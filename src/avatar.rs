use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use slint::ComponentHandle;

use crate::ui::{on_ui, spawn_then};
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

use crate::ui::{Account, AppWindow, Avatar, AvatarRow, Camera};

#[cfg(not(debug_assertions))]
const AVATARS_DIR: &str = "/usr/share/taidan/avatars";
#[cfg(debug_assertions)]
const AVATARS_DIR: &str = "data/avatars";

const PHOTO_SIZE: u32 = 512;
const PREVIEW_SIZE: u32 = 320;
const FIRST_FRAME_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const STOP_CHECK_INTERVAL_MS: i32 = 100;

type Buffer = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

// Slint's software renderer can't clip to rounded shapes
fn to_round_buffer(img: &image::RgbImage) -> Buffer {
    #[expect(clippy::cast_precision_loss, reason = "image sizes are small")]
    let r = img.width().min(img.height()) as f32 / 2.0;
    let mut buf = Buffer::new(img.width(), img.height());
    for ((x, y, px), out) in img.enumerate_pixels().zip(buf.make_mut_slice()) {
        #[expect(clippy::cast_precision_loss, reason = "image sizes are small")]
        let dist = (x as f32 + 0.5 - r).hypot(y as f32 + 0.5 - r);
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
        let alpha = ((r - dist + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
        *out = slint::Rgba8Pixel::new(px[0], px[1], px[2], alpha);
    }
    buf
}

fn center_square_scaled(img: &image::RgbImage, size: u32) -> image::RgbImage {
    let edge = img.width().min(img.height());
    let (x, y) = ((img.width() - edge) / 2, (img.height() - edge) / 2);
    let cropped = image::imageops::crop_imm(img, x, y, edge, edge).to_image();
    image::imageops::resize(&cropped, size, size, image::imageops::FilterType::Triangle)
}

fn load_presets() -> Vec<(PathBuf, Buffer)> {
    let dir = match crate::cfg::get_val("avatars_dir") {
        Some(serde_json::Value::String(dir)) => PathBuf::from(dir),
        _ => PathBuf::from(AVATARS_DIR),
    };
    let Ok(entries) = std::fs::read_dir(&dir)
        .inspect_err(|e| tracing::warn!(?dir, ?e, "cannot read user icons directory"))
    else {
        return vec![];
    };
    let mut paths = entries.filter_map(|e| e.ok().map(|e| e.path())).collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let img = image::open(&path)
                .inspect_err(|e| tracing::warn!(?path, ?e, "cannot load user icon"))
                .ok()?;
            let img = img.resize_to_fill(256, 256, image::imageops::FilterType::Triangle);
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            Some((path, to_round_buffer(&img.into_rgb8())))
        })
        .collect()
}

fn decode_frame(fmt: &v4l::Format, data: &[u8]) -> Option<image::RgbImage> {
    match &fmt.fourcc.repr {
        b"MJPG" => image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
            .inspect_err(|e| tracing::debug!(?e, "bad MJPEG frame"))
            .ok()
            .map(image::DynamicImage::into_rgb8),
        b"YUYV" => bt601_yuyv_to_rgb(fmt.width, fmt.height, fmt.stride, data),
        _ => None,
    }
}

fn bt601_yuyv_to_rgb(width: u32, height: u32, stride: u32, data: &[u8]) -> Option<image::RgbImage> {
    let row_len = usize::try_from(width).ok()? * 2;
    let stride = usize::try_from(stride).ok()?.max(row_len);
    let rows = usize::try_from(height).ok()?;
    let mut out = Vec::with_capacity(row_len / 2 * 3 * rows);
    for row in 0..rows {
        let start = row * stride;
        let line = data.get(start..start + row_len)?;
        for &[y0, u, y1, v] in line.as_chunks::<4>().0 {
            let (u, v) = (f32::from(u) - 128.0, f32::from(v) - 128.0);
            for y in [f32::from(y0), f32::from(y1)] {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped"
                )]
                let c = |x: f32| x.clamp(0.0, 255.0) as u8;
                out.extend([
                    c(1.402f32.mul_add(v, y)),
                    c(0.714f32.mul_add(-v, 0.344f32.mul_add(-u, y))),
                    c(1.772f32.mul_add(u, y)),
                ]);
            }
        }
    }
    image::RgbImage::from_raw(width, height, out)
}

