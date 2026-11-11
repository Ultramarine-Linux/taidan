use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetHostname;
impl super::Step for SetHostname {
    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        for (hostname, pretty) in [(&settings.device_name, true), (&settings.hostname, false)] {
            if !hostname.is_empty()
                && let Err(e) =
                    crate::root(crate::Verb::SetHostname { hostname: hostname.clone(), pretty })
                        .await
            {
                tracing::warn!(?e, hostname, pretty, "cannot set the hostname");
            }
        }
        Ok(())
    }
}
