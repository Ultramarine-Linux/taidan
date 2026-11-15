use crate::prelude::*;

pub(super) async fn run(
    settings: &crate::settings::Settings,
    sender: &impl crate::Callback,
) -> Res<()> {
    apply_keymap_to_login_screen(settings).await;
    if settings.locale.is_empty() {
        return Ok(());
    }
    if let Err(e) = crate::root(crate::Verb::SetLocale { locale: settings.locale.clone() }).await {
        tracing::warn!(?e, locale = settings.locale, "cannot set the system language");
        sender.send(crate::InstallMsg::LocaleFailed);
    }
    Ok(())
}

async fn apply_keymap_to_login_screen(settings: &crate::settings::Settings) {
    if !settings.kb_picked || settings.kb_layout.is_empty() {
        return;
    }
    let (layout, variant) = (&settings.kb_layout, settings.kb_variant.as_deref());
    if let Err(e) = crate::i18n::set_localectl_keymap(layout, variant).await {
        tracing::warn!(?e, layout, ?variant, "cannot set the system keymap");
    }
}
