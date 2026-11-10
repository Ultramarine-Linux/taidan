//! user icon: preset images, and taking a photo with the webcam
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

use crate::ui::{Account, AppWindow, Avatar, AvatarRow, Camera};

#[cfg(not(debug_assertions))]
const AVATARS_DIR: &str = "/usr/share/taidan/avatars";
#[cfg(debug_assertions)]
const AVATARS_DIR: &str = "data/avatars";

/// side of the saved photo, in pixels
const PHOTO_SIZE: u32 = 512;
/// side of the preview frames sent to the UI, in pixels
const PREVIEW_SIZE: u32 = 320;
/// how long a camera may take to send its first frame
const FIRST_FRAME_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
/// how often a stream waiting for a frame checks whether it should stop, in milliseconds
const POLL_MS: i32 = 100;

type Buffer = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

/// `img` (square) as a circle, transparent outside. done here rather than by clipping in Slint,
/// since the software renderer can't clip to rounded shapes
fn to_round_buffer(img: &image::RgbImage) -> Buffer {
    #[expect(clippy::cast_precision_loss, reason = "image sizes are small")]
    let r = img.width().min(img.height()) as f32 / 2.0;
    let mut buf = Buffer::new(img.width(), img.height());
    for ((x, y, px), out) in img.enumerate_pixels().zip(buf.make_mut_slice()) {
        #[expect(clippy::cast_precision_loss, reason = "image sizes are small")]
        let dist = (x as f32 + 0.5 - r).hypot(y as f32 + 0.5 - r);
        // anti-aliased over one pixel at the edge
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
        let a = ((r - dist + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
        *out = slint::Rgba8Pixel::new(px[0], px[1], px[2], a);
    }
    buf
}

/// the largest centered square of `img`, scaled to `size`
fn square(img: &image::RgbImage, size: u32) -> image::RgbImage {
    let edge = img.width().min(img.height());
    let (x, y) = ((img.width() - edge) / 2, (img.height() - edge) / 2);
    let cropped = image::imageops::crop_imm(img, x, y, edge, edge).to_image();
    image::imageops::resize(&cropped, size, size, image::imageops::FilterType::Triangle)
}

/// preset icons: `avatars_dir` in `/etc/taidan.toml`, else [`AVATARS_DIR`]
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
            // saved with the absolute path, since libtaidan copies the file during the install
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            Some((path, to_round_buffer(&img.into_rgb8())))
        })
        .collect()
}

/// decodes one frame from the camera
fn decode(fmt: &v4l::Format, data: &[u8]) -> Option<image::RgbImage> {
    match &fmt.fourcc.repr {
        b"MJPG" => image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
            .inspect_err(|e| tracing::debug!(?e, "bad MJPEG frame"))
            .ok()
            .map(image::DynamicImage::into_rgb8),
        b"YUYV" => yuyv_to_rgb(fmt.width, fmt.height, fmt.stride, data),
        _ => None,
    }
}

