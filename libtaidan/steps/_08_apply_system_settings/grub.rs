use crate::{cfg::Desktop, prelude::*, theme::GrubTheme};

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let Some(dark) = settings.dark_mode else { return Ok(()) };
    if cfg.desktop() != Desktop::Plasma || !crate::steps::can_use_dnf(cfg).await {
        return Ok(());
    }
    let theme = GrubTheme::new(dark, settings.breeze_dark);
    if let Err(e) = crate::root(crate::Verb::SetGrubTheme { theme }).await {
        tracing::warn!(?e, ?theme, "cannot set the GRUB theme");
    }
    Ok(())
}
