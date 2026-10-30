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
        // an empty name keeps whatever the system already has
        if !settings.device_name.is_empty() {
            crate::root("hostnamectl", ["hostname", "--pretty", &settings.device_name]).await?;
        }
        if !settings.hostname.is_empty() {
            crate::root("hostnamectl", ["hostname", "--static", &settings.hostname]).await?;
        }
        Ok(())
    }
}
