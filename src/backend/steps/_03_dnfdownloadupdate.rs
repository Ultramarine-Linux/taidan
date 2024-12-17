#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadUpdate;
impl super::Step for DnfDownloadUpdate {
    #[tracing::instrument]
    async fn run(
        &self,
        _: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::super::dnf::handle_dnf(sender, |dnf| dnf.args(["up", "-y", "--downloadonly"])).await
    }
}
