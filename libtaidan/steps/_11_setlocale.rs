use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetLocale;
impl super::Step for SetLocale {
    /// after the apps, so the language pack, and with it the locale, is installed
    #[tracing::instrument(skip(settings, sender))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()> {
        set_keymap(settings).await;
        // an empty locale keeps whatever the system already has
        if settings.locale.is_empty() {
            return Ok(());
        }
        // systemd refuses locales that aren't installed, which can happen offline or on atomic
        // editions, the system's language is a better place to end up than a failed setup. The
        // done page says so
        if let Err(e) =
            crate::root(crate::Verb::SetLocale { locale: settings.locale.clone() }).await
        {
            tracing::warn!(?e, locale = settings.locale, "cannot set the system language");
            sender.send(crate::InstallMsg::LocaleFailed);
        }
        Ok(())
    }
}

/// the keyboard page already set the picked keymap for the system, but that's done in the
/// background and only logged when it fails, so the login screen could be left on the old one.
/// One that wasn't picked is the system's already
async fn set_keymap(settings: &crate::settings::Settings) {
    if !settings.kb_picked || settings.kb_layout.is_empty() {
        return;
    }
    let (layout, variant) = (&settings.kb_layout, settings.kb_variant.as_deref());
    if let Err(e) = crate::i18n::set_localectl_keymap(layout, variant).await {
        tracing::warn!(?e, layout, ?variant, "cannot set the system keymap");
    }
}
