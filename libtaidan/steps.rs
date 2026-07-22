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

use crate::steps::{
    _00_useradd::UserAdd, _01_settheme::SetTheme, _02_dnfdownloadupdate::DnfDownloadUpdate,
    _03_dnfinstallupdate::DnfInstallUpdate, _04_script::Script,
    _05_dnfdownloadapps::DnfDownloadApps, _06_dnfinstallapps::DnfInstallApps,
    _07_drivers_codecs::DriversCodecs, _08_setup_imf::SetupImf,
};

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

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub fn cmd(name: &'static str, args: &[&str]) -> Res<()> {
    let p = std::process::Command::new(name)
        .args(args)
        .output()
        .map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        return Err(Err::ProgramFail {
            name,
            output: String::from_utf8_lossy(&p.stdout).into_owned(),
            rc: p.status.code(),
        });
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
        return Err(Err::ProgramFail {
            name,
            output: String::from_utf8_lossy(&p.stdout).into_owned(),
            rc: p.status.code(),
        });
    }
    Ok(())
}

pub(crate) use super::root;
