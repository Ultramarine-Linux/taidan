use std::process::{Command, Stdio};

use crate::child::ChildSlot;

const WALLPAPER: &str = "/usr/share/taidan/wallpaper.png";
const FALLBACK_COLOR: &str = "#000724";

static SWAYBG: ChildSlot = ChildSlot::new("swaybg");

fn show() {
    tracing::debug!(image = WALLPAPER, "Showing the wallpaper");
    let child = Command::new("swaybg")
        .args(["-i", WALLPAPER, "-m", "fill", "-c", FALLBACK_COLOR])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .inspect_err(|e| libtaidan::command::report_spawn_error("swaybg", e))
        .inspect_err(|e| tracing::error!(?e, "cannot start swaybg"))
        .ok();
    if let Some(previous) = SWAYBG.replace((), child) {
        crate::child::terminate("swaybg", previous);
    }
}

pub fn shutdown() {
    SWAYBG.stop();
}

pub fn initialize() {
    if crate::install::is_dev_session() {
        return;
    }
    show();
}
