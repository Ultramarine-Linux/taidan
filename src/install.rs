//! running the installation and leaving the OOBE
use libtaidan::{InstallMsg, steps::Stage};
use slint::ComponentHandle;

use crate::ui::{
    AppWindow, Drivers, Install, InstallState, Location, Network, Pages, Theme, ThemeMode,
};

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

/// the text of a panic's payload
fn panic_msg(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

/// runs the installation (or [`simulate_install`]) and returns the error message if it fails or
/// panics
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
            // debug rather than error, since errors go to Sentry and this may name the user
            tracing::debug!(?e, "installation error");
            e.to_string()
        })
    }));
    let msg = match res {
        Ok(Ok(())) => return None,
        Ok(Err(msg)) => msg,
        Err(payload) => format!("the installation crashed: {}", panic_msg(&*payload)),
    };
    // errors can name the user, e.g. ``user `alice` already exists``. `set_private_values` keeps
    // that out of Sentry
    tracing::error!(msg, "installation failed");
    Some(strip_ansi_escapes::strip_str(msg))
}

/// how to enroll a key for drivers under Secure Boot, see `Drivers.secure-boot`
const SECURE_BOOT_URL: &str = "https://fyra.to/secure-boot";

/// a QR code for `url`, [`None`] if it can't be made
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

/// whether `TAIDAN_SIMULATE_HW` pretends `name` is present, a comma-separated list of `nvidia`,
/// `broadcom-wifi`, `secure-boot` (that it blocks the drivers), or `all`
fn simulated_hw(name: &str) -> bool {
    std::env::var("TAIDAN_SIMULATE_HW")
        .is_ok_and(|sim| sim.split(',').any(|s| s.trim() == name || s.trim() == "all"))
}

/// pretends to install in a [`is_dev_session`]. set `TAIDAN_SIMULATE_ERROR` to fail partway
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
    // to see the done page with a driver that didn't install
    if std::env::var_os("TAIDAN_SIMULATE_DRIVER_ERROR").is_some() {
        callback.send(InstallMsg::DriversFailed);
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
            InstallMsg::StageProgress(frac) => {
                install.set_stage_progress(stage_progress(install.get_stage_progress(), frac));
            }
            InstallMsg::NeedsReboot => install.set_needs_reboot(true),
            InstallMsg::DriversFailed => install.set_drivers_failed(true),
            InstallMsg::LocaleFailed => install.set_locale_failed(true),
            InstallMsg::UpdatesReady => install.set_updates_ready(true),
            InstallMsg::Finish => {}
        }
    });
    if let Err(e) = set {
        tracing::error!(?e, "cannot update install progress");
    }
}

