pub use taidan_ui::*;

pub fn run() {
    tracing::debug!("Starting Taidan");
    let screen = crate::screen::select_scaling_backend();
    let ui = AppWindow::new().expect("cannot create app");
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
    crate::wallpaper::initialize_ui(&ui);
    init_actions(&ui.global());
    ui.set_pagenum(first_page);
    ui.set_framed(crate::install::is_dev_session());
    let software = std::env::var("SLINT_BACKEND").is_ok_and(|b| b.contains("software"));
    ui.set_rounded(!crate::install::is_dev_session() && !software);
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
        let out = work();
        on_ui(&ui, move |app| apply(&app, out));
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
