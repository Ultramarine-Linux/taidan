use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceName;
impl super::Step for DeviceName {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::super::pkexec(
            "root",
            "hostnamectl",
            &["set-hostname", &settings.device_name, "--pretty"],
        )
        .await?;
        super::super::pkexec(
            "root",
            "hostnamectl",
            &["set-hostname", &settings.hostname, "--static"],
        )
        .await?;

        Ok(())
    }
}
