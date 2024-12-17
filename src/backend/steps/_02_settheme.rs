#[derive(Clone, Copy, Debug, Default)]
pub struct SetTheme;
impl super::Step for SetTheme {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        todo!()
    }
}
