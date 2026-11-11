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
        if !settings.timezone.is_empty()
            && let Err(e) =
                crate::root(crate::Verb::SetTimezone { timezone: settings.timezone.clone() }).await
        {
            tracing::warn!(?e, timezone = settings.timezone, "cannot set the time zone");
        }
        if let Err(e) = crate::root(crate::Verb::EnableNtp).await {
            tracing::warn!(?e, "cannot turn on network time sync");
        }
        Ok(())
    }
}
