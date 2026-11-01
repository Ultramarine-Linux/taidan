use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetLocale;
impl super::Step for SetLocale {
    /// after the apps, so the language pack, and with it the locale, is installed
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // an empty locale keeps whatever the system already has
        if settings.locale.is_empty() {
            return Ok(());
        }
        // systemd refuses locales that aren't installed, which can happen offline, the system's
        // language is a better place to end up than a failed setup
        let lang = format!("LANG={}", settings.locale);
        if let Err(e) = crate::root("localectl", ["set-locale", &lang]).await {
            tracing::warn!(?e, locale = settings.locale, "cannot set the system language");
        }
        Ok(())
    }
}
