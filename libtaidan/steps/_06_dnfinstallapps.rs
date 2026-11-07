use crate::prelude::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallApps;
impl super::Step for DnfInstallApps {
    #[tracing::instrument(skip(settings, sender))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()> {
        if settings.nointernet {
            return Ok(());
        }
        // flatpaks still work without dnf, everything else here needs it
        let dnf = super::can_use_dnf(cfg).await;
        let pkgs: &[String] = if dnf { &settings.actions[1] } else { &[] };
        match (pkgs.is_empty(), settings.actions[2].is_empty()) {
            (true, true) => {}
            (true, false) => {
                super::super::flatpak::handle_flatpak(sender, |flatpak| {
                    flatpak
                        .args(["install", "-y", "--noninteractive", "--no-pull"])
                        .args(&settings.actions[2])
                })
                .await?;
            }
            (false, true) => {
                super::super::dnf::handle_dnf(sender, |dnf| {
                    dnf.args(["in", "-y", "--skip-broken"]).args(pkgs)
                })
                .await?;
            }
            (false, false) => {
                // run flatpak and dnf in parallel
                // this should be safe, supposedly they don't affect each other
                super::join_both(
                    super::super::flatpak::handle_flatpak(sender, |flatpak| {
                        flatpak
                            .args(["install", "-y", "--noninteractive", "--no-pull"])
                            .args(&settings.actions[2])
                    }),
                    super::super::dnf::handle_dnf(sender, |dnf| {
                        dnf.args(["in", "-y", "--skip-broken"]).args(pkgs)
                    }),
                )
                .await?;
            }
        }

        for script in settings.actions[3].iter().filter(|_| dnf) {
            super::root("sh", &["-c", script])
                .await
                .wrap_message(|_| format!("script failed: {script}"))?;
        }

        Ok(())
    }
}
