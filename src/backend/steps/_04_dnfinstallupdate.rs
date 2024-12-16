#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallUpdate;
impl super::Step for DnfInstallUpdate {
    async fn run(
        &self,
        _: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::super::dnf::handle_dnf(sender, |dnf| _ = dnf.args(["up", "-y"])).await
    }
}
