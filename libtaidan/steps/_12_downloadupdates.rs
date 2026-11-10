use crate::prelude::*;

/// downloads the system's updates as a dnf5 offline transaction and arms it, so the next restart
/// installs them, whether that's the one the drivers ask for or one the user picks later
///
/// Last, since dnf5 refuses a staged transaction once the rpmdb changed after it was downloaded,
/// and every step before it may install something. failing is never fatal, the system works fine
/// without the updates and gets them the usual way later.
#[derive(Clone, Copy, Debug, Default)]
pub struct DownloadUpdates;
impl super::Step for DownloadUpdates {
    #[tracing::instrument(skip(settings, callback))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &C,
    ) -> Res<()> {
        if cfg.taidan0.skip_dnf || super::is_ostree().await {
            return Ok(());
        }
        if settings.nointernet || !settings.download_updates {
            // one an earlier attempt staged would otherwise still install on the next restart
            if let Err(e) = crate::root(crate::Verb::DnfOfflineClean).await {
                tracing::debug!(?e, "cannot clean up offline updates, there were probably none");
            }
            return Ok(());
        }
        let downloaded =
            super::super::dnf::handle_dnf(callback, crate::Verb::DnfUpgradeOffline).await;
        if let Err(e) = downloaded {
            tracing::warn!(?e, "cannot download updates, leaving them for later");
            return Ok(());
        }
        // what `dnf5 offline reboot` does, minus the reboot, which is up to the user. it fails when
        // nothing was staged because the system is up to date already
        let armed = crate::root(crate::Verb::DnfOfflineArm).await;
        match armed {
            Ok(()) => callback.send(crate::InstallMsg::UpdatesReady),
            Err(e) => tracing::info!(?e, "no updates to install on restart"),
        }
        Ok(())
    }
}
