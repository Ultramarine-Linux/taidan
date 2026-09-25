use slint::ToSharedString;

slint::include_modules!();

pub fn run() {
    tracing::debug!("Starting Taidan");
    let ui = AppWindow::new().expect("cannot create app");
    // TODO: refactor
    let theme = ui.global::<Theme<'_>>();
    theme.set_mode(ThemeMode::Light);
    crate::l10n::initialize_ui(ui.as_weak(), ui.global::<Lang<'_>>());
    let cfg: Cfg<'_> = ui.global();
    crate::cfg::initialize_ui(ui.as_weak(), cfg);
    init_actions(ui.as_weak(), ui.global());
    // autoscale(&ui);
    ui.run().expect("cannot run ui");
}

fn init_actions(ui: slint::Weak<impl slint::ComponentHandle + 'static>, actions: Actions<'_>) {
    actions.on_open_terminal(|| {
        // TODO: handle this
        std::process::Command::new("pkexec")
            .args(["--user", "root", "env"])
            .args(std::env::vars().map(|(k, v)| format!("{k}={v}")))
            .args(["xdg-terminal-exec", "--", "sh", "-c"])
            .arg("echo 'Taidan has detected Alt+Shift+T. A terminal with superuser privilege is opened for debugging purposes. Only proceed if you know what you are doing, OTHERWISE YOU MAY RISK DATA LOSS OR CAUSE DAMAGES TO YOUR DEVICE. To exit the terminal, press Ctrl+D.' && sh")
            .spawn().expect("cannot spawn pkexec");
    });
}
