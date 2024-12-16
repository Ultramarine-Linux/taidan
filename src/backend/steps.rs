mod _00_useradd;
mod _01_settime;
mod _02_settheme;
mod _03_dnfupdate;
mod _04_dnfinstall;

use gettextrs::gettext;

use crate::backend::steps::{
    _00_useradd::UserAdd, _01_settime::SetTime, _02_settheme::SetTheme, _03_dnfupdate::DnfUpdate,
    _04_dnfinstall::DnfInstall,
};

#[enum_dispatch::enum_dispatch(Stage)]
pub trait Step {
    fn run(&self, settings: &crate::backend::settings::Settings) -> color_eyre::Result<()>;
}

pub const NUM_STAGES: usize = 5;

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    UserAdd,
    SetTime,
    SetTheme,
    DnfUpdate,
    DnfInstall,
}

impl Stage {
    #[must_use]
    pub const fn is_dnf(&self) -> bool {
        matches!(self, Self::DnfUpdate(_) | Self::DnfInstall(_))
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
            Stage::DnfUpdate(_) => 3,
            Stage::DnfInstall(_) => 4,
        }
    }
}

impl From<Stage> for String {
    fn from(value: Stage) -> Self {
        match value {
            Stage::UserAdd(_) => gettext("Creating User…"),
            Stage::SetTime(_) => gettext("Setting Timezone…"),
            Stage::SetTheme(_) => gettext("Configuring Themes…"),
            Stage::DnfUpdate(_) => gettext("Performing System Update…"),
            Stage::DnfInstall(_) => gettext("Adding User Programs…"),
        }
    }
}
