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
    /// whether to download updates to install on the next restart, on unless turned off
    #[serde(default = "yes")]
    pub download_updates: bool,
    /// whether apps may ask GeoClue for the device's location, [`None`] to leave GeoClue as the
    /// distro has it, when the location page is skipped
    pub location: Option<bool>,
    /// accessibility options, carried over to the new user
    pub a11y: crate::a11y::A11y,

    /// dark (`true`) or light mode for the new user, [`None`] to keep the distro's default
    pub dark_mode: Option<bool>,
    /// accent color for the new user, only applied along with [`Self::dark_mode`] and on desktops
    /// that follow one, see [`crate::theme::accent_supported`]
    pub accent: Option<crate::theme::Accent>,

    /// for networks that only reach the internet through a proxy
    pub proxy: Option<crate::proxy::Proxy>,

    /// input methods to set up, filled from [`Self::locale`] when empty unless
    /// [`Self::ims_picked`]
    pub ims: Vec<String>,
    /// whether the user picked [`Self::ims`] themselves, so an empty list means none rather than
    /// the ones that go with the locale
    pub ims_picked: bool,
    pub imf: crate::i18n::I18nImf,

    pub actions: [Vec<String>; crate::cfg::ACTION_TYPES],

    /// the tweaks to turn on, see [`TweakIds`]
    pub tweaks: TweakIds,
}

const fn yes() -> bool {
    true
}

/// ids of the tweaks to turn on, see [`crate::tweaks::Tweak::id()`], the rest are turned off.
/// Unknown for settings in the old format, a `bool` per tweak in an order that wasn't stable, so
/// there's no telling which ones they were, then no tweak is run at all
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct TweakIds(Option<Vec<String>>);

impl TweakIds {
    /// the ids, [`None`] if they're unknown
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
            Old(#[allow(dead_code)] Vec<bool>),
            Unknown(()),
        }
        Ok(Self(match serde::Deserialize::deserialize(de)? {
            Tweaks::Ids(ids) => Some(ids),
            Tweaks::Old(_) | Tweaks::Unknown(()) => None,
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
        assert_eq!(restored.location, None);
        assert_eq!(restored.a11y, crate::a11y::A11y::default());
        assert_eq!(restored.tweaks.ids(), None);
        assert!(restored.passwd.expose_secret().is_empty());
    }

    #[test]
    fn tweak_ids_deserialize() {
        let json = serde_json::json!({ "tweaks": ["rpmfusion", "flathub"] });
        let restored: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(restored.tweaks.ids().unwrap(), ["rpmfusion", "flathub"]);
        let json = serde_json::to_value(&restored).unwrap();
        assert_eq!(json["tweaks"], serde_json::json!(["rpmfusion", "flathub"]));
        // ones that never picked any turn them all off
        let restored: Settings = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(restored.tweaks.ids(), Some(&[][..]));
        assert_eq!(restored.location, None);
        // from when it was a plain `bool`
        let restored: Settings =
            serde_json::from_value(serde_json::json!({ "location": false })).unwrap();
        assert_eq!(restored.location, Some(false));
    }
}
