use relm4::Sender;
use steps::Step;

use crate::pages::_11_installing::InstallingPageMsg;

pub mod dnf;
pub mod settings;
pub mod steps;

#[tracing::instrument]
pub async fn start_install(
    settings: settings::Settings,
    sender: Sender<InstallingPageMsg>,
) -> color_eyre::Result<()> {
    tracing::info!("Starting installation");
    for stage in steps::Stage::all() {
        sender.send(InstallingPageMsg::UpdStage(*stage));
        stage.run(&settings, sender.clone()).await?;
    }
    Ok(())
}
