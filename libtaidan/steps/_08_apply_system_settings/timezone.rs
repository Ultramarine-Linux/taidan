use crate::prelude::*;

pub(super) async fn run(settings: &crate::settings::Settings) -> Res<()> {
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
