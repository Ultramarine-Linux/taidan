pub use taidan_ui::*;

pub fn run() {
    tracing::debug!("Starting Taidan");
    let screen = crate::screen::select_scaling_backend();
    let ui = AppWindow::new().expect("cannot create app");
    libtaidan::command::set_missing_command_handler({
        let ui = ui.as_weak();
        move |detail| {
            on_ui(&ui, move |app| show_fatal_error(&app, "error-command-title", &detail));
        }
    });
    _ = screen.set(ui.as_weak());
    // Wayland ignores the window icon for `<app_id>.desktop`, which must be set before showing
    slint::set_xdg_app_id(crate::APPID).expect("cannot set xdg app id");
    crate::l10n::initialize_ui(&ui.as_weak(), &ui.global::<Lang<'_>>());
    let nav = crate::cfg::PageNav::new(&crate::CFG, &ui.global());
    let first_page = nav.first();
    crate::cfg::initialize_ui(ui.as_weak(), &ui.global(), nav);
    crate::account::initialize_ui(ui.as_weak(), &ui.global());
    crate::install::initialize_ui(&ui.as_weak(), &ui.global());
    crate::avatar::initialize_ui(&ui.as_weak(), &ui.global());
    crate::network::initialize_ui(&ui.as_weak(), &ui.global());
    crate::timezone::initialize_ui(&ui.as_weak(), &ui.global());
    crate::tweaks::initialize_ui(&ui.as_weak(), &ui.global());
    crate::theme::initialize_ui(&ui.global());
    crate::ims::initialize_ui(&ui.as_weak(), &ui.global());
    crate::a11y::initialize_ui(&ui.as_weak(), &ui.global());
    crate::wallpaper::initialize();
    init_actions(&ui.global());
    ui.set_pagenum(first_page);
    ui.set_framed(crate::install::is_dev_session());
    let software = std::env::var("SLINT_BACKEND").is_ok_and(|b| b.contains("software"));
    ui.set_rounded(!crate::install::is_dev_session() && !software);
    let config_error = crate::CONFIG_ERROR.write().take();
    if let Some(error) = config_error {
        show_fatal_error(&ui, "error-config-title", &error);
    }
    for warning in crate::CFG.warnings() {
        tracing::warn!(%warning, "configuration warning");
        show_diagnostic_warning(&ui, &warning);
    }
    if std::env::var_os("TAIDAN_SIMULATE_WARNING").is_some() {
        show_simulated_warning(&ui);
    }
    ui.window().on_close_requested(|| {
        if crate::install::is_dev_session() {
            slint::CloseRequestResponse::HideWindow
        } else {
            tracing::info!("Ignoring the request to close the window");
            slint::CloseRequestResponse::KeepWindowShown
        }
    });
    ui.run().expect("cannot run ui");
    crate::network::shutdown();
    crate::a11y::shutdown();
    crate::wallpaper::shutdown();
}

/// shows an error in a dialog right away, rather than ending the flow on the error page. For
/// problems the OOBE can't fix itself, like a broken config, that the distro's maintainers need to
/// see too
pub fn show_error(ui: &AppWindow, title: &str, body: &str) {
    show_dialog(ui, title, body, false);
}

/// Show a dismissible warning without turning installation into a failure.
pub fn show_warning(ui: &AppWindow, title: &str, body: &str) {
    show_dialog(ui, title, body, true);
}

fn show_dialog(ui: &AppWindow, title: &str, body: &str, warning: bool) {
    let errors = ui.global::<Errors<'_>>();
    // Keep an earlier startup/worker error visible if another arrives before dismissal.
    if errors.get_open() {
        errors.set_body(format!("{}\n\n{title}\n{body}", errors.get_body()).into());
        // A fatal error supersedes a warning; warnings never downgrade a visible error.
        if !warning && errors.get_warn() {
            errors.set_title(title.into());
            errors.set_warn(false);
        }
        return;
    }
    if warning {
        errors.invoke_show_warn(title.into(), body.into());
    } else {
        errors.invoke_show(title.into(), body.into());
    }
}

pub fn show_diagnostic_warning(ui: &AppWindow, detail: &str) {
    let title = crate::l10n::PO_LOADER.read().get("warning-title");
    show_warning(ui, &title, &diagnostic_body(detail));
}

fn show_simulated_warning(ui: &AppWindow) {
    show_diagnostic_warning(ui, "Simulated warning (TAIDAN_SIMULATE_WARNING).");
}

/// Include local diagnostics without sending potentially sensitive command output to Sentry.
pub fn show_fatal_error(ui: &AppWindow, title_id: &str, detail: &str) {
    let title = crate::l10n::PO_LOADER.read().get(title_id);
    let body = diagnostic_body(detail);
    show_error(ui, &title, &body);
}

pub(crate) fn diagnostic_body(detail: &str) -> String {
    format!(
        "{}\n\nTaidan {}\nUI log: {}/taidan.log\nInstallation logs: {}\n\n{}",
        strip_ansi_escapes::strip_str(detail),
        env!("CARGO_PKG_VERSION"),
        crate::TEMP_DIR.display(),
        libtaidan::TEMP_DIR.display(),
        crate::l10n::PO_LOADER.read().get("error-maintainer-hint"),
    )
}

pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

fn debug_shell_enabled() -> bool {
    match std::fs::read_to_string("/proc/cmdline") {
        Ok(cmdline) => libtaidan::helper::cmdline_has_debug_shell(&cmdline),
        Err(e) => {
            tracing::error!(?e, "cannot read /proc/cmdline");
            false
        }
    }
}

fn init_actions(actions: &Actions<'_>) {
    actions.on_power_off(|| {
        if crate::install::is_dev_session() {
            tracing::warn!("Dev session: not powering off");
            return;
        }
        // logind lets root through the unit's inhibitor
        block_on_thread(async {
            if let Err(e) = libtaidan::root(libtaidan::Verb::PowerOff).await {
                tracing::error!(?e, "cannot power off");
            }
        });
    });
    actions.on_open_terminal(|| {
        block_on_thread(async {
            if !debug_shell_enabled() {
                tracing::info!("Ignoring Alt+Shift+T, boot with taidan.debug_shell to enable it");
                return;
            }
            let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") else {
                tracing::error!("no WAYLAND_DISPLAY for the debug terminal");
                return;
            };
            let verb = libtaidan::Verb::DebugShell { wayland_display };
            if let Err(e) = libtaidan::root(verb).await {
                tracing::warn!(?e, "debug terminal failed or exited");
            }
        });
    });
}

