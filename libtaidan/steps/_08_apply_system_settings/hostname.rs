use crate::prelude::*;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hostname {}

impl crate::steps::InstallStep for Hostname {
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
    for (hostname, pretty) in [(&settings.device_name, true), (&settings.hostname, false)] {
        if !hostname.is_empty()
            && let Err(e) =
                crate::root(crate::Verb::SetHostname { hostname: hostname.clone(), pretty }).await
        {
            tracing::warn!(?e, hostname, pretty, "cannot set the hostname");
        }
    }
    Ok(())
}
