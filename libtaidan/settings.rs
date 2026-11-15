use secrecy::SecretString;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,
    pub kb_picked: bool,

    pub device_name: String,
    pub hostname: String,

    pub timezone: String,
    pub locale: String,

    pub fullname: String,
    pub username: String,

    #[serde(skip_serializing)]
    pub passwd: SecretString,
    pub avatar: Option<std::path::PathBuf>,

    pub install_codecs_drivers: bool,
    #[serde(default = "yes")]
    pub download_updates: bool,
    pub location: Option<bool>,
    pub a11y: crate::a11y::A11y,

    pub dark_mode: Option<bool>,
    pub accent: Option<crate::theme::Accent>,

    pub proxy: Option<crate::proxy::Proxy>,
    #[serde(default = "yes")]
    pub proxy_persist: bool,

    pub ims: Vec<String>,
    pub ims_picked: bool,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    pub tweaks: TweakIds,
}

const fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct TweakIds(Option<Vec<String>>);

impl TweakIds {
    #[must_use]
    pub fn ids(&self) -> Option<&[String]> {
        self.0.as_deref()
    }
}

impl Default for TweakIds {
    fn default() -> Self {
        Self(Some(vec![]))
    }
}

impl FromIterator<String> for TweakIds {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(Some(iter.into_iter().collect()))
    }
}

impl<'de> serde::Deserialize<'de> for TweakIds {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Tweaks {
            Ids(Vec<String>),
            OldBoolPerTweak(#[allow(dead_code)] Vec<bool>),
            Unknown(()),
        }
        Ok(Self(match serde::Deserialize::deserialize(de)? {
            Tweaks::Ids(ids) => Some(ids),
            Tweaks::OldBoolPerTweak(_) | Tweaks::Unknown(()) => None,
        }))
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
    fn password_deserializes() {
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json["passwd"] = serde_json::Value::String("correct_horse_battery_staple".into());
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.passwd.expose_secret(), "correct_horse_battery_staple");
    }

    #[test]
    fn older_settings_deserialize() {
        let json = serde_json::json!({
            "skipconfig": false,
            "nointernet": true,
            "kb_layout": "us",
            "kb_variant": null,
            "device_name": "",
            "hostname": "box",
            "fullname": "Alice Smith",
            "username": "alice",
            "install_codecs_drivers": true,
            "ims": ["Pinyin"],
            "actions": [[], [], [], [], []],
            "tweaks": [true, false],
        });
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.username, "alice");
        assert!(restored.nointernet);
        assert!(restored.timezone.is_empty());
        assert!(restored.locale.is_empty());
        assert_eq!(restored.location, None);
        assert_eq!(restored.a11y, crate::a11y::A11y::default());
        assert_eq!(restored.tweaks.ids(), None);
        assert!(restored.passwd.expose_secret().is_empty());
        assert!(!restored.kb_picked);
        assert!(restored.proxy_persist);
    }

    #[test]
    fn tweak_ids_deserialize() {
        let json = serde_json::json!({ "tweaks": ["rpmfusion", "flathub"] });
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.tweaks.ids().unwrap(), ["rpmfusion", "flathub"]);
        let json = serde_json::to_value(&restored).unwrap();
        assert_eq!(json["tweaks"], serde_json::json!(["rpmfusion", "flathub"]));
    }

    #[test]
    fn missing_tweaks_turn_all_off() {
        let restored: Settings = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(restored.tweaks.ids(), Some(&[][..]));
        assert_eq!(restored.location, None);
    }

    #[test]
    fn location_from_a_plain_bool_deserializes() {
        let restored: Settings =
            serde_json::from_value(serde_json::json!({ "location": false })).unwrap();
        assert_eq!(restored.location, Some(false));
    }
}