fn find_camera() -> Option<v4l::Device> {
    v4l::context::enum_devices().into_iter().find_map(|node| {
        let dev = v4l::Device::with_path(node.path()).ok()?;
        let caps = dev.query_caps().ok()?;
        let formats = dev.enum_formats().ok()?;
        (caps.capabilities.contains(v4l::capability::Flags::VIDEO_CAPTURE)
            && formats.iter().any(|f| matches!(&f.fourcc.repr, b"MJPG" | b"YUYV")))
        .then_some(dev)
    })
}

struct Running {
    stop: Arc<AtomicBool>,
    latest_frame: Arc<Mutex<Option<image::RgbImage>>>,
}

thread_local! {
    static RUNNING: std::cell::RefCell<Option<Running>> = const { std::cell::RefCell::new(None) };
}

static STREAM_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

fn show_error(ui: &slint::Weak<AppWindow>, stop: Arc<AtomicBool>, error: String) {
    tracing::error!(error, "camera error");
    on_ui(ui, move |ui| {
        if !stop.load(Ordering::Relaxed) {
            ui.global::<Camera<'_>>().set_error(error.into());
        }
    });
}

fn stream(
    ui: &slint::Weak<AppWindow>,
    stop: &Arc<AtomicBool>,
    latest_frame: &Mutex<Option<image::RgbImage>>,
) -> std::io::Result<()> {
    let dev = find_camera().ok_or_else(|| std::io::Error::other("no camera found"))?;
    let formats = dev.enum_formats()?;
    let fourcc = [b"MJPG", b"YUYV"]
        .into_iter()
        .map(v4l::FourCC::new)
        .find(|cc| formats.iter().any(|f| f.fourcc == *cc))
        .ok_or_else(|| std::io::Error::other("unsupported camera format"))?;
    let mut fmt = dev.format()?;
    (fmt.width, fmt.height, fmt.fourcc) = (640, 480, fourcc);
    let fmt = dev.set_format(&fmt)?;
    tracing::debug!(?fmt, "Camera started");
    let handle = dev.handle();
    let mut stream = v4l::io::mmap::Stream::with_buffers(&dev, v4l::buffer::Type::VideoCapture, 4)?;
    // a timed out `next()` leaves its buffer queued and can't be called again, so only the first
    // frame uses it and the rest wait in `poll`
    stream.set_timeout(FIRST_FRAME_TIMEOUT);
    let mut started = false;
    let preview_pending = Arc::new(AtomicBool::new(false));
    while !stop.load(Ordering::Relaxed) {
        if started && handle.poll(libc::POLLIN, STOP_CHECK_INTERVAL_MS)? == 0 {
            continue;
        }
        let (buf, meta) = stream.next()?;
        started = true;
        let data = buf.get(..usize::try_from(meta.bytesused).unwrap_or(buf.len())).unwrap_or(buf);
        let Some(frame) = decode_frame(&fmt, data) else { continue };
        let preview = (!preview_pending.load(Ordering::Acquire)).then(|| mirrored_preview(&frame));
        *latest_frame.lock() = Some(frame);
        let Some(preview) = preview else { continue };
        preview_pending.store(true, Ordering::Release);
        let done = Arc::clone(&preview_pending);
        let stopped = Arc::clone(stop);
        let sent = ui.upgrade_in_event_loop(move |ui| {
            if !stopped.load(Ordering::Relaxed) {
                ui.global::<Camera<'_>>().set_frame(slint::Image::from_rgba8(preview));
            }
            done.store(false, Ordering::Release);
        });
        if sent.is_err() {
            preview_pending.store(false, Ordering::Release);
        }
    }
    Ok(())
}