/// the progress bar after `frac` comes in, which never goes back within a stage. dnf counts
/// downloads, then the transaction, each from 0, and a stage may run it more than once.
/// [`InstallMsg::UpdStage`] resets it to -1, so it starts over with the next stage
#[expect(clippy::cast_possible_truncation, reason = "a fraction, for display")]
const fn stage_progress(current: f32, frac: f64) -> f32 {
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
    // not `needs_reboot`, since drivers an earlier attempt installed still need it
    install.set_updates_ready(false);
    install.set_drivers_failed(false);
    install.set_locale_failed(false);
    // don't show where a failed attempt stopped until the first stage starts
    install.set_stage_id("".into());
    install.set_stage_num(0);
    install.set_stage_progress(-1.0);
    install.set_stage_count(i32::try_from(Stage::all().len()).expect("too many stages"));
    crate::SETTINGS.write().install_codecs_drivers = app.global::<Drivers<'_>>().get_install();
    // a skipped location page leaves GeoClue as the distro has it
    crate::SETTINGS.write().location =
        (!crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Location))
            .then(|| app.global::<Location<'_>>().get_enabled());
    crate::SETTINGS.write().download_updates = install.get_download_updates();
    // here rather than when leaving the network page, since the summary can still change it
    crate::SETTINGS.write().proxy_persist = app.global::<Network<'_>>().get_proxy_persist();
    // what the theme page shows, or the distro's default if it's skipped
    crate::SETTINGS.write().dark_mode =
        (!crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Theme))
            .then(|| app.global::<Theme<'_>>().get_mode() == ThemeMode::Dark);
    // only with the mode, so a skipped theme page keeps the distro's accent too
    let accent = crate::SETTINGS.read().dark_mode.and_then(|_| crate::theme::picked(&app.global()));
    crate::SETTINGS.write().accent = accent;
    // like the theme, a skipped language page keeps the distro's system locale. the UI language
    // only comes from the system's then anyway, or is English if it's one we don't have
    if crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Language) {
        crate::SETTINGS.write().locale.clear();
    }
    // only "Set Up Offline" on the network page skips the online steps no matter what. otherwise
    // this goes by the connection right now, as the summary does, also when the network page is
    // skipped. leaving `nointernet` off on an offline machine makes the dnf steps fail.
    let net = app.global::<Network<'_>>();
    let online = net.get_install_online();
    let mut settings = crate::SETTINGS.read().clone();
    // before anything that names them can be logged
    crate::set_private_values(&settings);
    // the input methods the keyboard page and summary show, even the ones that only followed the
    // UI language, since the locale they came from may have just been cleared. only on this
    // attempt's copy, so they keep following the language if it changes before a retry
    settings.ims = crate::ims::enabled(&app.global());
    settings.ims_picked = true;
    // only on this attempt's copy, so a later retry gets the online steps back once the
    // connection is back
    settings.nointernet = !online;
    if !online {
        tracing::info!(chosen = net.get_offline_chosen(), "Installing without the online steps");
    }
    // the error page's way out when an online step keeps failing. skipping them all still leaves
    // a working system, only without updates and the extra software
    if install.get_force_offline() {
        tracing::warn!("Retrying without the online steps");
        settings.nointernet = true;
    }
    // the password is in `SETTINGS` now, so drop the plaintext copies in the UI instead of keeping
    // them around for the rest of the session. going back from the error page to its page means
    // typing it again. retrying from the summary keeps the saved one.
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
        if error.is_none() {
            *APPLIED_A11Y.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some((settings.username.clone(), settings.a11y));
        }
        let used_internet = !settings.nointernet;
        let set = ui.upgrade_in_event_loop(move |ui| {
            let install = ui.global::<Install<'_>>();
            // the page moves here rather than on a `changed Install.state` in Slint, which can
            // miss a retry that fails right away, as Failed → Running → Failed may coalesce
            if let Some(error) = error {
                install.set_used_internet(used_internet);
                install.set_error(error.into());
                install.set_state(InstallState::Failed);
                ui.set_pagenum(ui.global::<Pages<'_>>().get_error());
            } else {
                install.set_state(InstallState::Done);
                ui.set_pagenum(ui.global::<Pages<'_>>().get_done());
            }
        });
        if let Err(e) = set {
            tracing::error!(?e, "cannot report install result");
        }
    });
}

/// the user the installation made and the accessibility options it gave them, which may have
/// changed on the installing or done page since
static APPLIED_A11Y: std::sync::Mutex<Option<(String, libtaidan::a11y::A11y)>> =
    std::sync::Mutex::new(None);

/// gives the user the accessibility options as they are now, if they changed since the
/// installation carried them over
async fn reapply_a11y() {
    let applied = APPLIED_A11Y.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    let Some((username, applied)) = applied else { return };
    let a11y = crate::SETTINGS.read().a11y;
    if a11y == applied {
        return;
    }
    // the password opens a systemd-homed home, which isn't mounted anymore
    let passwd = crate::SETTINGS.read().passwd.clone();
    let settings = libtaidan::settings::Settings { username, a11y, passwd, ..Default::default() };
    if let Err(e) = libtaidan::a11y::reapply_to_user(&settings, &crate::CFG, applied).await {
        tracing::warn!(?e, "cannot carry over the accessibility settings");
    }
}

/// whether [`finish`] has started
static FINISHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// marks the system as configured so the OOBE doesn't run again, then reboots or exits
async fn finish(ui: &slint::Weak<AppWindow>, reboot: bool) {
    if is_dev_session() {
        tracing::warn!(reboot, "Dev session: not marking the system configured, nor rebooting");
    } else {
        reapply_a11y().await;
        // libtaidan already did this at the end of the installation, this only makes sure. it
        // succeeds when there's nothing left to do
        libtaidan::mark_configured().await;
        if reboot && !reboot_now().await {
            // the drivers may need it, so rather than quitting, the button can be pressed again
            FINISHING.store(false, std::sync::atomic::Ordering::SeqCst);
            _ = ui.upgrade_in_event_loop(|app| {
                let install = app.global::<Install<'_>>();
                install.set_reboot_failed(true);
                install.set_finishing(false);
            });
            return;
        }
    }
    // the process ends with the event loop, and these would outlive it
    crate::network::shutdown();
    crate::a11y::shutdown();
    if let Err(e) = slint::invoke_from_event_loop(|| _ = slint::quit_event_loop()) {
        tracing::error!(?e, "cannot quit");
    }
}

