pub use taidan_ui::*;

/// # Panics
/// If the UI cannot be created or the event loop fails.
pub fn run() {
    tracing::debug!("Starting Taidan");
    let screen = crate::screen::init();
    let ui = AppWindow::new().expect("cannot create app");
    // set before the event loop starts, which is when the screens are known
    if screen.set(ui.as_weak()).is_err() {
        tracing::warn!("window already given to the screen scaling");
    }
    // Wayland ignores the window `icon`; compositors look up `<app_id>.desktop` instead.
    // Must be set after the platform exists (AppWindow::new) and before the window is shown.
    slint::set_xdg_app_id(crate::APPID).expect("cannot set xdg app id");
    crate::l10n::initialize_ui(&ui.as_weak(), &ui.global::<Lang<'_>>());
    let cfg: Cfg<'_> = ui.global();
    let nav = crate::cfg::PageNav::new(&crate::CFG, &ui.global());
    let first_page = nav.first();
    crate::cfg::initialize_ui(ui.as_weak(), &cfg, nav);
    crate::account::initialize_ui(ui.as_weak(), &ui.global());
    crate::install::initialize_ui(&ui.as_weak(), &ui.global());
    crate::avatar::initialize_ui(&ui.as_weak(), &ui.global());
    crate::network::initialize_ui(&ui.as_weak(), &ui.global());
    crate::timezone::initialize_ui(&ui.as_weak(), &ui.global());
    crate::tweaks::initialize_ui(&ui.as_weak(), &ui.global());
    crate::theme::initialize_ui(&ui.global());
    crate::ims::initialize_ui(&ui.as_weak(), &ui.global());
    crate::a11y::initialize_ui(&ui.as_weak(), &ui.global());
    init_actions(ui.as_weak(), &ui.global());
    ui.set_pagenum(first_page);
    ui.set_framed(crate::install::is_dev_session());
    // the OOBE has to be completed, Alt+F4 shouldn't drop to an unconfigured system. Finishing
    // quits the event loop directly instead of closing the window.
    ui.window().on_close_requested(|| {
        if crate::install::is_dev_session() {
            slint::CloseRequestResponse::HideWindow
        } else {
            tracing::info!("Ignoring the request to close the window");
            slint::CloseRequestResponse::KeepWindowShown
        }
    });
    ui.run().expect("cannot run ui");
    // closing the window in a dev session skips finish(), so stop the helpers here too
    crate::network::shutdown();
    crate::a11y::shutdown();
}

/// whether `taidan.debug_shell` is on the kernel command line, either bare or set to a true value
/// like systemd's `systemd.debug_shell`. A root terminal has to be opt-in. The helper checks it
/// too, this only saves asking it
fn debug_shell_enabled() -> bool {
    match std::fs::read_to_string("/proc/cmdline") {
        Ok(cmdline) => libtaidan::helper::cmdline_has_debug_shell(&cmdline),
        Err(e) => {
            tracing::error!(?e, "cannot read /proc/cmdline");
            false
        }
    }
}

fn init_actions(_ui: slint::Weak<impl slint::ComponentHandle + 'static>, actions: &Actions<'_>) {
    actions.on_power_off(|| {
        if crate::install::is_dev_session() {
            tracing::warn!("Dev session: not powering off");
            return;
        }
        // as root, which logind lets through the unit's inhibitor
        std::thread::spawn(|| {
            if let Err(e) = crate::RT.block_on(libtaidan::root(libtaidan::Verb::PowerOff)) {
                tracing::error!(?e, "cannot power off");
            }
        });
    });
    actions.on_open_terminal(|| {
        // wait on a separate thread so the UI keeps running while the terminal is open
        std::thread::spawn(|| {
            if !debug_shell_enabled() {
                tracing::info!("Ignoring Alt+Shift+T, boot with taidan.debug_shell to enable it");
                return;
            }
            let Some(wayland_display) = std::env::var("WAYLAND_DISPLAY").ok() else {
                tracing::error!("no WAYLAND_DISPLAY for the debug terminal");
                return;
            };
            let verb = libtaidan::Verb::DebugShell { wayland_display };
            match crate::RT.block_on(libtaidan::root(verb)) {
                Ok(()) => {}
                Err(e) => tracing::warn!(?e, "debug terminal failed or exited"),
            }
        });
    });
}
