use crate::{prelude::*, theme::GrubTheme};

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let Some(dark) = settings.dark_mode.filter(|_| !settings.skip_grub_theme) else {
        return Ok(());
    };
    if !crate::steps::can_use_dnf(cfg).await {
        return Ok(());
    }
    let theme = GrubTheme::new(dark, settings.breeze_dark);
    if let Err(e) = crate::root(crate::Verb::SetGrubTheme { theme }).await {
        tracing::warn!(?e, ?theme, "cannot set the GRUB theme");
    }
    Ok(())
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Grub {}

impl crate::steps::InstallStep for Grub {
    fn stage(&self) -> crate::steps::Stage {
        crate::steps::Stage::ApplySystemSettings
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _callback: &impl crate::Callback,
    ) -> Res<()> {
        run(settings, cfg).await
    }
}
