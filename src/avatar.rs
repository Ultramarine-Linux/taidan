use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use slint::ComponentHandle;

use crate::ui::{on_ui, spawn_then};

use crate::ui::{Account, AppWindow, Avatar, AvatarRow, Camera};

#[cfg(not(debug_assertions))]
const AVATARS_DIR: &str = "/usr/share/taidan/avatars";
#[cfg(debug_assertions)]
const AVATARS_DIR: &str = "data/avatars";

const PHOTO_SIZE: u32 = 512;
const PREVIEW_SIZE: u32 = 320;

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

struct Running {
    stop: Arc<AtomicBool>,
    latest_frame: Arc<Mutex<Option<image::RgbImage>>>,
}

thread_local! {
    static RUNNING: std::cell::RefCell<Option<Running>> = const { std::cell::RefCell::new(None) };
}

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
    let preview_pending = Arc::new(AtomicBool::new(false));
    crate::camera::stream(
        || !stop.load(Ordering::Relaxed),
        |frame| {
            let preview =
                (!preview_pending.load(Ordering::Acquire)).then(|| mirrored_preview(&frame));
            *latest_frame.lock() = Some(frame);
            let Some(preview) = preview else { return };
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
        },
    )
}

fn mirrored_preview(frame: &image::RgbImage) -> Buffer {
    to_round_buffer(&image::imageops::flip_horizontal(&center_square_scaled(frame, PREVIEW_SIZE)))
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
    std::thread::spawn(move || {
        if let Err(e) = stream(&ui, &stop, &latest_frame) {
            show_error(&ui, stop, e.to_string());
        }
    });
}

fn refresh(ui: &slint::Weak<AppWindow>) {
    if RUNNING.with_borrow(Option::is_some) {
        return;
    }
    spawn_then(ui.clone(), crate::camera::available, |ui, available| {
        tracing::debug!(available, "Checked for a camera");
        ui.global::<Camera<'_>>().set_available(available);
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
            let available = crate::camera::available();
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
