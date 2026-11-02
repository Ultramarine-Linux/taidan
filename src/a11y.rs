//! the accessibility page: applies the options to the OOBE's own KWin session as they change, and
//! keeps them in the settings so they're carried over to the new user
use std::sync::mpsc;

use libtaidan::a11y::A11y as Opts;
use slint::ComponentHandle;
use zbus::blocking::Connection;

use crate::ui::{A11y, AppWindow};

/// about [`libtaidan::a11y::ZOOM`]: zoom in multiplies by KWin's default zoom factor, 1.2
const ZOOM_STEPS: usize = 4;

/// the screen reader, here rather than in the worker so [`shutdown`] can stop it
static ORCA: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

/// stops Orca, which would outlive taidan otherwise
pub fn shutdown() {
    set_orca(false);
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, a11y: &A11y<'_>) {
    let (tx, rx) = mpsc::channel();
    // one thread, so changes are applied in order
    std::thread::spawn(move || worker(&rx));
    // resets anything an earlier run left in the session's config
    if let Err(e) = tx.send(Opts::default()) {
        tracing::error!(?e, "accessibility worker is gone");
    }
    a11y.on_changed({
        let ui = ui.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let opts = read(&ui.global::<A11y<'_>>());
            crate::SETTINGS.write().a11y = opts;
            if let Err(e) = tx.send(opts) {
                tracing::error!(?e, "accessibility worker is gone");
            }
        }
    });
}

fn read(a11y: &A11y<'_>) -> Opts {
    Opts {
        screen_reader: a11y.get_screen_reader(),
        zoom: a11y.get_zoom(),
        sticky_keys: a11y.get_sticky_keys(),
        slow_keys: a11y.get_slow_keys(),
        bounce_keys: a11y.get_bounce_keys(),
        dwell_click: a11y.get_dwell_click(),
        larger_text: a11y.get_larger_text(),
        legible_font: a11y.get_legible_font(),
        reduce_motion: a11y.get_reduce_motion(),
        color_filter: u8::try_from(a11y.get_color_filter()).unwrap_or_default(),
    }
}

fn worker(rx: &mpsc::Receiver<Opts>) {
    let bus = Connection::session().inspect_err(|e| tracing::error!(?e, "no session bus")).ok();
    // the session's config may be left over from an earlier run, so the first one writes it all
    let mut applied: Option<Opts> = None;
    while let Ok(mut next) = rx.recv() {
        // only the latest matters if several came in at once
        while let Ok(newer) = rx.try_recv() {
            next = newer;
        }
        let prev = applied.unwrap_or_default();
        write_config(applied.as_ref(), &next);
        if let Some(bus) = &bus {
            apply_effects(bus, applied.is_none(), &prev, &next);
        }
        if next.screen_reader != prev.screen_reader {
            set_orca(next.screen_reader);
        }
        applied = Some(next);
    }
    set_orca(false);
}

/// writes what KWin watches for (keyboard filters, animation speed, plugins), notifying it
fn write_config(prev: Option<&Opts>, next: &Opts) {
    let old = prev.map(Opts::session_entries).unwrap_or_default();
    for entry in next.session_entries() {
        if old.contains(&entry) {
            continue;
        }
        // in the OOBE user's own config, the one this KWin session reads
        let mut args = entry.kwriteconfig_args(entry.file);
        args.push("--notify".to_owned());
        match std::process::Command::new("kwriteconfig6").args(&args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => tracing::warn!(?status, ?entry, "kwriteconfig6 failed"),
            Err(e) => tracing::warn!(?e, ?entry, "cannot run kwriteconfig6"),
        }
    }
}

/// zoom, the color filter and dwell click are KWin effects, loaded and set over D-Bus
fn apply_effects(bus: &Connection, first: bool, prev: &Opts, next: &Opts) {
    if first || prev.color_filter != next.color_filter {
        let method = match (prev.color_filter, next.color_filter) {
            (_, 0) => "unloadEffect",
            // the mode was written with the config, a loaded effect has to read it again
            (1.., _) if !first => "reconfigureEffect",
            _ => "loadEffect",
        };
        effect(bus, method, "colorblindnesscorrection");
    }
    if first || prev.dwell_click != next.dwell_click {
        effect(bus, if next.dwell_click { "loadEffect" } else { "unloadEffect" }, "dwellclicker");
    }
    // not on the first change, which would undo zooming the user did with Meta+=
    if prev.zoom != next.zoom {
        if next.zoom {
            for _ in 0..ZOOM_STEPS {
                shortcut(bus, "view_zoom_in");
            }
        } else {
            shortcut(bus, "view_actual_size");
        }
    }
}

fn effect(bus: &Connection, method: &str, name: &str) {
    if let Err(e) = bus.call_method(
        Some("org.kde.KWin"),
        "/Effects",
        Some("org.kde.kwin.Effects"),
        method,
        &(name,),
    ) {
        tracing::warn!(?e, method, name, "cannot change KWin effect");
    }
}

/// runs one of KWin's global shortcuts, which KWin handles itself on Wayland
fn shortcut(bus: &Connection, name: &str) {
    if let Err(e) = bus.call_method(
        Some("org.kde.kglobalaccel"),
        "/component/kwin",
        Some("org.kde.kglobalaccel.Component"),
        "invokeShortcut",
        &(name,),
    ) {
        tracing::warn!(?e, name, "cannot run KWin shortcut");
    }
}

fn set_orca(on: bool) {
    let mut orca = ORCA.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(mut child) = orca.take() {
        if let Err(e) = child.kill() {
            tracing::warn!(?e, "cannot stop Orca");
        }
        if let Err(e) = child.wait() {
            tracing::warn!(?e, "cannot wait for Orca");
        }
    }
    if on {
        // Orca turns on AT-SPI's screen reader flag, which makes Slint expose the UI to it
        match std::process::Command::new("orca").arg("--replace").spawn() {
            Ok(child) => *orca = Some(child),
            Err(e) => tracing::error!(?e, "cannot start Orca"),
        }
    }
}
