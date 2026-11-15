use libtaidan::{InstallMsg, steps::Stage};
use slint::ComponentHandle;

use crate::ui::{
    AppWindow, Drivers, Install, InstallState, Location, Network, Pages, Theme, ThemeMode,
};
use crate::ui::{block_on_thread, on_ui, spawn_then};

#[must_use]
pub fn is_dev_session() -> bool {
    cfg!(debug_assertions)
        && uzers::get_current_username().is_none_or(|user| user.as_encoded_bytes() != b"taidan")
}

fn panic_msg(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

fn run_install(
    settings: &mut libtaidan::settings::Settings,
    callback: &(impl libtaidan::Callback + Sync),
) -> Option<String> {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if is_dev_session() {
            tracing::warn!("Dev session: simulating the installation instead");
            return crate::RT.block_on(simulate_install(callback)).map_err(|e| e.to_string());
        }
        crate::RT.block_on(libtaidan::start_install(settings, &crate::CFG, callback)).map_err(|e| {
            // errors go to Sentry and this may name the user
            tracing::debug!(?e, "installation error");
            e.to_string()
        })
    }));
    let msg = match res {
        Ok(Ok(())) => return None,
        Ok(Err(msg)) => msg,
        Err(payload) => format!("the installation crashed: {}", panic_msg(&*payload)),
    };
    tracing::error!(msg, "installation failed");
    Some(strip_ansi_escapes::strip_str(msg))
}

const SECURE_BOOT_ENROLL_URL: &str = "https://fyra.to/secure-boot";

fn qr_code(url: &str) -> Option<slint::Image> {
    let svg = qrcode::QrCode::new(url)
        .inspect_err(|e| tracing::error!(?e, "cannot make QR code"))
        .ok()?
        .render::<qrcode::render::svg::Color<'_>>()
        .quiet_zone(true)
        .build();
    slint::Image::load_from_svg_data(svg.as_bytes())
        .inspect_err(|e| tracing::error!(?e, "cannot load QR code"))
        .ok()
}

const SIMULATED_ERROR_AFTER_STAGE: Stage = Stage::SystemSetup;

fn simulated_hw(name: &str) -> bool {
    std::env::var("TAIDAN_SIMULATE_HW")
        .is_ok_and(|sim| sim.split(',').any(|s| s.trim() == name || s.trim() == "all"))
}

async fn simulate_install(
    callback: &(impl libtaidan::Callback + Sync),
) -> libtaidan::prelude::Res<()> {
    for stage in Stage::all() {
        callback.send(InstallMsg::UpdStage(*stage));
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        if *stage == SIMULATED_ERROR_AFTER_STAGE
            && std::env::var_os("TAIDAN_SIMULATE_ERROR").is_some()
        {
            return Err(libtaidan::prelude::Err::Msg("simulated error (TAIDAN_SIMULATE_ERROR)"));
        }
    }
    if simulated_hw("nvidia") {
        callback.send(InstallMsg::NeedsReboot);
    }
    if std::env::var_os("TAIDAN_SIMULATE_DRIVER_ERROR").is_some() {
        callback.send(InstallMsg::DriversFailed);
    }
    if std::env::var_os("TAIDAN_SIMULATE_UPDATES").is_some() {
        callback.send(InstallMsg::UpdatesReady);
    }
    callback.send(InstallMsg::Finish);
    Ok(())
}

fn on_install_msg(ui: &slint::Weak<AppWindow>, msg: InstallMsg) {
    on_ui(ui, move |ui| {
        let install = ui.global::<Install<'_>>();
        match msg {
            InstallMsg::UpdStage(stage) => {
                install.set_stage_id(stage.l10n_id().into());
                install.set_stage_num(i32::from(u8::from(stage)) + 1);
                install.set_stage_progress(-1.0);
            }
            InstallMsg::StageProgress(frac) => {
                install.set_stage_progress(furthest_stage_progress(
                    install.get_stage_progress(),
                    frac,
                ));
            }
            InstallMsg::NeedsReboot => install.set_needs_reboot(true),
            InstallMsg::DriversFailed => install.set_drivers_failed(true),
            InstallMsg::LocaleFailed => install.set_locale_failed(true),
            InstallMsg::UpdatesReady => install.set_updates_ready(true),
            InstallMsg::Finish => {}
        }
    });
}

#[expect(clippy::cast_possible_truncation, reason = "a fraction, for display")]
const fn furthest_stage_progress(current: f32, frac: f64) -> f32 {
    current.max(frac as f32)
}

