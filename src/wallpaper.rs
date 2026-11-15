use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::child::ChildSlot;
use crate::ui::AppWindow;

pub const LIGHT: &str = "/usr/share/backgrounds/default.png";
pub const DARK: &str = "/usr/share/backgrounds/default-dark.png";
const FALLBACK_COLOR: &str = "#113344";
const OLD_WALLPAPER_LINGER: Duration = Duration::from_millis(500);

static SWAYBG: ChildSlot = ChildSlot::new("swaybg");

fn spawn_swaybg(dark: bool) -> Option<Child> {
    let image = if dark && Path::new(DARK).exists() { DARK } else { LIGHT };
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

fn show(dark: bool) {
    if let Some(previous) = SWAYBG.replace((), spawn_swaybg(dark)) {
        std::thread::spawn(move || {
            std::thread::sleep(OLD_WALLPAPER_LINGER);
            crate::child::terminate("swaybg", previous);
        });
    }
}

pub fn shutdown() {
    SWAYBG.stop();
}

pub fn initialize_ui(ui: &AppWindow) {
    if crate::install::is_dev_session() {
        return;
    }
    show(ui.get_dark_mode());
    ui.on_dark_mode_changed(show);
}
