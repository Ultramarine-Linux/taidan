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

pub use _00_useradd::unlock as unlock_user;
pub(crate) use _07_drivers_codecs::secure_boot_enforced;
pub use _07_drivers_codecs::{Hardware, detect_hardware};
pub use _08_setup_imf::fix_homed as fix_homed_ims;

use crate::steps::{
    _00_useradd::UserAdd, _01_settheme::SetTheme, _04_systemsetup::SystemSetup,
    _05_dnfdownloadapps::DnfDownloadApps, _06_dnfinstallapps::DnfInstallApps,
    _07_drivers_codecs::DriversCodecs, _08_setup_imf::SetupImf, _09_sethostname::SetHostname,
    _10_settimezone::SetTimezone, _11_setlocale::SetLocale, _12_downloadupdates::DownloadUpdates,
};

/// whether this is an rpm-ostree system, which dnf can't install to
pub(crate) async fn is_ostree() -> bool {
    tokio::fs::try_exists("/ostree").await.unwrap_or_default()
}

/// whether dnf can install packages here, it can't with `skip_dnf` or on rpm-ostree
pub(crate) async fn can_use_dnf(cfg: &crate::cfg::Config) -> bool {
    !cfg.taidan0.skip_dnf && !is_ostree().await
}

/// runs `a` and `b` together and waits for both, the first error if either fails. Unlike
/// `try_join` it doesn't drop the other one when one fails, that would leave its root process
/// running with nobody reading its output
pub(crate) async fn join_both(
    a: impl std::future::Future<Output = Res<()>>,
    b: impl std::future::Future<Output = Res<()>>,
) -> Res<()> {
    let (a, b) = futures::future::join(a, b).await;
    a.and(b)
}

#[allow(async_fn_in_trait, clippy::unused_async)]
#[enum_dispatch::enum_dispatch(Stage)]
pub trait Step {
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()>;

    async fn pre<C: crate::Callback>(
        &self,
        _: &mut crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        Ok(())
    }
}

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Copy, Debug)]
pub enum Stage {
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
            Self::UserAdd(UserAdd),
            Self::SetHostname(SetHostname),
            Self::SetTimezone(SetTimezone),
            Self::SetTheme(SetTheme),
            Self::SystemSetup(SystemSetup),
            Self::DnfDownloadApps(DnfDownloadApps),
            Self::DnfInstallApps(DnfInstallApps),
            Self::DriversCodecs(DriversCodecs),
            Self::SetupImf(SetupImf),
            Self::SetLocale(SetLocale),
            Self::DownloadUpdates(DownloadUpdates),
        ]
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::UserAdd(UserAdd)
    }
}

impl From<Stage> for u8 {
    fn from(value: Stage) -> Self {
        match value {
            Stage::UserAdd(_) => 0,
            Stage::SetHostname(_) => 1,
            Stage::SetTimezone(_) => 2,
            Stage::SetTheme(_) => 3,
            Stage::SystemSetup(_) => 4,
            Stage::DnfDownloadApps(_) => 5,
            Stage::DnfInstallApps(_) => 6,
            Stage::DriversCodecs(_) => 7,
            Stage::SetupImf(_) => 8,
            Stage::SetLocale(_) => 9,
            Stage::DownloadUpdates(_) => 10,
        }
    }
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub fn cmd(name: &'static str, args: &[&str]) -> Res<()> {
    let p = std::process::Command::new(name)
        .args(args)
        .output()
        .map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        return Err(Err::program_fail(name, &p));
    }
    Ok(())
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn acmd(name: &'static str, args: &[&str]) -> Res<()> {
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

pub(crate) use super::root;
