mod _00_hostname;
mod _01_useradd;
mod _02_settheme;
mod _03_dnfdownloadupdate;
mod _04_dnfinstallupdate;
mod _05_script;
mod _06_dnfdownloadapps;
mod _07_dnfinstallapps;
mod _08_drivers_codecs;
mod _09_setup_imf;

use crate::prelude::*;

use crate::backend::steps::{
    _00_hostname::Hostname, _01_useradd::UserAdd, _02_settheme::SetTheme,
    _03_dnfdownloadupdate::DnfDownloadUpdate, _04_dnfinstallupdate::DnfInstallUpdate,
    _05_script::Script, _06_dnfdownloadapps::DnfDownloadApps, _07_dnfinstallapps::DnfInstallApps,
    _08_drivers_codecs::DriversCodecs, _09_setup_imf::SetupImf,
};

#[allow(async_fn_in_trait, clippy::unused_async)]
#[enum_dispatch::enum_dispatch(Stage)]
pub trait Step {
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()>;

    async fn pre(
        &self,
        _: &mut crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        Ok(())
    }
}

pub const NUM_STAGES: usize = 10;

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Hostname,
    UserAdd,
    SetTheme,
    DnfDownloadUpdate,
    DnfInstallUpdate,
    Script,
    DnfDownloadApps,
    DnfInstallApps,
    DriversCodecs,
    SetupImf,
}

impl Stage {
    #[must_use]
    pub const fn is_dnf(&self) -> bool {
        matches!(
            self,
            Self::DnfDownloadUpdate(_)
                | Self::DnfInstallUpdate(_)
                | Self::DnfDownloadApps(_)
                | Self::DnfInstallApps(_)
        )
    }
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Hostname(Hostname),
            Self::UserAdd(UserAdd),
            Self::SetTheme(SetTheme),
            Self::DnfDownloadUpdate(DnfDownloadUpdate),
            Self::DnfInstallUpdate(DnfInstallUpdate),
            Self::Script(Script),
            Self::DnfDownloadApps(DnfDownloadApps),
            Self::DnfInstallApps(DnfInstallApps),
            Self::DriversCodecs(DriversCodecs),
            Self::SetupImf(SetupImf),
        ]
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::Hostname(Hostname)
    }
}

impl From<Stage> for u8 {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Hostname(_) => 0,
            Stage::UserAdd(_) => 1,
            Stage::SetTheme(_) => 2,
            Stage::DnfDownloadUpdate(_) => 3,
            Stage::DnfInstallUpdate(_) => 4,
            Stage::Script(_) => 5,
            Stage::DnfDownloadApps(_) => 6,
            Stage::DnfInstallApps(_) => 7,
            Stage::DriversCodecs(_) => 8,
            Stage::SetupImf(_) => 9,
        }
    }
}

impl From<Stage> for String {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Hostname(_) => t!("steps-hostname"),
            Stage::DnfDownloadUpdate(_) => t!("steps-dnfdownloadupdate"),
            Stage::DnfInstallUpdate(_) => t!("steps-dnfinstallupdate"),
            Stage::Script(_) => t!("steps-script"),
            Stage::DnfDownloadApps(_) => t!("steps-dnfdownloadapps"),
            Stage::DnfInstallApps(_) => t!("steps-dnfinstallapps"),
            Stage::DriversCodecs(_) => t!("steps-driverscodecs"),
            _ => t!("page-installing-loading"),
        }
    }
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub fn cmd(name: &str, args: &[&str]) -> color_eyre::Result<()> {
    let p = std::process::Command::new(name)
        .args(args)
        .status()
        .wrap_err(format!("fail to run `{name}`"))?;
    if !p.success() {
        return Err(eyre!("`{name}` failed").note(format!("Exit code: {:?}", p.code())));
    }
    Ok(())
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn acmd(name: &str, args: &[&str]) -> color_eyre::Result<()> {
    let p = tokio::process::Command::new(name)
        .args(args)
        .status()
        .await
        .wrap_err(format!("fail to run `{name}`"))?;
    if !p.success() {
        return Err(eyre!("`{name}` failed").note(format!("Exit code: {:?}", p.code())));
    }
    Ok(())
}

pub(crate) use super::root;
