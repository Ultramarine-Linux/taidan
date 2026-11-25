pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize, deprecated)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(skip)]
    pub should_exit: bool,
    #[serde(default)]
    pub edition: String,

    /// The target GUI environment, independent of distribution edition metadata.
    #[serde(default)]
    pub gui_env: Option<String>,

    #[serde(default, deserialize_with = "crate::steps::deserialize_modules")]
    pub steps: Vec<crate::steps::Module>,

    #[serde(default, deserialize_with = "de_skip_pages")]
    pub skip_pages: Vec<SkipPage>,
    #[serde(default = "_default_org")]
    pub org: String,

    /// Compatibility fallback; configure [`crate::steps::Imf`] instead.
    ///
    /// ```compile_fail
    /// #![deny(deprecated)]
    /// let _ = libtaidan::cfg::Config::default().i18n;
    /// ```
    #[deprecated(note = "configure IMF selection on steps::Imf instead")]
    #[serde(default, deserialize_with = "de_legacy_i18n")]
    pub i18n: crate::i18n::I18nCfg,

    /// Compatibility fallback; configure [`crate::steps::UserAdd`] and
    /// [`crate::steps::SystemSetup`] instead.
    ///
    /// ```compile_fail
    /// #![deny(deprecated)]
    /// let _ = libtaidan::cfg::Config::default().taidan0;
    /// ```
    #[deprecated(note = "configure policy on steps::UserAdd and steps::SystemSetup instead")]
    #[serde(default, deserialize_with = "de_legacy_taidan0")]
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

fn de_legacy_i18n<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<crate::i18n::I18nCfg, D::Error> {
    let cfg = <crate::i18n::I18nCfg as serde::Deserialize>::deserialize(de)?;
    tracing::warn!(
        key = "i18n",
        "[i18n] is deprecated; configure imf on steps.apply_system_settings.imf instead"
    );
    Ok(cfg)
}

