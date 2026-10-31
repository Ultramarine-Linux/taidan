//! user icon: preset images, and taking a photo with the webcam
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

use crate::ui::{Account, AppWindow, Avatar, Camera};

#[cfg(not(debug_assertions))]
const AVATARS_DIR: &str = "/usr/share/taidan/avatars";
#[cfg(debug_assertions)]
const AVATARS_DIR: &str = "data/avatars";

/// side of the saved photo, in pixels
const PHOTO_SIZE: u32 = 512;
/// side of the preview frames sent to the UI, in pixels
const PREVIEW_SIZE: u32 = 320;

type Buffer = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

/// `img` (square) as a circle, transparent outside, done here rather than by clipping in Slint,
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

/// the largest centered square of `img`, scaled to `size` and mirrored, as people expect to
/// see themselves
fn square(img: &image::RgbImage, size: u32) -> image::RgbImage {
    let edge = img.width().min(img.height());
    let (x, y) = ((img.width() - edge) / 2, (img.height() - edge) / 2);
    let cropped = image::imageops::crop_imm(img, x, y, edge, edge).to_image();
    let scaled =
        image::imageops::resize(&cropped, size, size, image::imageops::FilterType::Triangle);
    image::imageops::flip_horizontal(&scaled)
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
fn decode(fourcc: v4l::FourCC, width: u32, height: u32, data: &[u8]) -> Option<image::RgbImage> {
    match &fourcc.repr {
        b"MJPG" => image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
            .inspect_err(|e| tracing::debug!(?e, "bad MJPEG frame"))
            .ok()
            .map(image::DynamicImage::into_rgb8),
        b"YUYV" => yuyv_to_rgb(width, height, data),
        _ => None,
    }
}

/// converts packed YUYV 4:2:2 (BT.601) to RGB
fn yuyv_to_rgb(width: u32, height: u32, data: &[u8]) -> Option<image::RgbImage> {
    let len = usize::try_from(width * height * 2).ok()?;
    let data = data.get(..len)?;
    let mut out = Vec::with_capacity(len / 2 * 3);
    for &[y0, u, y1, v] in data.as_chunks::<4>().0 {
        let (u, v) = (f32::from(u) - 128.0, f32::from(v) - 128.0);
        for y in [f32::from(y0), f32::from(y1)] {
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
            let c = |x: f32| x.clamp(0.0, 255.0) as u8;
            out.extend([
                c(1.402f32.mul_add(v, y)),
                c(0.714f32.mul_add(-v, 0.344f32.mul_add(-u, y))),
                c(1.772f32.mul_add(u, y)),
            ]);
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

/// a running camera, dropping the flag's last user stops it
struct Running {
    stop: Arc<AtomicBool>,
    /// latest full frame, for [`capture`]
    last: Arc<Mutex<Option<image::RgbImage>>>,
}

thread_local! {
    static RUNNING: std::cell::RefCell<Option<Running>> = const { std::cell::RefCell::new(None) };
}

fn set_error(ui: &slint::Weak<AppWindow>, error: String) {
    tracing::error!(error, "camera error");
    _ = ui.upgrade_in_event_loop(move |ui| ui.global::<Camera<'_>>().set_error(error.into()));
}

fn stream(
    ui: &slint::Weak<AppWindow>,
    stop: &AtomicBool,
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
    let mut stream = v4l::io::mmap::Stream::with_buffers(&dev, v4l::buffer::Type::VideoCapture, 4)?;
    while !stop.load(Ordering::Relaxed) {
        let (buf, meta) = stream.next()?;
        let data = buf.get(..usize::try_from(meta.bytesused).unwrap_or(buf.len())).unwrap_or(buf);
        let Some(frame) = decode(fmt.fourcc, fmt.width, fmt.height, data) else { continue };
        let preview = to_round_buffer(&square(&frame, PREVIEW_SIZE));
        if let Ok(mut last) = last.lock() {
            *last = Some(frame);
        }
        _ = ui.upgrade_in_event_loop(move |ui| {
            ui.global::<Camera<'_>>().set_frame(slint::Image::from_rgba8(preview));
        });
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
    RUNNING.set(Some(Running { stop: Arc::clone(&stop), last: Arc::clone(&last) }));
    let ui = ui.clone();
    std::thread::spawn(move || {
        if let Err(e) = stream(&ui, &stop, &last) {
            set_error(&ui, e.to_string());
        }
    });
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
    let photo = square(frame, PHOTO_SIZE);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // a new name each time, so the UI doesn't show a cached older photo
    let path = Path::new(&*crate::TEMP_DIR).join(format!("user-icon-{stamp}.png"));
    photo.save(&path)?;
    Ok((path, to_round_buffer(&photo)))
}

fn capture(ui: &slint::Weak<AppWindow>) {
    let frame = RUNNING.with_borrow(|r| r.as_ref().and_then(|r| r.last.lock().ok()?.clone()));
    let Some(frame) = frame else { return };
    match save_photo(&frame) {
        Ok((path, photo)) => {
            if let Some(app) = ui.upgrade() {
                let account = app.global::<Account<'_>>();
                account.set_avatar_id(path.to_string_lossy().as_ref().into());
                account.set_avatar(slint::Image::from_rgba8(photo));
            }
            stop(ui);
        }
        Err(e) => set_error(ui, format!("cannot save photo: {e}")),
    }
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
            ui.global::<Account<'_>>().set_avatars(slint::ModelRc::new(avatars));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuyv_grey_stays_grey() {
        // mid grey is Y=128, U=V=128
        let rgb = yuyv_to_rgb(2, 1, &[128, 128, 128, 128]).unwrap();
        assert_eq!(rgb.as_raw(), &[128, 128, 128, 128, 128, 128]);
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
