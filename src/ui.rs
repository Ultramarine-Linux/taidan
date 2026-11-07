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
    crate::cfg::initialize_ui(ui.as_weak(), &cfg);
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
    ui.set_pagenum(crate::cfg::first_page());
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
/// like systemd's `systemd.debug_shell`. The taidan user can pkexec anything, so the root terminal
/// has to be opt-in.
fn debug_shell_enabled() -> bool {
    match std::fs::read_to_string("/proc/cmdline") {
        Ok(cmdline) => cmdline_has_debug_shell(&cmdline),
        Err(e) => {
            tracing::error!(?e, "cannot read /proc/cmdline");
            false
        }
    }
}

fn cmdline_has_debug_shell(cmdline: &str) -> bool {
    cmdline.split_ascii_whitespace().any(|arg| match arg.split_once('=') {
        None => arg == "taidan.debug_shell",
        Some((key, value)) => {
            key == "taidan.debug_shell" && matches!(value, "1" | "yes" | "y" | "true" | "on")
        }
    })
}

fn init_actions(_ui: slint::Weak<impl slint::ComponentHandle + 'static>, actions: &Actions<'_>) {
    actions.on_power_off(|| {
        if crate::install::is_dev_session() {
            tracing::warn!("Dev session: not powering off");
            return;
        }
        // as root, which logind lets through the unit's inhibitor
        std::thread::spawn(|| {
            if let Err(e) = crate::RT.block_on(libtaidan::root("systemctl", ["poweroff"])) {
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
            let status = std::process::Command::new("pkexec")
                .args(["--user", "root", "env"])
                .args(std::env::vars_os().map(|(mut k, v)| {
                    k.push("=");
                    k.push(v);
                    k
                }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_shell_needs_the_flag() {
        assert!(cmdline_has_debug_shell("BOOT_IMAGE=/vmlinuz ro taidan.debug_shell quiet\n"));
        assert!(cmdline_has_debug_shell("ro taidan.debug_shell=1"));
        assert!(cmdline_has_debug_shell("ro taidan.debug_shell=yes"));
        assert!(!cmdline_has_debug_shell("BOOT_IMAGE=/vmlinuz ro rhgb quiet\n"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shell=0"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shell=no"));
        assert!(!cmdline_has_debug_shell("ro systemd.debug_shell"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shellx"));
    }
}