fn de_legacy_taidan0<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Taidan0Config, D::Error> {
    let cfg = <Taidan0Config as serde::Deserialize>::deserialize(de)?;
    tracing::warn!(
        key = "taidan0",
        "[taidan0] is deprecated; configure account policy on the user_add step and \
         DNF/offline policy on the system_setup step instead"
    );
    Ok(cfg)
}

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
    Analytics,
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
            "analytics" => Self::Analytics,
            "drivers" | "codecs" => Self::Drivers,
            "tweaks" => Self::Tweaks,
            "inputmethod" | "ims" => Self::InputMethod,
            "devicename" | "crashreport" => {
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

    #[must_use]
    pub fn desktop(&self) -> Desktop {
        match self.gui_env.as_deref() {
            Some(gui_env) => Desktop::from_gui_env(gui_env),
            None => Desktop::detect(&self.edition),
        }
    }

    #[must_use]
    pub fn imf_config(&self) -> crate::steps::Imf {
        self.steps
            .iter()
            .find_map(|step| match step {
                crate::steps::Module::ApplySystemSettings(settings) => Some(settings.imf()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn imf(&self) -> crate::i18n::I18nImf {
        self.imf_config().framework(self)
    }

    /// Non-fatal configuration issues suitable for a warning dialog.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.imf_config().configuration_warning(self).into_iter().collect()
    }

    #[must_use]
    pub fn user_add_config(&self) -> crate::steps::UserAdd {
        self.steps
            .iter()
            .find_map(|step| match step {
                crate::steps::Module::UserAdd(settings) => Some(settings.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn system_setup_config(&self) -> crate::steps::SystemSetup {
        self.steps
            .iter()
            .find_map(|step| match step {
                crate::steps::Module::SystemSetup(settings) => Some(settings.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn systemd_homed(&self) -> bool {
        self.user_add_config().systemd_homed(self)
    }

    #[must_use]
    pub fn continue_if_user_exists(&self) -> bool {
        self.user_add_config().continue_if_user_exists(self)
    }

    #[must_use]
    pub fn skip_dnf(&self) -> bool {
        self.system_setup_config().skip_dnf(self)
    }

    #[must_use]
    pub fn persist_no_internet_btn(&self) -> bool {
        self.system_setup_config().persist_no_internet_btn(self)
    }

    // The only production accesses to deprecated fields: compatibility fallback resolution.
    #[allow(deprecated)]
    pub(crate) fn legacy_imf(&self) -> Option<crate::i18n::I18nImf> {
        self.i18n.imf
    }

    #[allow(deprecated)]
    pub(crate) fn legacy_taidan0(&self) -> &Taidan0Config {
        &self.taidan0
    }

    pub fn read_os_release(&mut self) {
        let (name, variant_id) = os_release();
        self.distro = name;
        if self.edition.is_empty() {
            self.edition = variant_id;
        }
    }
}

pub(crate) fn read_os_release() -> String {
    ["/etc/os-release", "/usr/lib/os-release"]
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| {
            tracing::warn!("cannot read os-release");
            String::new()
        })
}

fn os_release() -> (String, String) {
    parse_os_release(&read_os_release())
}

pub(crate) fn os_release_field(file: &str, key: &str) -> Option<String> {
    let value = file.lines().find_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))?;
    Some(unquote(value).to_owned())
}

fn parse_os_release(file: &str) -> (String, String) {
    let get = |key: &str| os_release_field(file, key);
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

pub(crate) fn unquote(value: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| value.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Desktop {
    Gnome,
    Plasma,
    Budgie,
    Xfce,
    Other,
}

impl Desktop {
    #[must_use]
    pub fn from_gui_env(gui_env: &str) -> Self {
        let gui_env = gui_env.to_ascii_lowercase();
        match gui_env.strip_prefix("atomic-").unwrap_or(&gui_env) {
            "gnome" => Self::Gnome,
            "plasma" => Self::Plasma,
            "budgie" => Self::Budgie,
            "xfce" => Self::Xfce,
            _ => Self::Other,
        }
    }

    #[must_use]
    pub fn detect(edition: &str) -> Self {
        Self::from_edition(edition, std::path::Path::new("/usr/bin/plasmashell").exists())
    }

    #[must_use]
    pub fn from_edition(edition: &str, has_plasmashell: bool) -> Self {
        if has_plasmashell { Self::Plasma } else { Self::from_gui_env(edition) }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Desktop, IGNORED_LEGACY_INTERNET_KEYS, SkipPage, parse_os_release};

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
        assert_eq!(Desktop::from_gui_env("budgie"), Desktop::Budgie);
        assert_eq!(Desktop::from_gui_env("GNOME"), Desktop::Gnome);
        assert_eq!(Desktop::from_gui_env("atomic-GNOME"), Desktop::Gnome);
        assert_eq!(Desktop::from_gui_env("atomic-plasma"), Desktop::Plasma);
        assert_eq!(Desktop::from_gui_env("xfce"), Desktop::Xfce);
        assert_eq!(Desktop::from_gui_env("plasma"), Desktop::Plasma);
        assert_eq!(Desktop::from_gui_env("niri"), Desktop::Other);
        assert_eq!(Desktop::from_gui_env(""), Desktop::Other);
    }

    #[test]
    fn unset_gui_env_falls_back_to_edition() {
        assert_eq!(Desktop::from_edition("atomic-gnome", false), Desktop::Gnome);
        assert_eq!(Desktop::from_edition("budgie", false), Desktop::Budgie);
        assert_eq!(Desktop::from_edition("xfce", false), Desktop::Xfce);
        assert_eq!(Desktop::from_edition("", false), Desktop::Other);
        assert_eq!(Desktop::from_edition("gnome", true), Desktop::Plasma);
        let cfg = Config { edition: "atomic-plasma".into(), ..Default::default() };
        assert_eq!(cfg.desktop(), Desktop::detect("atomic-plasma"));
    }

    #[test]
    fn gui_env_not_edition_selects_desktop_and_imf_defaults() {
        use crate::i18n::I18nImf;
        use crate::steps::ImfStart;
        for (gui_env, desktop, imf, start) in [
            (Some("gnome"), Desktop::Gnome, I18nImf::IBus, ImfStart::XdgAutostart),
            (Some("plasma"), Desktop::Plasma, I18nImf::Fcitx5, ImfStart::Kwinrc),
            (Some("niri"), Desktop::Other, I18nImf::Fcitx5, ImfStart::XdgAutostart),
            (Some("noctalia"), Desktop::Other, I18nImf::Fcitx5, ImfStart::XdgAutostart),
        ] {
            let cfg = Config {
                edition: "atomic-gnome".into(),
                gui_env: gui_env.map(str::to_owned),
                ..Default::default()
            };
            assert_eq!(cfg.desktop(), desktop);
            assert_eq!(cfg.imf(), imf);
            assert_eq!(cfg.imf_config().start(cfg.desktop()), start);
        }
    }

    #[test]
    fn atomic_gui_env_normalizes_without_using_edition() {
        let cfg = Config {
            gui_env: Some("atomic-gnome".into()),
            edition: "atomic-plasma".into(),
            ..Default::default()
        };
        assert_eq!(cfg.desktop(), Desktop::Gnome);
        assert_eq!(cfg.imf(), crate::i18n::I18nImf::IBus);
    }

    #[test]
    fn stage_imf_options_override_legacy_options_and_defaults() {
        let cfg: Config = basic_toml::from_str(
            r#"
                gui_env = "plasma"
                [i18n]
                imf = "ibus"
                [[steps]]
                stage = "apply_system_settings"
                [[steps.apply_system_settings]]
                [steps.apply_system_settings.imf]
                imf = "fcitx5"
                start = "xdg_autostart"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.imf(), crate::i18n::I18nImf::Fcitx5);
        assert_eq!(cfg.imf_config().start(cfg.desktop()), crate::steps::ImfStart::XdgAutostart);
        assert!(!cfg.ui.contains_key("steps"));
        assert!(!cfg.ui.contains_key("gui_env"));
        let restored: Config = serde_json::from_value(serde_json::to_value(&cfg).unwrap()).unwrap();
        assert_eq!(restored.imf(), cfg.imf());
        assert_eq!(
            restored.imf_config().start(restored.desktop()),
            cfg.imf_config().start(cfg.desktop())
        );

        let legacy: Config = basic_toml::from_str("[i18n]\nimf = 'ibus'").unwrap();
        assert_eq!(legacy.imf(), crate::i18n::I18nImf::IBus);

        let cfg: Config = basic_toml::from_str(
            r#"
                gui_env = "niri"
                [[steps]]
                stage = "apply_system_settings"
                [[steps.apply_system_settings]]
                [steps.apply_system_settings.imf]
                imf = "ibus"
                start = "kwinrc"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.imf(), crate::i18n::I18nImf::IBus);
        assert_eq!(cfg.imf_config().start(cfg.desktop()), crate::steps::ImfStart::Kwinrc);
    }

    #[test]
    fn omitted_stage_options_use_defaults() {
        let cfg: Config = basic_toml::from_str(
            r#"
                gui_env = "plasma"
                [[steps]]
                stage = "apply_system_settings"
                [[steps.apply_system_settings]]
                [steps.apply_system_settings.imf]
            "#,
        )
        .unwrap();
        assert_eq!(cfg.imf(), crate::i18n::I18nImf::Fcitx5);
        assert_eq!(cfg.imf_config().start(cfg.desktop()), crate::steps::ImfStart::Kwinrc);
    }

    #[test]
    fn invalid_stage_configs_are_rejected() {
        for input in [
            "[[steps]]\nstage = 'user_add'\n[[steps]]\nstage = 'user_add'",
            "[[steps]]\nstage = 'apply_system_settings'\n\
             [[steps.apply_system_settings]]\n[steps.apply_system_settings.imf]\n\
             [[steps.apply_system_settings]]\n[steps.apply_system_settings.imf]",
            "[[steps]]\nstage = 'user_add'\n[[steps.apply_system_settings]]",
            "[[steps]]\nstage = 'unknown'",
            "[[steps]]\nstage = 'user_add'\nstart = 'kwinrc'",
            "[[steps]]\nstage = 'apply_system_settings'\n\
             [[steps.apply_system_settings]]\n[steps.apply_system_settings.imf]\nstart = 'bogus'",
            "[[steps]]\nstage = 'apply_system_settings'\n\
             [[steps.apply_system_settings]]\n[steps.apply_system_settings.imf]\nimf = 'bogus'",
        ] {
            assert!(basic_toml::from_str::<Config>(input).is_err(), "{input}");
        }
    }

    #[test]
    fn legacy_policy_is_used_when_step_options_are_omitted() {
        let cfg: Config = basic_toml::from_str(
            r#"
                [taidan0]
                continue_if_user_exists = true
                systemd_homed = true
                skip_dnf = true
                persist_no_internet_btn = true
                [[steps]]
                stage = "user_add"
                [[steps]]
                stage = "system_setup"
            "#,
        )
        .unwrap();
        assert!(cfg.continue_if_user_exists());
        assert!(cfg.systemd_homed());
        assert!(cfg.skip_dnf());
        assert!(cfg.persist_no_internet_btn());
    }

    #[test]
    fn explicit_false_step_policy_overrides_true_legacy_policy() {
        let cfg: Config = basic_toml::from_str(
            r#"
                [taidan0]
                continue_if_user_exists = true
                systemd_homed = true
                skip_dnf = true
                persist_no_internet_btn = true
                [[steps]]
                stage = "user_add"
                continue_if_user_exists = false
                systemd_homed = false
                [[steps]]
                stage = "system_setup"
                skip_dnf = false
                persist_no_internet_btn = false
            "#,
        )
        .unwrap();
        assert!(!cfg.continue_if_user_exists());
        assert!(!cfg.systemd_homed());
        assert!(!cfg.skip_dnf());
        assert!(!cfg.persist_no_internet_btn());
        let restored: Config = serde_json::from_value(serde_json::to_value(&cfg).unwrap()).unwrap();
        assert!(!restored.systemd_homed());
        assert!(!restored.skip_dnf());
    }

    #[test]
    fn step_policy_defaults_and_partial_legacy_fallbacks() {
        let defaults = Config::default();
        assert!(!defaults.continue_if_user_exists());
        assert!(!defaults.systemd_homed());
        assert!(!defaults.skip_dnf());
        assert!(!defaults.persist_no_internet_btn());

        let cfg: Config = basic_toml::from_str(
            r#"
                [taidan0]
                continue_if_user_exists = true
                persist_no_internet_btn = true
                [[steps]]
                stage = "user_add"
                systemd_homed = true
                [[steps]]
                stage = "system_setup"
                skip_dnf = true
            "#,
        )
        .unwrap();
        assert!(cfg.continue_if_user_exists());
        assert!(cfg.systemd_homed());
        assert!(cfg.skip_dnf());
        assert!(cfg.persist_no_internet_btn());
    }

    #[test]
    #[allow(deprecated, reason = "verify source compatibility for legacy field users")]
    fn legacy_rust_fields_remain_usable() {
        let mut cfg = Config::default();
        cfg.i18n.imf = Some(crate::i18n::I18nImf::IBus);
        cfg.taidan0.systemd_homed = true;
        assert_eq!(cfg.imf(), crate::i18n::I18nImf::IBus);
        assert!(cfg.systemd_homed());
    }

    #[test]
    fn only_explicit_legacy_tables_warn_during_deserialization() {
        // Tracing's process-wide callsite cache can race other tests without subscribers.
        // Run this capture assertion in its own test process instead of changing global logging.
        const ISOLATED: &str = "TAIDAN_TEST_ISOLATED_DEPRECATION_WARNINGS";
        if std::env::var_os(ISOLATED).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "cfg::tests::only_explicit_legacy_tables_warn_during_deserialization",
                ])
                .env(ISOLATED, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct Writer(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(bytes)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = Writer(output.clone());
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::WARN)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::callsite::rebuild_interest_cache();
            let _: Config =
                basic_toml::from_str("[[steps]]\nstage = 'user_add'\nsystemd_homed = true")
                    .unwrap();
            assert!(output.lock().unwrap().is_empty());
            // Explicit empty/default-valued legacy tables are deprecated too.
            let _: Config = basic_toml::from_str("[i18n]\n[taidan0]\nskip_dnf = false").unwrap();
        });
        let output = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(output.contains("[i18n] is deprecated"), "{output}");
        assert!(output.contains("[taidan0] is deprecated"), "{output}");
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
