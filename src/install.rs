//! running the installation and leaving the OOBE
use libtaidan::{InstallMsg, steps::Stage};
use slint::ComponentHandle;

use crate::ui::{AppWindow, Drivers, Install, InstallState, Location, Network, Theme, ThemeMode};

/// whether this is a developer running Taidan on their own machine, where installing (creating a
/// user, running dnf, …) or rebooting would be destructive, the same rule as libtaidan's `set_keymap`
#[must_use]
pub fn is_dev_session() -> bool {
    cfg!(debug_assertions)
        && uzers::get_current_username().is_none_or(|user| user.as_encoded_bytes() != b"taidan")
}

/// translation id of the stage's description
const fn stage_l10n_id(stage: Stage) -> &'static str {
    match stage {
        Stage::UserAdd(_) => "steps-useradd",
        Stage::SetHostname(_) => "steps-sethostname",
        Stage::SetTimezone(_) => "steps-settimezone",
        Stage::SetTheme(_) => "steps-settheme",
        Stage::SystemSetup(_) => "steps-systemsetup",
        Stage::DnfDownloadApps(_) => "steps-dnfdownloadapps",
        Stage::DnfInstallApps(_) => "steps-dnfinstallapps",
        Stage::DriversCodecs(_) => "steps-driverscodecs",
        Stage::SetupImf(_) => "steps-setupimf",
        Stage::SetLocale(_) => "steps-setlocale",
        Stage::DownloadUpdates(_) => "steps-dnfdownloadupdate",
    }
}

/// libtaidan's view of [`crate::CFG`]
fn backend_cfg() -> serde_json::Result<libtaidan::cfg::Config> {
    let json = serde_json::to_value(&*crate::CFG)?;
    let mut cfg: libtaidan::cfg::Config = serde_json::from_value(json)?;
    cfg.distro.clone_from(&crate::CFG.distro);
    Ok(cfg)
}

/// the text of a panic's payload
fn panic_msg(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

/// runs the installation (or [`simulate_install`]), the error message if it fails or panics
fn run_install(
    settings: &mut libtaidan::settings::Settings,
    callback: &(impl libtaidan::Callback + Sync),
) -> Option<String> {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if is_dev_session() {
            tracing::warn!("Dev session: simulating the installation instead");
            return crate::RT.block_on(simulate_install(callback)).map_err(|e| format!("{e:?}"));
        }
        let cfg = backend_cfg().map_err(|e| format!("cannot convert the config: {e:?}"))?;
        crate::RT
            .block_on(libtaidan::start_install(settings, &cfg, callback))
            .map_err(|e| format!("{e:?}"))
    }));
    let msg = match res {
        Ok(Ok(())) => return None,
        Ok(Err(msg)) => msg,
        Err(payload) => format!("the installation crashed: {}", panic_msg(&*payload)),
    };
    tracing::error!(msg, "installation failed");
    sentry::capture_message(&msg, sentry::Level::Error);
    Some(strip_ansi_escapes::strip_str(msg))
}

/// whether `TAIDAN_SIMULATE_HW` pretends `name` is present, a comma-separated list of `nvidia`,
/// `broadcom-wifi`, `broadcom-bluetooth`, or `all`
fn simulated_hw(name: &str) -> bool {
    std::env::var("TAIDAN_SIMULATE_HW")
        .is_ok_and(|sim| sim.split(',').any(|s| s.trim() == name || s.trim() == "all"))
}

/// pretends to install, for [`is_dev_session`], set `TAIDAN_SIMULATE_ERROR` to fail halfway
async fn simulate_install(
    callback: &(impl libtaidan::Callback + Sync),
) -> libtaidan::prelude::Res<()> {
    for (i, stage) in Stage::all().iter().enumerate() {
        callback.send(InstallMsg::UpdStage(*stage));
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        if i == 4 && std::env::var_os("TAIDAN_SIMULATE_ERROR").is_some() {
            return Err(libtaidan::prelude::Err::Msg("simulated error (TAIDAN_SIMULATE_ERROR)"));
        }
    }
    if simulated_hw("nvidia") {
        callback.send(InstallMsg::NeedsReboot);
    }
    // to see the done page with updates waiting
    if std::env::var_os("TAIDAN_SIMULATE_UPDATES").is_some() {
        callback.send(InstallMsg::UpdatesReady);
    }
    callback.send(InstallMsg::Finish);
    Ok(())
}

