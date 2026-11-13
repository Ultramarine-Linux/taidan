mod _00_useradd;
mod _01_settheme;
mod _04_systemsetup;
mod _05_dnfdownloadapps;
mod _06_dnfinstallapps;
mod _07_drivers_codecs;
mod _08_setup_imf;
mod _09_sethostname;
mod _10_settimezone;
mod _11_setlocale;
mod _12_downloadupdates;

use crate::prelude::*;

pub(crate) use _00_useradd::previous_uid;
pub use _00_useradd::unlock as unlock_user;
pub(crate) use _04_systemsetup::LANGPACK_PREFIXES;
pub(crate) use _05_dnfdownloadapps::{DNF_EXTRAS_ARGS, imf_packages};
pub(crate) use _07_drivers_codecs::secure_boot_enforced;
pub(crate) use _07_drivers_codecs::{Codecs, NVIDIA_PRIMARY_GPU_CONF, driver_packages};
pub use _07_drivers_codecs::{Hardware, detect_hardware};
pub use _08_setup_imf::fix_homed as fix_homed_ims;

pub(crate) async fn is_ostree() -> bool {
    tokio::fs::try_exists("/ostree").await.unwrap_or_default()
}

pub(crate) async fn can_use_dnf(cfg: &crate::cfg::Config) -> bool {
    !cfg.taidan0.skip_dnf && !is_ostree().await
}

async fn flatpaks_and_packages<C: crate::Callback>(
    callback: &C,
    refs: &[String],
    pkgs: &[String],
    download_only: bool,
) -> Res<()> {
    let flatpaks = async {
        if refs.is_empty() {
            return Ok(());
        }
        let verb = crate::Verb::FlatpakInstall { refs: refs.to_vec(), download_only };
        crate::dnf::run_logged("flatpak", &verb, |_| {}).await
    };
    let packages = async {
        if pkgs.is_empty() {
            return Ok(());
        }
        let verb = crate::Verb::DnfExtras { pkgs: pkgs.to_vec(), download_only };
        crate::dnf::handle_dnf(callback, verb).await
    };
    // flatpak and dnf don't affect each other, and neither is cancelled when the other fails
    let (flatpaks, packages) = futures::future::join(flatpaks, packages).await;
    flatpaks.and(packages)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    UserAdd,
    SetHostname,
    SetTimezone,
    SetTheme,
    SystemSetup,
    DnfDownloadApps,
    DnfInstallApps,
    DriversCodecs,
    SetupImf,
    SetLocale,
    DownloadUpdates,
}

impl Stage {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::UserAdd,
            Self::SetHostname,
            Self::SetTimezone,
            Self::SetTheme,
            Self::SystemSetup,
            Self::DnfDownloadApps,
            Self::DnfInstallApps,
            Self::DriversCodecs,
            Self::SetupImf,
            Self::SetLocale,
            Self::DownloadUpdates,
        ]
    }

    #[must_use]
    pub const fn l10n_id(self) -> &'static str {
        match self {
            Self::UserAdd => "steps-useradd",
            Self::SetHostname => "steps-sethostname",
            Self::SetTimezone => "steps-settimezone",
            Self::SetTheme => "steps-settheme",
            Self::SystemSetup => "steps-systemsetup",
            Self::DnfDownloadApps => "steps-dnfdownloadapps",
            Self::DnfInstallApps => "steps-dnfinstallapps",
            Self::DriversCodecs => "steps-driverscodecs",
            Self::SetupImf => "steps-setupimf",
            Self::SetLocale => "steps-setlocale",
            Self::DownloadUpdates => "steps-dnfdownloadupdate",
        }
    }

    #[tracing::instrument(skip(settings))]
    pub async fn pre(
        self,
        settings: &mut crate::settings::Settings,
        cfg: &crate::cfg::Config,
    ) -> Res<()> {
        match self {
            Self::SystemSetup => _04_systemsetup::pre(settings, cfg).await,
            Self::DnfDownloadApps => _05_dnfdownloadapps::pre(settings, cfg).await,
            _ => Ok(()),
        }
    }

    #[tracing::instrument(skip(settings, callback))]
    pub async fn run(
        self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &impl crate::Callback,
    ) -> Res<()> {
        match self {
            Self::UserAdd => _00_useradd::run(settings, cfg).await,
            Self::SetHostname => _09_sethostname::run(settings).await,
            Self::SetTimezone => _10_settimezone::run(settings).await,
            Self::SetTheme => _01_settheme::run(settings, cfg).await,
            Self::SystemSetup => _04_systemsetup::run(settings, cfg).await,
            Self::DnfDownloadApps => _05_dnfdownloadapps::run(settings, cfg, callback).await,
            Self::DnfInstallApps => _06_dnfinstallapps::run(settings, cfg, callback).await,
            Self::DriversCodecs => _07_drivers_codecs::run(settings, cfg, callback).await,
            Self::SetupImf => _08_setup_imf::run(settings, cfg).await,
            Self::SetLocale => _11_setlocale::run(settings, callback).await,
            Self::DownloadUpdates => _12_downloadupdates::run(settings, cfg, callback).await,
        }
    }
}

impl From<Stage> for u8 {
    fn from(stage: Stage) -> Self {
        stage as Self
    }
}

pub async fn acmd(
    name: &'static str,
    args: impl IntoIterator<Item: AsRef<std::ffi::OsStr>>,
) -> Res<()> {
    let p = tokio::process::Command::new(name)
        .args(args)
        .output()
        .await
        .map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        return Err(Err::program_fail(name, &p));
    }
    Ok(())
}

pub(crate) use super::{root, root_with_stdin};
