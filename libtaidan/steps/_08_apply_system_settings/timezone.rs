use crate::prelude::*;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Timezone {}

impl crate::steps::InstallStep for Timezone {
    fn stage(&self) -> crate::steps::Stage {
        crate::steps::Stage::ApplySystemSettings
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        _cfg: &crate::cfg::Config,
        _callback: &impl crate::Callback,
    ) -> Res<()> {
        run(settings).await
    }
}

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
