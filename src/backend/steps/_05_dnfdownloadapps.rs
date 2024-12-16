#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadApps;
impl super::Step for DnfDownloadApps {
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        todo!()
    }
}
