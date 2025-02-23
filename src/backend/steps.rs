mod _00_useradd;
mod _01_settheme;
mod _02_dnfdownloadupdate;
mod _03_dnfinstallupdate;
mod _04_script;
mod _05_dnfdownloadapps;
mod _06_dnfinstallapps;
mod _07_drivers_codecs;
mod _08_setup_imf;

use crate::prelude::*;

use crate::backend::steps::{
    _00_useradd::UserAdd, _01_settheme::SetTheme, _02_dnfdownloadupdate::DnfDownloadUpdate,
    _03_dnfinstallupdate::DnfInstallUpdate, _04_script::Script,
    _05_dnfdownloadapps::DnfDownloadApps, _06_dnfinstallapps::DnfInstallApps,
    _07_drivers_codecs::DriversCodecs, _08_setup_imf::SetupImf,
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

pub const NUM_STAGES: usize = 9;

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Copy, Debug)]
pub enum Stage {
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
        Self::UserAdd(UserAdd)
    }
}

impl From<Stage> for u8 {
    fn from(value: Stage) -> Self {
        match value {
            Stage::UserAdd(_) => 0,
            Stage::SetTheme(_) => 1,
            Stage::DnfDownloadUpdate(_) => 2,
            Stage::DnfInstallUpdate(_) => 3,
            Stage::Script(_) => 4,
            Stage::DnfDownloadApps(_) => 5,
            Stage::DnfInstallApps(_) => 6,
            Stage::DriversCodecs(_) => 7,
            Stage::SetupImf(_) => 8,
        }
    }
}

impl From<Stage> for String {
    fn from(value: Stage) -> Self {
        match value {
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
