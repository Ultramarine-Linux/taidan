use relm4::Sender;
use steps::Step;

use crate::pages::_11_installing::InstallingPageMsg;

pub mod dnf;
pub mod flatpak;
pub mod settings;
pub mod steps;

use crate::prelude::REQWEST_CLIENT;

#[tracing::instrument]
pub async fn start_install(
    mut settings: settings::Settings,
    sender: Sender<InstallingPageMsg>,
) -> color_eyre::Result<()> {
    tracing::info!("Starting installation");
    for stage in steps::Stage::all() {
        tracing::debug!(?stage, "Running pre()");
        stage.pre(&mut settings, sender.clone()).await?;
        tracing::info!(?stage, "Running stage");
        sender
            .send(InstallingPageMsg::UpdStage(*stage))
            .expect("sender dropped?");
        stage.run(&settings, sender.clone()).await?;
    }
    sender
        .send(InstallingPageMsg::Finish)
        .expect("sender dropped?");
    Ok(())
}

#[tracing::instrument]
pub async fn start_simple_install(
    settings: settings::Settings,
    sender: Sender<InstallingPageMsg>,
) -> color_eyre::Result<()> {
    tracing::info!("Starting installation");
    tracing::info!("Running UserAdd");
    steps::Stage::UserAdd(Default::default())
        .run(&settings, sender.clone())
        .await?;
    tracing::info!("Running SetTime");
    steps::Stage::SetTime(Default::default())
        .run(&settings, sender.clone())
        .await?;
    sender
        .send(InstallingPageMsg::Finish)
        .expect("sender dropped?");
    Ok(())
}