fn on_install_msg(ui: &slint::Weak<AppWindow>, msg: InstallMsg) {
    let set = ui.upgrade_in_event_loop(move |ui| {
        let install = ui.global::<Install<'_>>();
        match msg {
            InstallMsg::UpdStage(stage) => {
                install.set_stage_id(stage_l10n_id(stage).into());
                install.set_stage_num(i32::from(u8::from(stage)) + 1);
                install.set_stage_progress(-1.0);
            }
            #[expect(clippy::cast_possible_truncation, reason = "a fraction, for display")]
            InstallMsg::StageProgress(frac) => install.set_stage_progress(frac as f32),
            InstallMsg::NeedsReboot => install.set_needs_reboot(true),
            InstallMsg::UpdatesReady => install.set_updates_ready(true),
            InstallMsg::Finish => {}
        }
    });
    if let Err(e) = set {
        tracing::error!(?e, "cannot update install progress");
    }
}

fn start(ui: &slint::Weak<AppWindow>) {
    let Some(app) = ui.upgrade() else { return };
    let install = app.global::<Install<'_>>();
    if install.get_state() == InstallState::Running {
        return;
    }
    install.set_state(InstallState::Running);
    install.set_error("".into());
    install.set_needs_reboot(false);
    install.set_updates_ready(false);
    install.set_stage_count(i32::try_from(Stage::all().len()).expect("too many stages"));
    crate::SETTINGS.write().install_codecs_drivers = app.global::<Drivers<'_>>().get_install();
    crate::SETTINGS.write().location = app.global::<Location<'_>>().get_enabled();
    crate::SETTINGS.write().download_updates = install.get_download_updates();
    // what the theme page shows, unless it's skipped, then the distro's default stays
    crate::SETTINGS.write().dark_mode =
        (!crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Theme))
            .then(|| app.global::<Theme<'_>>().get_mode() == ThemeMode::Dark);
    // with the mode only, a skipped theme page keeps the distro's accent too
    let accent = crate::SETTINGS.read().dark_mode.and_then(|_| crate::theme::picked(&app.global()));
    crate::SETTINGS.write().accent = accent;
    // like the theme, a skipped language page keeps the distro's system locale. The UI language
    // only comes from the system's then anyway, or is English if it's one we don't have
    if crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Language) {
        crate::SETTINGS.write().locale.clear();
    }
    // the input methods the keyboard page and summary show, even the ones that only followed the
    // UI language, since the locale they came from may have just been cleared
    let ims = crate::ims::enabled(&app.global());
    crate::SETTINGS.write().ims = ims;
    crate::SETTINGS.write().ims_picked = true;
    // `Network.save` only runs from the network page. With it skipped the device name stays empty,
    // so the system keeps its hostname as when the name is left blank. Nobody chose to go offline
    // either, so go by the connection right now, as the summary does. Leaving `nointernet` off on
    // an offline machine makes the dnf steps fail.
    let online = app.global::<Network<'_>>().get_online();
    if crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Network) {
        crate::SETTINGS.write().nointernet = !online;
    }
    let mut settings = crate::SETTINGS.read().clone();
    // the rest only goes on this attempt's copy. `SETTINGS.nointernet` stays what the network page
    // chose, so a later retry gets the online steps back once the connection is back
    if !online && !settings.nointernet {
        // online when leaving the network page, but not anymore, e.g. on a retry after the
        // connection dropped
        tracing::warn!("The connection is gone since the network page, installing offline");
        settings.nointernet = true;
    }
    // the error page's way out when an online step keeps failing. Skipping them all still leaves
    // a working system, just without updates and the extra software
    if install.get_force_offline() {
        tracing::warn!("Retrying without the online steps");
        settings.nointernet = true;
    }
    // the password is in `SETTINGS` now, so drop the plaintext copies in the UI instead of keeping
    // them around for the rest of the session. Going back from the error page to its page means
    // typing it again, retrying from the summary keeps the saved one.
    let account = app.global::<crate::ui::Account<'_>>();
    account.set_password("".into());
    account.set_password_repeat("".into());
    let ui = ui.clone();
    std::thread::spawn(move || {
        let callback = {
            let ui = ui.clone();
            move |msg| on_install_msg(&ui, msg)
        };
        let error = run_install(&mut settings, &callback);
        let used_internet = !settings.nointernet;
        let set = ui.upgrade_in_event_loop(move |ui| {
            let install = ui.global::<Install<'_>>();
            if let Some(error) = error {
                install.set_used_internet(used_internet);
                install.set_error(error.into());
                install.set_state(InstallState::Failed);
            } else {
                install.set_state(InstallState::Done);
            }
        });
        if let Err(e) = set {
            tracing::error!(?e, "cannot report install result");
        }
    });
}

