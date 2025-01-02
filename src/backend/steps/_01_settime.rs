#[derive(Clone, Copy, Debug, Default)]
pub struct SetTime;
impl super::Step for SetTime {
    #[tracing::instrument]
    async fn run(
        &self,
        _: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::cmd(
            "systemctl",
            &["enable", "systemd-timesyncd.service", "--now"],
        )?;

        Ok(())
    }
}
