use gettextrs::gettext;
use relm4::Sender;

use crate::pages::_11_installing::InstallingPageMsg;

pub mod dnf;

pub const NUM_STAGES: usize = 5;

#[derive(Debug)]
pub enum Stage {
    UserAdd,
    SetTime,
    SetTheme,
    DnfUpdate,
    DnfInstall,
}

impl From<Stage> for String {
    fn from(value: Stage) -> Self {
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
pub async fn start_install(sender: Sender<InstallingPageMsg>) {
    tracing::info!("Starting installation");
}
