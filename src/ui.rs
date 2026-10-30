// Slint's generated code only allows clippy::{all,pedantic,nursery}, not the restriction lints we enable.
#[allow(clippy::restriction)]
mod generated {
    slint::include_modules!();
}
pub use generated::*;

/// # Panics
/// If the UI cannot be created or the event loop fails.
pub fn run() {
    tracing::debug!("Starting Taidan");
    let ui = AppWindow::new().expect("cannot create app");
    // Wayland ignores the window `icon`; compositors look up `<app_id>.desktop` instead.
    // Must be set after the platform exists (AppWindow::new) and before the window is shown.
    slint::set_xdg_app_id(crate::APPID).expect("cannot set xdg app id");
    crate::l10n::initialize_ui(&ui.as_weak(), &ui.global::<Lang<'_>>());
    let cfg: Cfg<'_> = ui.global();
    crate::cfg::initialize_ui(ui.as_weak(), &cfg);
    crate::account::initialize_ui(ui.as_weak(), &ui.global());
    crate::install::initialize_ui(&ui.as_weak(), &ui.global());
    crate::avatar::initialize_ui(&ui.as_weak(), &ui.global());
    crate::network::initialize_ui(&ui.as_weak(), &ui.global());
    init_actions(ui.as_weak(), &ui.global());
    // autoscale(&ui);
    ui.run().expect("cannot run ui");
}

fn init_actions(_ui: slint::Weak<impl slint::ComponentHandle + 'static>, actions: &Actions<'_>) {
    actions.on_open_terminal(|| {
        // wait on a separate thread so the UI keeps running while the terminal is open
        std::thread::spawn(|| {
            let status = std::process::Command::new("pkexec")
                .args(["--user", "root", "env"])
                .args(std::env::vars().map(|(k, v)| format!("{k}={v}")))
                .args(["xdg-terminal-exec", "--", "sh", "-c"])
                .arg("echo 'Taidan has detected Alt+Shift+T. A terminal with superuser privilege is opened for debugging purposes. Only proceed if you know what you are doing, OTHERWISE YOU MAY RISK DATA LOSS OR CAUSE DAMAGES TO YOUR DEVICE. To exit the terminal, press Ctrl+D.' && sh")
                .status();
            match status {
                Ok(status) if !status.success() => tracing::warn!(?status, "debug terminal exited"),
                Ok(_) => {}
                Err(e) => tracing::error!(?e, "cannot spawn pkexec for debug terminal"),
            }
        });
    });
}