/// whether [`finish`] has started
static FINISHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// marks the system as configured so the OOBE doesn't run again, then reboots or exits
async fn finish(reboot: bool) {
    if is_dev_session() {
        tracing::warn!(reboot, "Dev session: not marking the system configured, nor rebooting");
    } else {
        // libtaidan did these at the end of the installation already, this only makes sure. Both
        // succeed when there's nothing left to do
        if let Err(e) = libtaidan::root("rm", ["-rf", "/var/.unconfigured", "/.unconfigured"]).await
        {
            tracing::warn!(?e, "cannot remove /var/.unconfigured and/or /.unconfigured");
        }
        if let Err(e) =
            libtaidan::root("systemctl", ["disable", "taidan-initial-setup-reconfiguration"]).await
        {
            tracing::warn!(?e, "cannot disable taidan-initial-setup-reconfiguration.service");
        }
        if reboot && let Err(e) = libtaidan::root("systemctl", ["reboot"]).await {
            tracing::error!(?e, "cannot reboot");
        }
    }
    // the process ends with the event loop, and these would outlive it
    crate::network::shutdown();
    crate::a11y::shutdown();
    if let Err(e) = slint::invoke_from_event_loop(|| _ = slint::quit_event_loop()) {
        tracing::error!(?e, "cannot quit");
    }
}

/// back to [`InstallState::Idle`] after a failure, so the summary page can start it again
fn reset(ui: &slint::Weak<AppWindow>) {
    let Some(app) = ui.upgrade() else { return };
    let install = app.global::<Install<'_>>();
    if install.get_state() == InstallState::Running {
        return;
    }
    install.set_state(InstallState::Idle);
    install.set_error("".into());
    install.set_used_internet(false);
    install.set_force_offline(false);
    install.set_needs_reboot(false);
    install.set_updates_ready(false);
    install.set_stage_id("".into());
    install.set_stage_num(0);
    install.set_stage_progress(-1.0);
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, install: &Install<'_>) {
    // dnf can't install to rpm-ostree systems, and distros that skip dnf don't want it run
    install.set_can_download_updates(
        !crate::CFG.taidan0.skip_dnf && !std::path::Path::new("/ostree").exists(),
    );
    install.on_start({
        let ui = ui.clone();
        move || start(&ui)
    });
    install.on_reset({
        let ui = ui.clone();
        move || reset(&ui)
    });
    install.on_finish({
        let ui = ui.clone();
        move |reboot| {
            // a failed installation may have left no user, so the OOBE must run again
            let state = ui.upgrade().map(|app| app.global::<Install<'_>>().get_state());
            if state != Some(InstallState::Done) {
                tracing::error!(?state, "not finishing, the installation hasn't succeeded");
                return;
            }
            // once, a second press on Done would run it all again
            if FINISHING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            std::thread::spawn(move || crate::RT.block_on(finish(reboot)));
        }
    });
    if let Some(app) = ui.upgrade() {
        let drivers = app.global::<Drivers<'_>>();
        // as on the GTK version, skipping the drivers (`codecs`) page means not installing them
        if crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Drivers) {
            drivers.set_install(false);
        }
        // codecs come from dnf, see `DnfDownloadApps`
        drivers.set_codecs(!crate::CFG.taidan0.skip_dnf);
    }

    // detect hardware for the drivers page in the background, as `lspci` can take a moment
    let ui = ui.clone();
    std::thread::spawn(move || {
        let mut hw = crate::RT.block_on(libtaidan::steps::detect_hardware());
        tracing::debug!(?hw, "Detected hardware");
        // pretend to have hardware, to see the drivers list
        if std::env::var_os("TAIDAN_SIMULATE_HW").is_some() {
            hw.nvidia |= simulated_hw("nvidia");
            hw.broadcom_wifi |= simulated_hw("broadcom-wifi");
            hw.broadcom_bluetooth |= simulated_hw("broadcom-bluetooth");
            tracing::debug!(?hw, "Simulated hardware (TAIDAN_SIMULATE_HW)");
        }
        let set = ui.upgrade_in_event_loop(move |ui| {
            let drivers = ui.global::<Drivers<'_>>();
            drivers.set_nvidia(hw.nvidia);
            drivers.set_broadcom_wifi(hw.broadcom_wifi);
            drivers.set_broadcom_bluetooth(hw.broadcom_bluetooth);
        });
        if let Err(e) = set {
            tracing::error!(?e, "cannot set detected hardware");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_converts_to_backend_cfg() {
        let json = serde_json::to_value(crate::cfg::Config::default()).unwrap();
        serde_json::from_value::<libtaidan::cfg::Config>(json).unwrap();
    }

    #[test]
    fn simulated_install_reports_every_stage() {
        let msgs = std::sync::Mutex::new(vec![]);
        let callback = |msg| msgs.lock().unwrap().push(msg);
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(simulate_install(&callback)).unwrap();
        let msgs = msgs.into_inner().unwrap();
        assert_eq!(msgs.len(), Stage::all().len() + 1);
        assert!(matches!(msgs.last(), Some(InstallMsg::Finish)));
    }
}
