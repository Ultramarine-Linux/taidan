use crate::prelude::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallApps;
impl super::Step for DnfInstallApps {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if settings.nointernet {
            return Ok(());
        }
        match (
            settings.actions[1].is_empty(),
            settings.actions[2].is_empty(),
        ) {
            (true, true) => {}
            (true, false) => {
                super::super::flatpak::handle_flatpak(sender.clone(), |flatpak| {
                    flatpak
                        .args(["install", "-y", "--noninteractive", "--no-pull"])
                        .args(&settings.actions[2])
                })
                .await?;
            }
            (false, true) => {
                super::super::dnf::handle_dnf(sender, |dnf| {
                    dnf.args(["in", "-y"]).args(&settings.actions[1])
                })
                .await?;
            }
            (false, false) => {
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
            }
        }

        for script in &settings.actions[3] {
            super::root("sh", &["-c", script])
                .await
                .wrap_err("script failed")
                .note(format!("script: {script:?}"))?;
        }

        Ok(())
    }
}
