use crate::prelude::*;

// RPM Fusion says to swap before installing `@multimedia`, otherwise `--skip-broken` silently
// drops the codecs (https://rpmfusion.org/Howto/Multimedia). `--allowerasing` doesn't help since
// libsolv drops weak `--skip-broken` jobs before it tries erasing anything
async fn swap_ffmpeg(callback: &impl crate::Callback) {
    let installed = tokio::process::Command::new("rpm")
        .args(["-q", "--quiet", "ffmpeg-free"])
        .status()
        .await
        .is_ok_and(|s| s.success());
    if !installed {
        return;
    }
    if let Err(e) = crate::dnf::handle_dnf(callback, crate::Verb::DnfSwapFfmpeg).await {
        tracing::warn!(?e, "cannot swap ffmpeg-free for ffmpeg, codecs may be left out");
    }
}

pub(super) async fn run(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    sender: &impl crate::Callback,
) -> Res<()> {
    if settings.nointernet {
        return Ok(());
    }
    let dnf = super::can_use_dnf(cfg).await;
    let pkgs: &[String] = if dnf { &settings.actions[1] } else { &[] };
    let codecs = super::_07_drivers_codecs::Codecs::codecs();
    if pkgs.iter().any(|pkg| codecs.contains(&pkg.as_str())) {
        swap_ffmpeg(sender).await;
    }
    super::flatpaks_and_packages(sender, &settings.actions[2], pkgs, false).await?;
    if dnf {
        for script in &settings.actions[3] {
            super::root(crate::Verb::RunScript { script: script.clone() })
                .await
                .wrap_message(|_| format!("script failed: {script}"))?;
        }
    }
    Ok(())
}
