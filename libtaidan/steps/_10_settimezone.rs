use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetTimezone;
impl super::Step for SetTimezone {
    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // an empty zone keeps whatever the system already has, as does one that didn't take,
        // which isn't worth failing setup over
        if !settings.timezone.is_empty()
            && let Err(e) =
                crate::root("timedatectl", ["set-timezone", "--", &settings.timezone]).await
        {
            tracing::warn!(?e, timezone = settings.timezone, "cannot set the time zone");
        }
        // a wrong clock breaks TLS, but setup can go on without it
        if let Err(e) = crate::time::enable_ntp().await {
            tracing::warn!(?e, "cannot turn on network time sync");
        }
        Ok(())
    }
}