fn start(ui: &slint::Weak<AppWindow>) {
    let Some(app) = ui.upgrade() else { return };
    let install = app.global::<Install<'_>>();
    if install.get_state() == InstallState::Running {
        return;
    }
    install.set_state(InstallState::Running);
    install.set_error("".into());
    clear_outcomes_except_needs_reboot(&install);
    clear_stage(&install);
    install.set_stage_count(i32::try_from(Stage::all().len()).expect("too many stages"));
    save_page_choices(&app, &install);
    let net = app.global::<Network<'_>>();
    let online = net.get_install_online();
    let mut settings = crate::SETTINGS.read().clone();
    crate::set_private_values(&settings);
    settings.ims = crate::ims::enabled(&app.global());
    settings.ims_picked = true;
    settings.nointernet = !online;
    if !online {
        tracing::info!(chosen = net.get_offline_chosen(), "Installing without the online steps");
    }
    if install.get_force_offline() {
        tracing::warn!("Retrying without the online steps");
        settings.nointernet = true;
    }
    let account = app.global::<crate::ui::Account<'_>>();
    account.set_password("".into());
    account.set_password_repeat("".into());
    let progress_ui = ui.clone();
    spawn_then(
        ui.clone(),
        move || {
            let error = run_install(&mut settings, &move |msg| on_install_msg(&progress_ui, msg));
            if error.is_none() {
                *A11Y_APPLIED_AT_INSTALL.lock() =
                    Some(AppliedA11y { username: settings.username.clone(), a11y: settings.a11y });
            }
            (error, !settings.nointernet)
        },
        |ui, (error, used_internet)| {
            let install = ui.global::<Install<'_>>();
            // a `changed Install.state` in Slint can coalesce Failed → Running → Failed
            if let Some(error) = error {
                install.set_used_internet(used_internet);
                install.set_error(error.into());
                install.set_state(InstallState::Failed);
                ui.set_pagenum(ui.global::<Pages<'_>>().get_error());
            } else {
                install.set_state(InstallState::Done);
                ui.set_pagenum(ui.global::<Pages<'_>>().get_done());
            }
        },
    );
}

fn save_page_choices(app: &AppWindow, install: &Install<'_>) {
    let install_codecs_drivers = app.global::<Drivers<'_>>().get_install();
    let location = crate::cfg::shows_page(crate::cfg::SkipPage::Location)
        .then(|| app.global::<Location<'_>>().get_enabled());
    let download_updates = install.get_download_updates();
    let proxy_persist = app.global::<Network<'_>>().get_proxy_persist();
    let dark_mode = crate::cfg::shows_page(crate::cfg::SkipPage::Theme)
        .then(|| app.global::<Theme<'_>>().get_mode() == ThemeMode::Dark);
    let accent = dark_mode.and_then(|_| crate::theme::picked(&app.global()));
    let tweaks = crate::tweaks::enabled(&app.global());
    let mut settings = crate::SETTINGS.write();
    settings.tweaks = tweaks;
    settings.install_codecs_drivers = install_codecs_drivers;
    settings.location = location;
    settings.download_updates = download_updates;
    settings.proxy_persist = proxy_persist;
    settings.dark_mode = dark_mode;
    settings.accent = accent;
    if !crate::cfg::shows_page(crate::cfg::SkipPage::Language) {
        settings.locale.clear();
    }
}

fn clear_outcomes_except_needs_reboot(install: &Install<'_>) {
    install.set_updates_ready(false);
    install.set_drivers_failed(false);
    install.set_locale_failed(false);
}

fn clear_stage(install: &Install<'_>) {
    install.set_stage_id("".into());
    install.set_stage_num(0);
    install.set_stage_progress(-1.0);
}

#[derive(Clone)]
struct AppliedA11y {
    username: String,
    a11y: libtaidan::a11y::A11y,
}

static A11Y_APPLIED_AT_INSTALL: parking_lot::Mutex<Option<AppliedA11y>> =
    parking_lot::Mutex::new(None);

async fn reapply_changed_a11y() {
    let applied = A11Y_APPLIED_AT_INSTALL.lock().clone();
    let Some(AppliedA11y { username, a11y: applied }) = applied else { return };
    let a11y = crate::SETTINGS.read().a11y;
    if a11y == applied {
        return;
    }
    // systemd-homed needs the password to open the home again
    let passwd = crate::SETTINGS.read().passwd.clone();
    let settings = libtaidan::settings::Settings { username, a11y, passwd, ..Default::default() };
    if let Err(e) = libtaidan::a11y::reapply_to_user(&settings, &crate::CFG, applied).await {
        tracing::warn!(?e, "cannot carry over the accessibility settings");
    }
}

static FINISHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

async fn finish(ui: &slint::Weak<AppWindow>, reboot: bool) {
    if is_dev_session() {
        tracing::warn!(reboot, "Dev session: not marking the system configured, nor rebooting");
    } else {
        reapply_changed_a11y().await;
        libtaidan::mark_configured().await;
        if reboot && !reboot_with_retries().await {
            FINISHING.store(false, std::sync::atomic::Ordering::SeqCst);
            on_ui(ui, |app| {
                let install = app.global::<Install<'_>>();
                install.set_reboot_failed(true);
                install.set_finishing(false);
            });
            return;
        }
    }
    crate::network::shutdown();
    crate::a11y::shutdown();
    if let Err(e) = slint::invoke_from_event_loop(|| _ = slint::quit_event_loop()) {
        tracing::error!(?e, "cannot quit");
    }
}

async fn reboot_with_retries() -> bool {
    // pkexec can fail for a moment after a VT switch
    for attempt in 1..=3 {
        match libtaidan::root(libtaidan::Verb::Reboot).await {
            Ok(()) => return true,
            Err(e) => tracing::error!(?e, attempt, "cannot reboot"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    false
}

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
    install.set_updates_ready(false);
    install.set_reboot_failed(false);
    clear_stage(&install);
}

pub(crate) fn can_use_dnf() -> bool {
    !crate::CFG.taidan0.skip_dnf && !std::path::Path::new("/ostree").exists()
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, install: &Install<'_>) {
    install.set_can_download_updates(can_use_dnf());
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
            let Some(app) = ui.upgrade() else { return };
            let install = app.global::<Install<'_>>();
            let state = install.get_state();
            if state != InstallState::Done {
                tracing::error!(?state, "not finishing, the installation hasn't succeeded");
                return;
            }
            if FINISHING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            install.set_finishing(true);
            let ui = ui.clone();
            block_on_thread(async move { finish(&ui, reboot).await });
        }
    });
    if let Some(app) = ui.upgrade() {
        app.global::<Drivers<'_>>().set_codecs(can_use_dnf());
    }
    spawn_hardware_detection(ui.clone());
}

fn add_simulated_hardware(hw: &mut libtaidan::steps::Hardware) {
    hw.nvidia |= simulated_hw("nvidia");
    hw.broadcom_wifi |= simulated_hw("broadcom-wifi");
    let has_blockable_drivers = hw.nvidia || hw.broadcom_wifi;
    hw.secure_boot_blocks |= simulated_hw("secure-boot") && has_blockable_drivers;
    tracing::debug!(?hw, "Simulated hardware (TAIDAN_SIMULATE_HW)");
}

fn spawn_hardware_detection(ui: slint::Weak<AppWindow>) {
    spawn_then(
        ui,
        || {
            let mut hw = crate::RT.block_on(libtaidan::steps::detect_hardware());
            tracing::debug!(?hw, "Detected hardware");
            if cfg!(debug_assertions) && std::env::var_os("TAIDAN_SIMULATE_HW").is_some() {
                add_simulated_hardware(&mut hw);
            }
            hw
        },
        |ui, hw| {
            let drivers = ui.global::<Drivers<'_>>();
            drivers.set_nvidia(hw.nvidia);
            drivers.set_broadcom_wifi(hw.broadcom_wifi);
            if hw.secure_boot_blocks {
                drivers.set_secure_boot_url(SECURE_BOOT_ENROLL_URL.into());
                if let Some(qr) = qr_code(SECURE_BOOT_ENROLL_URL) {
                    drivers.set_secure_boot_qr(qr);
                }
                drivers.set_secure_boot(true);
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_boot_qr_code() {
        let qr = qr_code(SECURE_BOOT_ENROLL_URL).unwrap();
        assert!(qr.size().width > 0);
    }

    #[test]
    fn stage_progress_never_goes_back() {
        assert!((furthest_stage_progress(-1.0, 0.25) - 0.25).abs() < f32::EPSILON);
        assert!((furthest_stage_progress(0.25, 0.5) - 0.5).abs() < f32::EPSILON);
        assert!((furthest_stage_progress(1.0, 0.1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn simulated_install_reports_every_stage() {
        let msgs = std::sync::Mutex::new(vec![]);
        let callback = |msg| msgs.lock().unwrap().push(msg);
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(simulate_install(&callback)).unwrap();
        let msgs = msgs.into_inner().unwrap();
        assert_eq!(msgs.len(), Stage::all().len() + 1);
        let stages = msgs
            .iter()
            .filter_map(|msg| match msg {
                InstallMsg::UpdStage(stage) => Some(*stage),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(stages, Stage::all());
        assert_eq!(stages.iter().filter(|stage| **stage == Stage::ApplySystemSettings).count(), 1);
        assert!(matches!(msgs.last(), Some(InstallMsg::Finish)));
    }
}
