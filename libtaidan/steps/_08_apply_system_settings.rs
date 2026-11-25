mod grub;
mod hostname;
mod imf;
mod locale;
mod theme;
mod timezone;

use super::InstallStep;
use crate::prelude::*;
use enum_dispatch::enum_dispatch;

pub use grub::Grub;
pub use hostname::Hostname;
pub use imf::{Imf, ImfStart};
pub use locale::Locale;
pub use theme::Theme;
pub use timezone::Timezone;

pub(super) use imf::apply_to_skel as apply_imf_to_skel;
pub use imf::fix_homed;
pub(super) use theme::apply as apply_theme;

/// Sub-step options are owned by the operations that apply them.
#[enum_dispatch(InstallStep)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemSetting {
    Hostname,
    Timezone,
    Theme,
    Grub,
    Imf,
    Locale,
}

/// Apply quick settings after IMF packages have been installed, in a fixed order.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySystemSettings {
    #[serde(default, deserialize_with = "deserialize_settings")]
    pub apply_system_settings: Vec<SystemSetting>,
}

fn deserialize_settings<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Vec<SystemSetting>, D::Error> {
    use serde::de::Error as _;
    let settings = <Vec<SystemSetting> as serde::Deserialize>::deserialize(de)?;
    for (index, setting) in settings.iter().enumerate() {
        if settings[..index]
            .iter()
            .any(|other| std::mem::discriminant(other) == std::mem::discriminant(setting))
        {
            return Err(D::Error::custom("duplicate system setting configuration"));
        }
    }
    Ok(settings)
}

impl ApplySystemSettings {
    #[must_use]
    pub fn imf(&self) -> Imf {
        self.apply_system_settings
            .iter()
            .find_map(|setting| match setting {
                SystemSetting::Imf(imf) => Some(*imf),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn configured_settings(&self) -> Vec<SystemSetting> {
        [
            Hostname::default().into(),
            Timezone::default().into(),
            Theme::default().into(),
            Grub::default().into(),
            Imf::default().into(),
            Locale::default().into(),
        ]
        .into_iter()
        .map(|default| {
            self.apply_system_settings
                .iter()
                .find(|setting| {
                    std::mem::discriminant(*setting) == std::mem::discriminant(&default)
                })
                .cloned()
                .unwrap_or(default)
        })
        .collect()
    }
}

impl InstallStep for ApplySystemSettings {
    fn stage(&self) -> super::Stage {
        super::Stage::ApplySystemSettings
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &impl crate::Callback,
    ) -> Res<()> {
        for setting in self.configured_settings() {
            setting.run(settings, cfg, callback).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_steps_fill_defaults_and_keep_setting_order() {
        let settings: ApplySystemSettings = serde_json::from_value(serde_json::json!({
            "apply_system_settings": [
                { "locale": {} },
                { "imf": { "start": "xdg_autostart" } },
                { "hostname": {} }
            ]
        }))
        .unwrap();
        let configured = settings.configured_settings();
        assert_eq!(configured.len(), 6);
        assert!(matches!(configured[0], SystemSetting::Hostname(_)));
        assert!(matches!(configured[1], SystemSetting::Timezone(_)));
        assert!(matches!(configured[2], SystemSetting::Theme(_)));
        assert!(matches!(configured[3], SystemSetting::Grub(_)));
        let SystemSetting::Imf(imf) = configured[4] else {
            panic!("expected IMF sub-step");
        };
        assert_eq!(imf.start, Some(ImfStart::XdgAutostart));
        assert!(matches!(configured[5], SystemSetting::Locale(_)));
    }

    #[test]
    fn duplicate_sub_steps_are_rejected() {
        for setting in ["imf", "hostname", "timezone", "theme", "grub", "locale"] {
            let single = serde_json::json!({
                "apply_system_settings": [{ setting: {} }]
            });
            assert!(serde_json::from_value::<ApplySystemSettings>(single).is_ok());
            let input = serde_json::json!({
                "apply_system_settings": [
                    { setting: {} },
                    { setting: {} }
                ]
            });
            assert!(serde_json::from_value::<ApplySystemSettings>(input).is_err());
        }
    }

    #[tokio::test]
    async fn sub_step_enum_dispatches_async_methods() {
        let cfg = crate::cfg::Config::default();
        let mut settings = crate::settings::Settings::default();
        let module: SystemSetting = Locale::default().into();
        module.pre(&mut settings, &cfg).await.unwrap();
        module.run(&settings, &cfg, &|_| panic!("skipped locale sent a message")).await.unwrap();
    }

    #[tokio::test]
    async fn homed_theme_and_input_methods_are_left_to_the_skeleton() {
        let mut cfg = crate::cfg::Config::default();
        cfg.steps
            .push(crate::steps::UserAdd { systemd_homed: Some(true), ..Default::default() }.into());
        let settings = crate::settings::Settings {
            dark_mode: Some(true),
            ims: vec!["Pinyin".into()],
            ..Default::default()
        };
        // An empty username would fail Home::user if either tried writing the home here.
        theme::run(&settings, &cfg).await.unwrap();
        Imf::default().run(&settings, &cfg, &|_| {}).await.unwrap();
    }

    #[tokio::test]
    async fn skipped_language_and_keymap_emit_no_warning() {
        let settings = crate::settings::Settings::default();
        assert!(settings.locale.is_empty());
        assert!(!settings.kb_picked);
        locale::run(&settings, &|_| panic!("skipped language emitted a message")).await.unwrap();
    }
}
