#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallApps;
impl super::Step for DnfInstallApps {
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        todo!()
    }
}
