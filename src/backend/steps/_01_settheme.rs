#[derive(Clone, Copy, Debug, Default)]
pub struct SetTheme;
impl super::Step for SetTheme {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if let Err(e) = super::super::theme::set_theme(
            Some(&settings.username),
            settings.theme_is_dark,
            settings.accent,
        )
        .await
        {
            tracing::warn!(?e, "cannot set theme");
        }
        if let Err(e) =
            super::super::theme::set_night_light(Some(&settings.username), settings.nightlight)
                .await
        {
            tracing::warn!(?e, "cannot set nightlight");
        }
        Ok(())
    }
}