fn mirrored_preview(frame: &image::RgbImage) -> Buffer {
    to_round_buffer(&image::imageops::flip_horizontal(&center_square_scaled(frame, PREVIEW_SIZE)))
}

fn wait_for_release(previous: &std::thread::JoinHandle<()>) {
    let deadline =
        std::time::Instant::now() + FIRST_FRAME_TIMEOUT + std::time::Duration::from_secs(1);
    while !previous.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn start(ui: &slint::Weak<AppWindow>) {
    let Some(app) = ui.upgrade() else { return };
    let camera = app.global::<Camera<'_>>();
    camera.set_error("".into());
    camera.set_frame(slint::Image::default());
    camera.set_open(true);
    let (stop, latest_frame) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(None)));
    let old = RUNNING.replace(Some(Running {
        stop: Arc::clone(&stop),
        latest_frame: Arc::clone(&latest_frame),
    }));
    if let Some(old) = old {
        old.stop.store(true, Ordering::Relaxed);
    }
    let ui = ui.clone();
    let mut stream_thread = STREAM_THREAD.lock();
    let previous = stream_thread.take();
    *stream_thread = Some(std::thread::spawn(move || {
        if let Some(previous) = previous {
            wait_for_release(&previous);
        }
        if let Err(e) = stream(&ui, &stop, &latest_frame) {
            show_error(&ui, stop, e.to_string());
        }
    }));
}

fn refresh(ui: &slint::Weak<AppWindow>) {
    if RUNNING.with_borrow(Option::is_some) {
        return;
    }
    spawn_then(
        ui.clone(),
        || find_camera().is_some(),
        |ui, available| {
            tracing::debug!(available, "Checked for a camera");
            ui.global::<Camera<'_>>().set_available(available);
        },
    );
}

fn stop(ui: &slint::Weak<AppWindow>) {
    if let Some(running) = RUNNING.take() {
        running.stop.store(true, Ordering::Relaxed);
    }
    if let Some(app) = ui.upgrade() {
        app.global::<Camera<'_>>().set_open(false);
    }
}

fn save_photo(frame: &image::RgbImage) -> image::ImageResult<(PathBuf, Buffer)> {
    let photo = center_square_scaled(frame, PHOTO_SIZE);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let path = Path::new(&*crate::TEMP_DIR).join(format!("user-icon-{stamp}.png"));
    photo.save(&path)?;
    Ok((path, to_round_buffer(&photo)))
}

