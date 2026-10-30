//! running the installation and leaving the OOBE
use libtaidan::{InstallMsg, steps::Stage};
use slint::ComponentHandle;

use crate::ui::{AppWindow, Drivers, Install, InstallState};

/// whether this is a developer running Taidan on their own machine, where installing (creating a
/// user, running dnf, …) or rebooting would be destructive, the same rule as libtaidan's `set_keymap`
fn is_dev_session() -> bool {
    cfg!(debug_assertions)
        && uzers::get_current_username().is_none_or(|user| user.as_encoded_bytes() != b"taidan")
}

/// translation id of the stage's description
const fn stage_l10n_id(stage: Stage) -> &'static str {
    match stage {
        Stage::UserAdd(_) => "steps-useradd",
        Stage::SetTheme(_) => "steps-settheme",
        Stage::DnfDownloadUpdate(_) => "steps-dnfdownloadupdate",
        Stage::DnfInstallUpdate(_) => "steps-dnfinstallupdate",
        Stage::Script(_) => "steps-script",
        Stage::DnfDownloadApps(_) => "steps-dnfdownloadapps",
        Stage::DnfInstallApps(_) => "steps-dnfinstallapps",
        Stage::DriversCodecs(_) => "steps-driverscodecs",
        Stage::SetupImf(_) => "steps-setupimf",
    }
}

/// libtaidan's view of [`crate::CFG`]
fn backend_cfg() -> libtaidan::cfg::Config {
    let json = serde_json::to_value(&*crate::CFG).expect("cannot serialize cfg");
    let mut cfg: libtaidan::cfg::Config =
        serde_json::from_value(json).expect("cfg is not a valid libtaidan cfg");
    cfg.distro.clone_from(&crate::CFG.distro);
    cfg
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
    install.set_stage_count(i32::try_from(Stage::all().len()).expect("too many stages"));
    crate::SETTINGS.write().install_codecs_drivers = app.global::<Drivers<'_>>().get_install();
    let mut settings = crate::SETTINGS.read().clone();
    // the password is in `SETTINGS` now and there's no way back to its page, so drop the
    // plaintext copies in the UI instead of keeping them around for the rest of the session
    let account = app.global::<crate::ui::Account<'_>>();
    account.set_password("".into());
    account.set_password_repeat("".into());
    let ui = ui.clone();
    std::thread::spawn(move || {
        let callback = {
            let ui = ui.clone();
            move |msg| on_install_msg(&ui, msg)
        };
        let res = if is_dev_session() {
            tracing::warn!("Dev session: simulating the installation instead");
            crate::RT.block_on(simulate_install(&callback))
        } else {
            crate::RT.block_on(libtaidan::start_install(&mut settings, &backend_cfg(), &callback))
        };
        let error = res.err().map(|e| {
            let msg = format!("{e:?}");
            tracing::error!(msg, "installation failed");
            sentry::capture_message(&msg, sentry::Level::Error);
            strip_ansi_escapes::strip_str(msg)
        });
        let set = ui.upgrade_in_event_loop(move |ui| {
            let install = ui.global::<Install<'_>>();
            if let Some(error) = error {
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

/// marks the system as configured so the OOBE doesn't run again, then reboots or exits
async fn finish(reboot: bool) {
    if is_dev_session() {
        tracing::warn!(reboot, "Dev session: not marking the system configured, nor rebooting");
    } else {
        if let Err(e) = libtaidan::root("rm", ["-rf", "/var/.unconfigured", "/.unconfigured"]).await
        {
            tracing::error!(?e, "cannot remove /var/.unconfigured and/or /.unconfigured");
        }
        if let Err(e) =
            libtaidan::root("systemctl", ["disable", "taidan-initial-setup-reconfiguration"]).await
        {
            tracing::error!(?e, "cannot disable taidan-initial-setup-reconfiguration.service");
        }
        if reboot && let Err(e) = libtaidan::root("systemctl", ["reboot"]).await {
            tracing::error!(?e, "cannot reboot");
        }
    }
    if let Err(e) = slint::invoke_from_event_loop(|| _ = slint::quit_event_loop()) {
        tracing::error!(?e, "cannot quit");
    }
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, install: &Install<'_>) {
    install.on_start({
        let ui = ui.clone();
        move || start(&ui)
    });
    install.on_finish(|reboot| {
        std::thread::spawn(move || crate::RT.block_on(finish(reboot)));
    });

    // detect hardware for the drivers page in the background, as `lspci` can take a moment
    let ui = ui.clone();
    std::thread::spawn(move || {
        let hw = crate::RT.block_on(libtaidan::steps::detect_hardware());
        tracing::debug!(?hw, "Detected hardware");
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
