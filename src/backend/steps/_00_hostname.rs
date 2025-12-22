use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Hostname;
impl super::Step for Hostname {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::super::pkexec(
            "root",
            "hostnamectl",
            &["set-hostname", &settings.computername, "--pretty"],
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
