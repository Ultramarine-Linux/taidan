use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetTimezone;
impl super::Step for SetTimezone {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // an empty zone keeps whatever the system already has
        if !settings.timezone.is_empty() {
            crate::root("timedatectl", ["set-timezone", &settings.timezone]).await?;
        }
        // a wrong clock breaks TLS, but setup can go on without it
        if let Err(e) = crate::time::enable_ntp().await {
            tracing::warn!(?e, "cannot turn on network time sync");
        }
        Ok(())
    }
}
