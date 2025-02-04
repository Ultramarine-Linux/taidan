#[derive(Clone, Copy, Debug, Default)]
pub struct SetTime;
impl super::Step for SetTime {
    #[tracing::instrument]
    async fn run(
        &self,
        _: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if let Err(e) = super::cmd(
            "systemctl",
            &["enable", "systemd-timesyncd.service", "--now"],
        ) {
            tracing::warn!(?e, "cannot enable systemd-timesyncd");
        }

        Ok(())
    }
}
