use secrecy::SecretString;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
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

    pub ims: Vec<String>,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: Vec<bool>,
}

impl Settings {
    pub fn new() -> Self {
        let mut ret = Self::default();
        ret.tweaks = vec![false; crate::tweaks::TWEAKS.len()];
        ret
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

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

    #[test]
    fn existing_serialized_settings_with_password_deserializes_correctly() {
        let settings =
            Settings { passwd: "correct_horse_battery_staple".into(), ..Settings::default() };
        let mut json = serde_json::to_value(&settings).unwrap();
        // Add `passwd`: match previous serialization behavior to test existing files
        json["passwd"] = serde_json::Value::String("correct_horse_battery_staple".into());
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.passwd.expose_secret(), "correct_horse_battery_staple");
    }
}
