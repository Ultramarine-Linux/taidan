use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::ui::AppWindow;

const LIGHT: &str = "/usr/share/backgrounds/default.png";
const DARK: &str = "/usr/share/backgrounds/default-dark.png";
const FALLBACK_COLOR: &str = "#113344";
const OLD_WALLPAPER_LINGER: Duration = Duration::from_millis(500);

static SWAYBG: Mutex<Option<Child>> = Mutex::new(None);

fn lock_swaybg() -> MutexGuard<'static, Option<Child>> {
    SWAYBG.lock().unwrap_or_else(PoisonError::into_inner)
}

fn image_for(dark: bool) -> &'static str {
    if dark && Path::new(DARK).exists() { DARK } else { LIGHT }
}

fn spawn_swaybg(dark: bool) -> Option<Child> {
    let image = image_for(dark);
    tracing::debug!(image, "Showing the wallpaper");
    Command::new("swaybg")
        .args(["-i", image, "-m", "fill", "-c", FALLBACK_COLOR])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .inspect_err(|e| tracing::error!(?e, "cannot start swaybg"))
        .ok()
}

fn stop_swaybg(mut swaybg: Child) {
    if let Err(e) = swaybg.kill() {
        tracing::warn!(?e, "cannot stop swaybg");
    }
    if let Err(e) = swaybg.wait() {
        tracing::warn!(?e, "cannot wait for swaybg");
    }
}

fn show(dark: bool) {
    let previous = std::mem::replace(&mut *lock_swaybg(), spawn_swaybg(dark));
    if let Some(previous) = previous {
        std::thread::spawn(move || {
            std::thread::sleep(OLD_WALLPAPER_LINGER);
            stop_swaybg(previous);
        });
    }
}

pub fn shutdown() {
    let swaybg = lock_swaybg().take();
    if let Some(swaybg) = swaybg {
        stop_swaybg(swaybg);
    }
}

pub fn initialize_ui(ui: &AppWindow) {
    if crate::install::is_dev_session() {
        return;
    }
    show(ui.get_dark_mode());
    ui.on_dark_mode_changed(show);
}
