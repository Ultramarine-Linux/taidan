mod _00_useradd;
mod _01_settime;
mod _02_settheme;
mod _03_dnfdownloadupdate;
mod _04_dnfinstallupdate;
mod _05_dnfdownloadapps;
mod _06_dnfinstallapps;
mod _07_drivers_codecs;

use crate::prelude::*;
use gettextrs::gettext;

use crate::backend::steps::{
    _00_useradd::UserAdd, _01_settime::SetTime, _02_settheme::SetTheme,
    _03_dnfdownloadupdate::DnfDownloadUpdate, _04_dnfinstallupdate::DnfInstallUpdate,
    _05_dnfdownloadapps::DnfDownloadApps, _06_dnfinstallapps::DnfInstallApps,
    _07_drivers_codecs::DriversCodecs,
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

pub const NUM_STAGES: usize = 8;

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    UserAdd,
    SetTime,
    SetTheme,
    DnfDownloadUpdate,
    DnfInstallUpdate,
    DnfDownloadApps,
    DnfInstallApps,
    DriversCodecs,
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
            Self::SetTime(SetTime),
            Self::SetTheme(SetTheme),
            Self::DnfDownloadUpdate(DnfDownloadUpdate),
            Self::DnfInstallUpdate(DnfInstallUpdate),
            Self::DnfDownloadApps(DnfDownloadApps),
            Self::DnfInstallApps(DnfInstallApps),
            Self::DriversCodecs(DriversCodecs),
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
            Stage::SetTime(_) => 1,
            Stage::SetTheme(_) => 2,
            Stage::DnfDownloadUpdate(_) => 3,
            Stage::DnfInstallUpdate(_) => 4,
            Stage::DnfDownloadApps(_) => 5,
            Stage::DnfInstallApps(_) => 6,
            Stage::DriversCodecs(_) => 7,
        }
    }
}

impl From<Stage> for String {
    fn from(value: Stage) -> Self {
        match value {
            Stage::UserAdd(_) => gettext("Creating User…"),
            Stage::SetTime(_) => gettext("Setting Timezone…"),
            Stage::SetTheme(_) => gettext("Configuring Themes…"),
            Stage::DnfDownloadUpdate(_) => gettext("Downloading System Update…"),
            Stage::DnfInstallUpdate(_) => gettext("Installing System Update…"),
            Stage::DnfDownloadApps(_) => gettext("Downloading User Programs…"),
            Stage::DnfInstallApps(_) => gettext("Installing User Programs…"),
            Stage::DriversCodecs(_) => gettext("Installing additional drivers…"),
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
