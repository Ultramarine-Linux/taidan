mod hostname;
mod imf;
mod locale;
mod theme;
mod timezone;

use crate::prelude::*;

pub(super) use imf::apply_to_skel as apply_imf_to_skel;
pub use imf::fix_homed;
pub(super) use theme::apply as apply_theme;

/// Apply the quick settings together, after the input method packages have been installed.
/// Each sub-stage keeps its own skipped-setting and best-effort error handling.
pub(super) async fn run(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    callback: &impl crate::Callback,
) -> Res<()> {
    hostname::run(settings).await?;
    timezone::run(settings).await?;
    theme::run(settings, cfg).await?;
    imf::run(settings, cfg).await?;
    locale::run(settings, callback).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn homed_theme_and_input_methods_are_left_to_the_skeleton() {
        let mut cfg = crate::cfg::Config::default();
        cfg.taidan0.systemd_homed = true;
        let settings = crate::settings::Settings {
            dark_mode: Some(true),
            ims: vec!["Pinyin".into()],
            ..Default::default()
        };
        // An empty username would fail Home::user if either tried writing the home here.
        theme::run(&settings, &cfg).await.unwrap();
        imf::run(&settings, &cfg).await.unwrap();
    }

    #[tokio::test]
    async fn skipped_language_and_keymap_emit_no_warning() {
        let settings = crate::settings::Settings::default();
        assert!(settings.locale.is_empty());
        assert!(!settings.kb_picked);
        locale::run(&settings, &|_| panic!("skipped language emitted a message")).await.unwrap();
    }
}