thread_local! {
    static SAVING_PHOTO: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn capture(ui: &slint::Weak<AppWindow>) {
    if SAVING_PHOTO.get() {
        return;
    }
    let running = RUNNING.with_borrow(|r| {
        r.as_ref().and_then(|r| Some((Arc::clone(&r.stop), r.latest_frame.lock().clone()?)))
    });
    let Some((stopped, frame)) = running else { return };
    SAVING_PHOTO.set(true);
    spawn_then(
        ui.clone(),
        move || save_photo(&frame),
        |app, saved| {
            SAVING_PHOTO.set(false);
            match saved {
                Ok(_) if stopped.load(Ordering::Relaxed) => {}
                Ok((path, photo)) => {
                    let account = app.global::<Account<'_>>();
                    account.set_avatar_id(path.to_string_lossy().as_ref().into());
                    account.set_avatar(slint::Image::from_rgba8(photo));
                    stop(&app.as_weak());
                }
                Err(e) => show_error(&app.as_weak(), stopped, format!("cannot save photo: {e}")),
            }
        },
    );
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, camera: &Camera<'_>) {
    let with_ui = |f: fn(&slint::Weak<AppWindow>)| {
        let ui = ui.clone();
        move || f(&ui)
    };
    camera.on_start(with_ui(start));
    camera.on_stop(with_ui(stop));
    camera.on_capture(with_ui(capture));
    camera.on_refresh(with_ui(refresh));

    spawn_then(
        ui.clone(),
        || {
            let presets = load_presets();
            let available = find_camera().is_some();
            tracing::debug!(presets = presets.len(), available, "Loaded user icons");
            (presets, available)
        },
        |ui, (presets, available)| {
            ui.global::<Camera<'_>>().set_available(available);
            let avatars = presets
                .into_iter()
                .map(|(path, buf)| Avatar {
                    id: path.to_string_lossy().as_ref().into(),
                    image: slint::Image::from_rgba8(buf),
                })
                .collect::<slint::VecModel<_>>();
            let rows = picker_rows(slint::Model::row_count(&avatars))
                .into_iter()
                .map(|slots| AvatarRow { slots: slint::ModelRc::new(slint::VecModel::from(slots)) })
                .collect::<slint::VecModel<_>>();
            let account = ui.global::<Account<'_>>();
            account.set_avatars(slint::ModelRc::new(avatars));
            account.set_avatar_rows(slint::ModelRc::new(rows));
        },
    );
}

const CAMERA_SLOT: i32 = -1;

fn picker_rows(presets: usize) -> Vec<Vec<i32>> {
    let middle = if presets >= 2 { 2 } else { 0 };
    let (mut above, mut below) = (vec![], vec![]);
    let mut left = presets - middle;
    for width in [2, 3].into_iter().cycle() {
        if left == 0 {
            break;
        }
        let taken = left.min(2 * width);
        above.push(taken.div_ceil(2));
        below.push(taken / 2);
        left -= taken;
    }
    let camera_row = above.len();
    let sizes = above.into_iter().rev().chain([middle]).chain(below.into_iter().filter(|&n| n > 0));
    let mut next = 0..;
    let mut rows: Vec<Vec<i32>> = sizes.map(|n| next.by_ref().take(n).collect()).collect();
    rows[camera_row].insert(middle / 2, CAMERA_SLOT);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_rows_around_the_camera() {
        assert_eq!(picker_rows(0), [vec![-1]]);
        assert_eq!(picker_rows(1), [vec![0], vec![-1]]);
        assert_eq!(picker_rows(2), [vec![0, -1, 1]]);
        assert_eq!(picker_rows(6), [vec![0, 1], vec![2, -1, 3], vec![4, 5]]);
        assert_eq!(picker_rows(5), [vec![0, 1], vec![2, -1, 3], vec![4]]);
        assert_eq!(
            picker_rows(12),
            [vec![0, 1, 2], vec![3, 4], vec![5, -1, 6], vec![7, 8], vec![9, 10, 11]]
        );
    }

    #[test]
    fn yuyv_grey_stays_grey() {
        let mid_grey_pair = [128, 128, 128, 128];
        let rgb = bt601_yuyv_to_rgb(2, 1, 0, &mid_grey_pair).unwrap();
        assert_eq!(rgb.as_raw(), &[128, 128, 128, 128, 128, 128]);
    }

    #[test]
    fn yuyv_skips_line_padding() {
        let two_lines_padded_with_white =
            [16, 128, 16, 128, 255, 255, 255, 255, 235, 128, 235, 128, 255, 255, 255, 255];
        let rgb = bt601_yuyv_to_rgb(2, 2, 8, &two_lines_padded_with_white).unwrap();
        assert_eq!(rgb.as_raw(), &[16, 16, 16, 16, 16, 16, 235, 235, 235, 235, 235, 235]);
        let cut_off_in_last_line = &two_lines_padded_with_white[..9];
        assert!(bt601_yuyv_to_rgb(2, 2, 8, cut_off_in_last_line).is_none());
    }

    #[test]
    fn round_buffer_is_transparent_in_corners() {
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([255, 0, 0]));
        let buf = to_round_buffer(&img);
        assert_eq!(buf.as_slice()[0].a, 0);
        assert_eq!(buf.as_slice()[4 * 8 + 4].a, 255);
    }

    #[test]
    fn square_crops_center() {
        let img = image::RgbImage::from_fn(4, 2, |x, _| image::Rgb([u8::try_from(x).unwrap(); 3]));
        let sq = center_square_scaled(&img, 2);
        assert_eq!((sq.width(), sq.height()), (2, 2));
    }
}
