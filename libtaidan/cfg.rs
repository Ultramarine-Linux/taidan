pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(skip)]
    pub should_exit: bool,
    #[serde(default)]
    pub edition: String,

    #[serde(default, deserialize_with = "de_skip_pages")]
    pub skip_pages: Vec<SkipPage>,
    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: crate::i18n::I18nCfg,

    #[serde(default)]
    pub taidan0: Taidan0Config,

    #[serde(flatten)]
    pub ui: serde_json::Map<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        basic_toml::from_str("").expect("an empty config parses")
    }
}

const IGNORED_LEGACY_INTERNET_KEYS: [&str; 2] = ["internet_timeout", "internet_retry_interval"];

fn _default_org() -> String {
    "Fyra Labs".into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipPage {
    Accessibility,
    Language,
    Keyboard,
    Welcome,
    Theme,
    Timezone,
    Network,
    Location,
    Drivers,
    Tweaks,
    InputMethod,
}

impl SkipPage {
    fn parse(name: &str) -> Result<Self, &'static str> {
        Ok(match name {
            "accessibility" => Self::Accessibility,
            "language" => Self::Language,
            "keyboard" => Self::Keyboard,
            "welcome" => Self::Welcome,
            "theme" => Self::Theme,
            "timezone" => Self::Timezone,
            "network" | "internet" => Self::Network,
            "location" => Self::Location,
            "drivers" | "codecs" => Self::Drivers,
            "tweaks" => Self::Tweaks,
            "inputmethod" | "ims" => Self::InputMethod,
            "devicename" | "analytics" | "crashreport" => {
                return Err("this page no longer exists");
            }
            "whoareyou" | "password" | "summary" | "installing" | "finish" | "done" | "error" => {
                return Err("this page can't be skipped");
            }
            _ => return Err("unknown page"),
        })
    }
}

fn de_skip_pages<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<SkipPage>, D::Error> {
    let names = <Vec<String> as serde::Deserialize>::deserialize(de)?;
    Ok(names
        .iter()
        .filter_map(|name| {
            SkipPage::parse(&name.to_lowercase())
                .inspect_err(|reason| tracing::warn!(name, reason, "ignoring skip_pages entry"))
                .ok()
        })
        .collect())
}

impl Config {
    /// Create a new [`Config`] by reading from `/etc/taidan.toml`, else [`Self::default()`].
    ///
    /// # Errors
    /// Failure to parse `/etc/taidan.toml`.
    pub fn new() -> Result<Self, basic_toml::Error> {
        let Ok(s) = std::fs::read_to_string("/etc/taidan.toml")
            .inspect_err(|err| tracing::warn!(?err, "cannot read /etc/taidan.toml"))
        else {
            return Ok(Self::default());
        };
        basic_toml::from_str(&s).inspect(Self::warn_ignored_keys)
    }

    fn warn_ignored_keys(&self) {
        for key in IGNORED_LEGACY_INTERNET_KEYS.into_iter().filter(|key| self.ui.contains_key(*key))
        {
            tracing::warn!(
                key,
                "ignoring /etc/taidan.toml key, online detection uses NetworkManager's connectivity \
                 check now"
            );
        }
    }

    pub fn read_os_release(&mut self) {
        let (name, variant_id) = os_release();
        self.distro = name;
        if self.edition.is_empty() {
            self.edition = variant_id;
        }
    }
}

fn os_release() -> (String, String) {
    let file = ["/etc/os-release", "/usr/lib/os-release"]
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| {
            tracing::warn!("cannot read os-release");
            String::new()
        });
    parse_os_release(&file)
}

fn parse_os_release(file: &str) -> (String, String) {
    let get = |key: &str| {
        let value =
            file.lines().find_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))?;
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        Some(value.to_owned())
    };
    let name = get("NAME").unwrap_or_else(|| {
        tracing::warn!("no NAME= in os-release, using Linux");
        "Linux".to_owned()
    });
    let variant_id = get("VARIANT_ID").unwrap_or_else(|| {
        tracing::warn!("no VARIANT_ID= in os-release");
        String::new()
    });
    (name, variant_id)
}

#[must_use]
pub fn desktop(edition: &str) -> String {
    let edition = edition.to_ascii_lowercase();
    edition.strip_prefix("atomic-").map(str::to_owned).unwrap_or(edition)
}

#[cfg(test)]
mod tests {
    use super::{Config, IGNORED_LEGACY_INTERNET_KEYS, SkipPage, desktop, parse_os_release};

    #[test]
    fn defaults_are_the_serde_ones() {
        assert_eq!(Config::default().org, "Fyra Labs");
    }

    #[test]
    fn skip_pages_accepts_old_names() {
        let cfg: Config = basic_toml::from_str(
            r#"skip_pages = ["codecs", "internet", "Theme", "inputmethod", "nope"]"#,
        )
        .unwrap();
        assert_eq!(
            cfg.skip_pages,
            [SkipPage::Drivers, SkipPage::Network, SkipPage::Theme, SkipPage::InputMethod]
        );
        assert!(!cfg.ui.contains_key("skip_pages"));
        assert!(!cfg.ui.contains_key("org"));
    }

    #[test]
    fn old_internet_keys_still_parse() {
        let cfg: Config =
            basic_toml::from_str("internet_timeout = 20\ninternet_retry_interval = 5").unwrap();
        assert_eq!(cfg.skip_pages, []);
        assert!(IGNORED_LEGACY_INTERNET_KEYS.iter().all(|key| cfg.ui.contains_key(*key)));
    }

    #[test]
    fn desktops() {
        assert_eq!(desktop("budgie"), "budgie");
        assert_eq!(desktop("atomic-GNOME"), "gnome");
        assert_eq!(desktop(""), "");
    }

    #[test]
    fn os_release_fields() {
        let file = "NAME=\"Ultramarine Linux\"\nPRETTY_NAME=\"x\"\nVARIANT_ID=budgie\n";
        assert_eq!(parse_os_release(file), ("Ultramarine Linux".into(), "budgie".into()));
        assert_eq!(parse_os_release("NAME='Arch Linux'\n"), ("Arch Linux".into(), String::new()));
    }

    #[test]
    fn os_release_defaults() {
        assert_eq!(parse_os_release(""), ("Linux".into(), String::new()));
        assert_eq!(parse_os_release("PRETTY_NAME=x\nVARIANT=y\n"), ("Linux".into(), String::new()));
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Taidan0Config {
    #[serde(default)]
    pub continue_if_user_exists: bool,
    #[serde(default)]
    pub skip_dnf: bool,
    #[serde(default)]
    pub persist_no_internet_btn: bool,
    #[serde(default)]
    pub systemd_homed: bool,
}
