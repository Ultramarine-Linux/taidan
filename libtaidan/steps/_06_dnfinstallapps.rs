use super::_05_dnfdownloadapps::DNF_EXTRAS_ARGS;
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallApps;

/// swaps Fedora's `ffmpeg-free` for RPM Fusion's `ffmpeg`, as RPM Fusion says to before
/// installing `@multimedia` (<https://rpmfusion.org/Howto/Multimedia>). Otherwise their conflicts
/// make `--skip-broken` silently drop the codecs. `--allowerasing` on the extras transaction
/// wouldn't do: dnf5 makes `--skip-broken` installs weak solver jobs, and libsolv drops a weak job
/// before it tries erasing anything. Not fatal, e.g. without RPM Fusion there's no `ffmpeg`
async fn swap_ffmpeg(callback: &impl crate::Callback) {
    let installed = tokio::process::Command::new("rpm")
        .args(["-q", "--quiet", "ffmpeg-free"])
        .status()
        .await
        .is_ok_and(|s| s.success());
    if !installed {
        return;
    }
    let swap = super::super::dnf::handle_dnf(callback, |dnf| {
        dnf.args(["swap", "-y", "--allowerasing", "ffmpeg-free", "ffmpeg"])
    });
    if let Err(e) = swap.await {
        tracing::warn!(?e, "cannot swap ffmpeg-free for ffmpeg, codecs may be left out");
    }
}

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
        let codecs = super::_07_drivers_codecs::Codecs::codecs();
        if pkgs.iter().any(|pkg| codecs.contains(&pkg.as_str())) {
            swap_ffmpeg(sender).await;
        }
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
                super::super::dnf::handle_dnf(sender, |dnf| dnf.args(DNF_EXTRAS_ARGS).args(pkgs))
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
                        dnf.args(DNF_EXTRAS_ARGS).args(pkgs)
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
