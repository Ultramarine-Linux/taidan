use crate::prelude::*;
use tokio::process::Command;
#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallApps;
impl super::Step for DnfInstallApps {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        // run flatpak and dnf in parallel
        // this should be safe, supposedly they don't affect each other
        futures::future::try_join(
            super::super::flatpak::handle_flatpak(sender.clone(), |flatpak| {
                flatpak
                    .args(["install", "-y", "--noninteractive", "--no-pull"])
                    .args(&settings.actions[2])
            }),
            super::super::dnf::handle_dnf(sender, |dnf| {
                dnf.args(["in", "-y"]).args(&settings.actions[1])
            }),
        )
        .await?;

        for script in &settings.actions[3] {
            let status = (Command::new("sh").args(["-c", script]).status().await)
                .wrap_err("fail to run `sh`")?;
            if !status.success() {
                return Err(eyre!("script failed")
                    .note(format!("status: {status:?}"))
                    .note(format!("script: {script:?}")));
            }
        }

        Ok(())
    }
}
