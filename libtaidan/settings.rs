use secrecy::SecretString;

/// missing fields take their defaults, so settings saved by older versions still load
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub skipconfig: bool,
    pub nointernet: bool,

    pub kb_layout: String,
    pub kb_variant: Option<String>,

    pub device_name: String,
    pub hostname: String,

    /// IANA time zone like `America/Denver`, empty to keep the system's
    pub timezone: String,
    /// POSIX locale like `pt_BR.UTF-8`, empty to keep the system's
    pub locale: String,

    pub fullname: String,
    pub username: String,

    #[serde(skip_serializing)]
    pub passwd: SecretString,
    /// image to use as the user's icon
    pub avatar: Option<std::path::PathBuf>,

    pub install_codecs_drivers: bool,
    /// whether apps may ask GeoClue for the device's location
    pub location: bool,
    /// accessibility options, carried over to the new user
    pub a11y: crate::a11y::A11y,

    /// dark (`true`) or light mode for the new user, [`None`] to keep the distro's default
    pub dark_mode: Option<bool>,

    /// for networks that only reach the internet through a proxy
    pub proxy: Option<crate::proxy::Proxy>,

    pub ims: Vec<String>,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    /// ids of the tweaks to turn on, see [`crate::tweaks::Tweak::id()`], the rest are turned off
    #[serde(deserialize_with = "tweak_ids")]
    pub tweaks: Vec<String>,
}

/// tweak ids, or nothing for the old format: a `bool` per tweak in an order that wasn't stable,
/// so there's no telling which ones they were
fn tweak_ids<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<String>, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Tweaks {
        Ids(Vec<String>),
        Old(#[allow(dead_code)] Vec<bool>),
    }
    Ok(match serde::Deserialize::deserialize(de)? {
        Tweaks::Ids(ids) => ids,
        Tweaks::Old(_) => vec![],
    })
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
        assert!(!restored.location);
        assert_eq!(restored.a11y, crate::a11y::A11y::default());
        assert!(restored.tweaks.is_empty());
        assert!(restored.passwd.expose_secret().is_empty());
    }

    #[test]
    fn tweak_ids_deserialize() {
        let json = serde_json::json!({ "tweaks": ["rpmfusion", "flathub"] });
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.tweaks, ["rpmfusion", "flathub"]);
    }
}