pub fn on_ui<F: FnOnce(AppWindow) + Send + 'static>(ui: &slint::Weak<AppWindow>, f: F) {
    if let Err(e) = ui.upgrade_in_event_loop(f) {
        tracing::error!(?e, "cannot update the UI");
    }
}

pub fn spawn_then<T, W, A>(ui: slint::Weak<AppWindow>, work: W, apply: A)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    A: FnOnce(&AppWindow, T) + Send + 'static,
{
    std::thread::spawn(move || {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
            Ok(out) => on_ui(&ui, move |app| apply(&app, out)),
            Err(payload) => {
                let detail = format!("A background task crashed: {}", panic_message(&*payload));
                tracing::error!("a background task crashed");
                on_ui(&ui, move |app| show_fatal_error(&app, "error-internal-title", &detail));
            }
        }
    });
}

pub fn block_on_thread<F>(future: F) -> std::thread::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    std::thread::spawn(move || crate::RT.block_on(future))
}

pub fn latest_worker<T, M, H>(merge: M, mut handle: H) -> std::sync::mpsc::Sender<T>
where
    T: Send + 'static,
    M: Fn(T, T) -> T + Send + 'static,
    H: FnMut(T) + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(oldest) = rx.recv() {
            handle(rx.try_iter().fold(oldest, &merge));
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    slint::slint! {
        import { TdnDialog, TdnErrorDialog, TdnPara } from "ui/components.slint";

        export component DialogLayoutTest inherits Window {
            width: 600px;
            height: 500px;
            in property <string> body;
            in property <string> heading: "Dialog";
            out property <length> base-height: base.dialog-height;
            out property <length> error-height: error.dialog-height;
            out property <length> body-content-height: error.body-content-height;
            out property <length> body-visible-height: error.body-visible-height;

            base := TdnDialog {
                title: root.heading;
                dialog-width: 30rem;
                TdnPara { text: root.body; }
            }
            error := TdnErrorDialog {
                title: root.heading;
                body: root.body;
            }
        }
    }

    struct HeadlessPlatform;
    impl slint::platform::Platform for HeadlessPlatform {
        fn create_window_adapter(
            &self,
        ) -> Result<std::rc::Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
            Ok(slint::platform::software_renderer::MinimalSoftwareWindow::new(
                slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
            ))
        }
    }

    #[test]
    fn dialogs_grow_for_wrapped_text_and_scroll_at_the_window_limit() {
        slint::platform::set_platform(Box::new(HeadlessPlatform)).unwrap();
        let ui = DialogLayoutTest::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(600.0, 500.0));
        ui.set_body("Short text".into());
        let short_base = ui.get_base_height();
        let short_error = ui.get_error_height();
        ui.set_heading("A long dialog heading that wraps across multiple lines. ".repeat(6).into());
        assert!(ui.get_base_height() > short_base, "wrapped heading did not grow the dialog");
        ui.set_heading("Dialog".into());
        ui.set_body("This is a wrapped line of diagnostic information. ".repeat(12).into());
        assert!(ui.get_base_height() > short_base, "base dialog did not grow");
        assert!(
            ui.get_error_height() > short_error,
            "error dialog did not grow: short={short_error}, current={}, content={}, visible={}",
            ui.get_error_height(),
            ui.get_body_content_height(),
            ui.get_body_visible_height()
        );
        assert!(
            ui.get_body_content_height() <= ui.get_body_visible_height() + 1.0,
            "dialog should fit medium text before scrolling: content={}, visible={}",
            ui.get_body_content_height(),
            ui.get_body_visible_height()
        );
        let wide_content_height = ui.get_body_content_height();
        ui.window().set_size(slint::LogicalSize::new(400.0, 500.0));
        assert!(ui.get_body_content_height() >= wide_content_height);
        ui.set_body("Long diagnostic line.\n".repeat(100).into());
        assert!(ui.get_error_height() <= 500.0);
        assert!(ui.get_body_visible_height() > 0.0);
        assert!(ui.get_body_content_height() > ui.get_body_visible_height());
    }

    #[test]
    fn fatal_dialog_opens_and_preserves_an_earlier_error() {
        slint::platform::set_platform(Box::new(HeadlessPlatform)).unwrap();
        let ui = AppWindow::new().unwrap();
        show_fatal_error(&ui, "error-config-title", "invalid TOML");
        let errors = ui.global::<Errors<'_>>();
        assert!(errors.get_open());
        assert!(!errors.get_warn());
        assert!(errors.get_body().contains("invalid TOML"));
        show_fatal_error(&ui, "error-installation-title", "useradd failed");
        assert!(errors.get_body().contains("invalid TOML"));
        assert!(errors.get_body().contains("useradd failed"));
        errors.set_open(false);
        show_fatal_error(&ui, "error-internal-title", "worker crashed");
        assert!(errors.get_open());
        assert!(errors.get_body().contains("worker crashed"));
        assert!(!errors.get_body().contains("invalid TOML"));

        errors.set_open(false);
        show_diagnostic_warning(&ui, "non-fatal warning");
        assert!(errors.get_open());
        assert!(errors.get_warn());
        show_warning(&ui, "Another warning", "second warning");
        assert!(errors.get_warn());
        assert!(errors.get_body().contains("non-fatal warning"));
        assert!(errors.get_body().contains("second warning"));
        show_fatal_error(&ui, "error-config-title", "invalid steps");
        assert!(!errors.get_warn());
        assert!(errors.get_body().contains("invalid steps"));
        show_warning(&ui, "Warning", "later warning");
        assert!(!errors.get_warn());
        assert!(errors.get_body().contains("invalid steps"));
        assert!(errors.get_body().contains("later warning"));

        errors.set_open(false);
        let (fallback, error) =
            crate::cfg::prepare_config(serde_json::from_value::<crate::cfg::Config>(
                serde_json::json!({"steps": [{"stage": "invalid_stage"}]}),
            ));
        assert!(!fallback.should_exit);
        show_fatal_error(&ui, "error-config-title", &error.unwrap());
        assert!(errors.get_open());
        assert!(!errors.get_warn());
        assert!(errors.get_body().contains("invalid_stage"));

        errors.set_open(false);
        show_simulated_warning(&ui);
        assert!(errors.get_open());
        assert!(errors.get_warn());
        assert!(errors.get_body().contains("TAIDAN_SIMULATE_WARNING"));
    }

    #[test]
    fn diagnostics_include_log_locations_and_strip_terminal_escapes() {
        let body = diagnostic_body("\x1b[31mfailed\x1b[0m");
        assert!(body.starts_with("failed\n"));
        assert!(!body.contains('\x1b'));
        assert!(body.contains("taidan.log"));
        assert!(body.contains(&libtaidan::TEMP_DIR.display().to_string()));
    }

    #[test]
    fn panic_messages_handle_both_string_types() {
        assert_eq!(panic_message(&"failed"), "failed");
        assert_eq!(panic_message(&String::from("failed")), "failed");
        assert_eq!(panic_message(&42_u32), "unknown panic");
    }
}