/// asks systemd to reboot, retrying since `pkexec` can fail for a moment after a VT switch.
/// returns whether it did
async fn reboot_now() -> bool {
    for attempt in 1..=3 {
        match libtaidan::root(libtaidan::Verb::Reboot).await {
            Ok(()) => return true,
            Err(e) => tracing::error!(?e, attempt, "cannot reboot"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    false
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
    install.set_updates_ready(false);
    install.set_reboot_failed(false);
    install.set_stage_id("".into());
    install.set_stage_num(0);
    install.set_stage_progress(-1.0);
}

/// whether dnf can install packages here, see `libtaidan::steps::can_use_dnf`
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
            // a failed installation may have left no user, so the OOBE must run again
            let state = ui.upgrade().map(|app| app.global::<Install<'_>>().get_state());
            if state != Some(InstallState::Done) {
                tracing::error!(?state, "not finishing, the installation hasn't succeeded");
                return;
            }
            // only once, since a second press on Done would run it all again
            if FINISHING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            if let Some(app) = ui.upgrade() {
                app.global::<Install<'_>>().set_finishing(true);
            }
            let ui = ui.clone();
            std::thread::spawn(move || crate::RT.block_on(finish(&ui, reboot)));
        }
    });
    if let Some(app) = ui.upgrade() {
        let drivers = app.global::<Drivers<'_>>();
        // a skipped drivers (`codecs`) page still installs them, as on the GTK version. hiding
        // it is how a distro installs them without asking. codecs come from dnf, see
        // `DnfDownloadApps`
        drivers.set_codecs(can_use_dnf());
    }

    // detect hardware for the drivers page in the background, as `lspci` can take a moment
    let ui = ui.clone();
    std::thread::spawn(move || {
        let mut hw = crate::RT.block_on(libtaidan::steps::detect_hardware());
        tracing::debug!(?hw, "Detected hardware");
        // pretend to have hardware, to see the drivers list. debug builds only, so a stray
        // variable can't list hardware that isn't there
        if cfg!(debug_assertions) && std::env::var_os("TAIDAN_SIMULATE_HW").is_some() {
            hw.nvidia |= simulated_hw("nvidia");
            hw.broadcom_wifi |= simulated_hw("broadcom-wifi");
            // only matters with drivers it blocks, as on real hardware
            hw.secure_boot_blocks |= simulated_hw("secure-boot") && (hw.nvidia || hw.broadcom_wifi);
            tracing::debug!(?hw, "Simulated hardware (TAIDAN_SIMULATE_HW)");
        }
        let set = ui.upgrade_in_event_loop(move |ui| {
            let drivers = ui.global::<Drivers<'_>>();
            drivers.set_nvidia(hw.nvidia);
            drivers.set_broadcom_wifi(hw.broadcom_wifi);
            if hw.secure_boot_blocks {
                drivers.set_secure_boot_url(SECURE_BOOT_URL.into());
                if let Some(qr) = qr_code(SECURE_BOOT_URL) {
                    drivers.set_secure_boot_qr(qr);
                }
                drivers.set_secure_boot(true);
            }
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
    fn secure_boot_qr_code() {
        let qr = qr_code(SECURE_BOOT_URL).unwrap();
        assert!(qr.size().width > 0);
    }

    #[test]
    fn stage_progress_never_goes_back() {
        // -1 until the stage reports any
        assert!((stage_progress(-1.0, 0.25) - 0.25).abs() < f32::EPSILON);
        assert!((stage_progress(0.25, 0.5) - 0.5).abs() < f32::EPSILON);
        // the transaction starting from 0 after the downloads
        assert!((stage_progress(1.0, 0.1) - 1.0).abs() < f32::EPSILON);
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
