use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetHostname;
impl super::Step for SetHostname {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // an empty name keeps whatever the system already has. A name that didn't take isn't
        // worth failing setup over, the system's is a fine place to end up
        if !settings.device_name.is_empty()
            && let Err(e) =
                crate::root("hostnamectl", ["hostname", "--pretty", "--", &settings.device_name])
                    .await
        {
            tracing::warn!(?e, name = settings.device_name, "cannot set the device name");
        }
        if !settings.hostname.is_empty()
            && let Err(e) =
                crate::root("hostnamectl", ["hostname", "--static", "--", &settings.hostname]).await
        {
            tracing::warn!(?e, hostname = settings.hostname, "cannot set the hostname");
        }
        Ok(())
    }
}
