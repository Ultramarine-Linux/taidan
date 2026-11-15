use crate::prelude::*;

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
