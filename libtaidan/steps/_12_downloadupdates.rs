use crate::prelude::*;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadUpdates {}

impl super::InstallStep for DownloadUpdates {
    fn stage(&self) -> super::Stage {
        super::Stage::DownloadUpdates
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &impl crate::Callback,
    ) -> Res<()> {
        run(settings, cfg, callback).await
    }
}

// must stay the last step, dnf5 refuses a staged transaction once the rpmdb changes
pub(super) async fn run(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    callback: &impl crate::Callback,
) -> Res<()> {
    if !super::can_use_dnf(cfg).await {
        return Ok(());
    }
    if settings.nointernet || !settings.download_updates {
        if let Err(e) = crate::root(crate::Verb::DnfOfflineClean).await {
            tracing::debug!(?e, "cannot clean up offline updates, there were probably none");
        }
        return Ok(());
    }
    if let Err(e) = crate::dnf::handle_dnf(callback, crate::Verb::DnfUpgradeOffline).await {
        tracing::warn!(?e, "cannot download updates, leaving them for later");
        return Ok(());
    }
    match crate::root(crate::Verb::DnfOfflineArm).await {
        Ok(()) => callback.send(crate::InstallMsg::UpdatesReady),
        Err(e) => tracing::info!(?e, "no updates to install on restart"),
    }
    Ok(())
}