/// converts packed YUYV 4:2:2 (BT.601) to RGB. each line starts `stride` bytes after the last,
/// which drivers may pad past `width * 2`
fn yuyv_to_rgb(width: u32, height: u32, stride: u32, data: &[u8]) -> Option<image::RgbImage> {
    let row_len = usize::try_from(width).ok()? * 2;
    // 0 means the driver didn't say, so the lines are packed
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

/// the first device that can capture video, as laptops often list metadata-only nodes as well
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

/// the running camera stream, which ends once `stop` is set
struct Running {
    stop: Arc<AtomicBool>,
    /// latest full frame, for [`capture`]
    last: Arc<Mutex<Option<image::RgbImage>>>,
}

thread_local! {
    static RUNNING: std::cell::RefCell<Option<Running>> = const { std::cell::RefCell::new(None) };
}

/// shows `error` unless its stream was stopped by then, since a newer one may be showing
fn set_error(ui: &slint::Weak<AppWindow>, stop: Arc<AtomicBool>, error: String) {
    tracing::error!(error, "camera error");
    _ = ui.upgrade_in_event_loop(move |ui| {
        if !stop.load(Ordering::Relaxed) {
            ui.global::<Camera<'_>>().set_error(error.into());
        }
    });
}

fn stream(
    ui: &slint::Weak<AppWindow>,
    stop: &Arc<AtomicBool>,
    last: &Mutex<Option<image::RgbImage>>,
) -> std::io::Result<()> {
    let dev = find_camera().ok_or_else(|| std::io::Error::other("no camera found"))?;
    let formats = dev.enum_formats()?;
    // prefer MJPEG, which gets higher resolutions over USB than raw YUYV does
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
    // only for the first frame, which starts the stream. the rest wait in `poll` below, since a
    // timed out `next()` leaves its buffer queued and can't be called again
    stream.set_timeout(FIRST_FRAME_TIMEOUT);
    let mut started = false;
    // set while the UI hasn't shown the last preview. newer frames get no preview until it has
    let in_flight = Arc::new(AtomicBool::new(false));
    while !stop.load(Ordering::Relaxed) {
        // waits for a frame in short steps, so a stop is noticed even if the camera stalls
        if started && handle.poll(libc::POLLIN, POLL_MS)? == 0 {
            continue;
        }
        let (buf, meta) = stream.next()?;
        started = true;
        let data = buf.get(..usize::try_from(meta.bytesused).unwrap_or(buf.len())).unwrap_or(buf);
        let Some(frame) = decode(&fmt, data) else { continue };
        let preview = (!in_flight.load(Ordering::Acquire))
            // mirrored, as people expect to see themselves. the saved photo isn't, so text in
            // it reads the right way
            .then(|| to_round_buffer(&image::imageops::flip_horizontal(&square(&frame, PREVIEW_SIZE))));
        if let Ok(mut last) = last.lock() {
            *last = Some(frame);
        }
        let Some(preview) = preview else { continue };
        in_flight.store(true, Ordering::Release);
        let done = Arc::clone(&in_flight);
        let stopped = Arc::clone(stop);
        let sent = ui.upgrade_in_event_loop(move |ui| {
            // otherwise a stopped stream's frame could show up in a newer stream before its own
            if !stopped.load(Ordering::Relaxed) {
                ui.global::<Camera<'_>>().set_frame(slint::Image::from_rgba8(preview));
            }
            done.store(false, Ordering::Release);
        });
        if sent.is_err() {
            in_flight.store(false, Ordering::Release);
        }
    }
    Ok(())
}

fn start(ui: &slint::Weak<AppWindow>) {
    let Some(app) = ui.upgrade() else { return };
    let camera = app.global::<Camera<'_>>();
    camera.set_error("".into());
    camera.set_frame(slint::Image::default());
    camera.set_open(true);
    let (stop, last) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(None)));
    let old = RUNNING.replace(Some(Running { stop: Arc::clone(&stop), last: Arc::clone(&last) }));
    if let Some(old) = old {
        old.stop.store(true, Ordering::Relaxed);
    }
    let ui = ui.clone();
    let previous = STREAM_THREAD.lock().ok().and_then(|mut t| t.take());
    let thread = std::thread::spawn(move || {
        // the previous stream releases the camera once it notices it's stopped, within `POLL_MS`,
        // or `FIRST_FRAME_TIMEOUT` while it waits for its first frame. opening it before then
        // fails as busy. the wait is bounded in case it's stuck
        if let Some(previous) = previous {
            let tries = FIRST_FRAME_TIMEOUT.as_millis() / 50 + 20;
            for _ in 0..tries {
                if previous.is_finished() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        if let Err(e) = stream(&ui, &stop, &last) {
            set_error(&ui, stop, e.to_string());
        }
    });
    if let Ok(mut t) = STREAM_THREAD.lock() {
        *t = Some(thread);
    }
}

/// checks for a camera again, since one may have been plugged in after the OOBE started
fn refresh(ui: &slint::Weak<AppWindow>) {
    // a running stream holds the camera, so it's there anyway
    if RUNNING.with_borrow(Option::is_some) {
        return;
    }
    let ui = ui.clone();
    std::thread::spawn(move || {
        let available = find_camera().is_some();
        tracing::debug!(available, "Checked for a camera");
        _ = ui.upgrade_in_event_loop(move |ui| ui.global::<Camera<'_>>().set_available(available));
    });
}

/// the thread of the last stream, see [`start`]
static STREAM_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

fn stop(ui: &slint::Weak<AppWindow>) {
    if let Some(running) = RUNNING.take() {
        running.stop.store(true, Ordering::Relaxed);
    }
    if let Some(app) = ui.upgrade() {
        app.global::<Camera<'_>>().set_open(false);
    }
}

fn save_photo(frame: &image::RgbImage) -> image::ImageResult<(PathBuf, Buffer)> {
    let photo = square(frame, PHOTO_SIZE);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // a new name each time, so the UI doesn't show a cached older photo
    let path = Path::new(&*crate::TEMP_DIR).join(format!("user-icon-{stamp}.png"));
    photo.save(&path)?;
    Ok((path, to_round_buffer(&photo)))
}

thread_local! {
    /// whether a photo is being saved, so pressing Take again doesn't save another
    static SAVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// saves the latest frame on a thread, since encoding the PNG would stall the UI
fn capture(ui: &slint::Weak<AppWindow>) {
    if SAVING.get() {
        return;
    }
    let running = RUNNING.with_borrow(|r| {
        r.as_ref().and_then(|r| Some((Arc::clone(&r.stop), r.last.lock().ok()?.clone()?)))
    });
    let Some((stopped, frame)) = running else { return };
    SAVING.set(true);
    let ui = ui.clone();
    std::thread::spawn(move || match save_photo(&frame) {
        Ok((path, photo)) => {
            _ = ui.upgrade_in_event_loop(move |app| {
                SAVING.set(false);
                // cancelled while it saved
                if stopped.load(Ordering::Relaxed) {
                    return;
                }
                let account = app.global::<Account<'_>>();
                account.set_avatar_id(path.to_string_lossy().as_ref().into());
                account.set_avatar(slint::Image::from_rgba8(photo));
                stop(&app.as_weak());
            });
        }
        Err(e) => {
            _ = ui.upgrade_in_event_loop(|_| SAVING.set(false));
            set_error(&ui, stopped, format!("cannot save photo: {e}"));
        }
    });
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, camera: &Camera<'_>) {
    camera.on_start({
        let ui = ui.clone();
        move || start(&ui)
    });
    camera.on_stop({
        let ui = ui.clone();
        move || stop(&ui)
    });
    camera.on_capture({
        let ui = ui.clone();
        move || capture(&ui)
    });
    camera.on_refresh({
        let ui = ui.clone();
        move || refresh(&ui)
    });

    let ui = ui.clone();
    std::thread::spawn(move || {
        let presets = load_presets();
        let available = find_camera().is_some();
        tracing::debug!(presets = presets.len(), available, "Loaded user icons");
        _ = ui.upgrade_in_event_loop(move |ui| {
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
        });
    });
}

/// rows of the icon picker for `presets` icons, as indices into them, with -1 for the camera. the
/// camera is in the middle row with a preset on each side. the rest go in rows above and below
/// it, two, then three, then two and so on, so they stagger like a honeycomb. rows mirror each
/// other across the middle, and the one above gets any odd one out
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
    // (preset count, whether it's the camera's row)
    let sizes = above.into_iter().rev().map(|n| (n, false)).chain([(middle, true)]);
    let sizes = sizes.chain(below.into_iter().filter(|&n| n > 0).map(|n| (n, false)));
    let mut next = 0..;
    sizes
        .map(|(n, camera)| {
            let mut row: Vec<i32> = next.by_ref().take(n).collect();
            if camera {
                row.insert(n / 2, -1);
            }
            row
        })
        .collect()
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
        // mid grey is Y=128, U=V=128
        let rgb = yuyv_to_rgb(2, 1, 0, &[128, 128, 128, 128]).unwrap();
        assert_eq!(rgb.as_raw(), &[128, 128, 128, 128, 128, 128]);
    }

    #[test]
    fn yuyv_skips_line_padding() {
        // two lines of one pixel pair, each padded to 8 bytes with white that mustn't show up
        let data = [16, 128, 16, 128, 255, 255, 255, 255, 235, 128, 235, 128, 255, 255, 255, 255];
        let rgb = yuyv_to_rgb(2, 2, 8, &data).unwrap();
        assert_eq!(rgb.as_raw(), &[16, 16, 16, 16, 16, 16, 235, 235, 235, 235, 235, 235]);
        // cut off before the last line
        assert!(yuyv_to_rgb(2, 2, 8, &data[..9]).is_none());
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
        let sq = square(&img, 2);
        assert_eq!((sq.width(), sq.height()), (2, 2));
    }
}
