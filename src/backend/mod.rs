use gettextrs::gettext;
use relm4::Sender;

use crate::pages::_11_installing::InstallingPageMsg;

pub mod dnf;
pub mod settings;
pub mod steps;

#[tracing::instrument]
pub async fn start_install(settings: settings::Settings, sender: Sender<InstallingPageMsg>) {
    tracing::info!("Starting installation");
}
