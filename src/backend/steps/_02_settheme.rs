#[derive(Clone, Copy, Debug, Default)]
pub struct SetTheme;
impl super::Step for SetTheme {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        super::super::theme::set_theme(
            Some(&settings.username),
            settings.theme_is_dark,
            settings.accent,
        )
        .await?;
        super::super::theme::set_night_light(Some(&settings.username), settings.nightlight).await?;
        Ok(())
    }
}
