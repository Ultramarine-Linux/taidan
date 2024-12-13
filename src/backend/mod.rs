use gettextrs::gettext;
use relm4::Sender;

use crate::pages::_11_installing::InstallingPageMsg;

pub mod dnf;
pub mod settings;

pub const NUM_STAGES: usize = 5;

#[derive(Debug, Clone, Default)]
pub enum Stage {
    #[default]
    UserAdd,
    SetTime,
    SetTheme,
    DnfUpdate,
    DnfInstall,
}

impl Stage {
    #[must_use]
    pub const fn is_dnf(&self) -> bool {
        matches!(self, Self::DnfUpdate | Self::DnfInstall)
    }
}

impl From<&Stage> for usize {
    fn from(value: &Stage) -> Self {
        match value {
            Stage::UserAdd => 0,
            Stage::SetTime => 1,
            Stage::SetTheme => 2,
            Stage::DnfUpdate => 3,
            Stage::DnfInstall => 4,
        }
    }
}

impl From<&Stage> for String {
    fn from(value: &Stage) -> Self {
        match value {
            Stage::UserAdd => gettext("Creating User…"),
            Stage::SetTime => gettext("Setting Timezone…"),
            Stage::SetTheme => gettext("Configuring Themes…"),
            Stage::DnfUpdate => gettext("Performing System Update…"),
            Stage::DnfInstall => gettext("Adding User Programs…"),
        }
    }
}

#[tracing::instrument]
pub async fn start_install(settings: settings::Settings, sender: Sender<InstallingPageMsg>) {
    tracing::info!("Starting installation");
}
