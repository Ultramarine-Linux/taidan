use secrecy::SecretString;

#[derive(Clone, Debug, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub langlocale: &'static str,

    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,

    pub device_name: String,
    pub hostname: String,

    pub fullname: String,
    pub username: String,

    #[serde(skip_serializing)]
    pub passwd: SecretString,

    pub install_codecs_drivers: bool,

    pub ims: Vec<&'static str>,

    pub theme_is_dark: bool,
    pub accent: Option<super::theme::AccentColor>,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: Vec<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            langlocale: Default::default(),
            skipconfig: Default::default(),
            nointernet: Default::default(),
            kb_layout: Default::default(),
            kb_variant: Default::default(),
            device_name: Default::default(),
            hostname: Default::default(),
            fullname: Default::default(),
            username: Default::default(),
            passwd: Default::default(),
            install_codecs_drivers: true,
            ims: Default::default(),
            theme_is_dark: Default::default(),
            accent: Default::default(),
            actions: Default::default(),
            tweaks: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_password() {
        let settings =
            Settings { passwd: "correct_horse_battery_staple".into(), ..Settings::default() };
        let rendered = format!("{settings:?}");
        assert!(!rendered.contains("correct_horse_battery_staple"), "{rendered}");
        assert!(rendered.contains("[REDACTED]"), "{rendered}");
    }

    #[test]
    fn serialized_output_omits_password() {
        let settings =
            Settings { passwd: "correct_horse_battery_staple".into(), ..Settings::default() };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.contains("correct_horse_battery_staple"), "{json}");
        assert!(!json.contains("passwd"), "{json}");
    }
}
