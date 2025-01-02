#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadUpdate;
impl super::Step for DnfDownloadUpdate {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if settings.nointernet {
            return Ok(());
        }
        super::super::dnf::handle_dnf(sender, |dnf| dnf.args(["up", "-y", "--downloadonly"])).await
    }
}
