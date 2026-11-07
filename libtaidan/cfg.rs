use crate::passwd::user_already_exists;

pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(default)]
    pub edition: String,

    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: crate::i18n::I18nCfg,

    #[serde(default)]
    pub taidan0: Taidan0Config,
}

fn _default_org() -> String {
    "Fyra Labs".into()
}

const fn _default_internet_retry_interval() -> u64 {
    5
}

const fn _default_internet_timeout() -> u64 {
    20
}

impl Config {
    /// Create a new [`Config`] by reading from `/etc/taidan.toml`, else [`Self::default()`].
    ///
    /// Remember to [`Self::populate`] afterwards.
    ///
    /// # Errors
    /// Failure to parse `/etc/taidan.toml`.
    pub fn new() -> Result<Self, basic_toml::Error> {
        let Ok(s) = std::fs::read_to_string("/etc/taidan.toml")
            .inspect_err(|err| tracing::warn!(?err, "cannot read /etc/taidan.toml"))
        else {
            return Ok(Self::default());
        };
        basic_toml::from_str(&s)
    }

    /// Populate the [`Config`] struct.
    #[tracing::instrument]
    pub fn populate(&mut self) {
        self.taidan0.check();
        self.read_os_release();
        tracing::trace!("Populated config: {self:#?}");
    }

    /// the distro name, and the edition unless the config sets it, from os-release
    pub fn read_os_release(&mut self) {
        let (name, variant_id) = os_release();
        self.distro = name;
        if self.edition.is_empty() {
            self.edition = variant_id;
        }
    }
}

/// `NAME=` and `VARIANT_ID=` from `/etc/os-release`, or `/usr/lib/os-release` as os-release(5) says
#[must_use]
pub fn os_release() -> (String, String) {
    let file = ["/etc/os-release", "/usr/lib/os-release"]
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| {
            tracing::warn!("cannot read os-release");
            String::new()
        });
    parse_os_release(&file)
}

/// `NAME=` and `VARIANT_ID=` from the contents of os-release(5)
///
/// missing keys get the defaults from os-release(5): `Linux` for `NAME=` and `""` for `VARIANT_ID=`
#[must_use]
pub fn parse_os_release(file: &str) -> (String, String) {
    let get = |key: &str| {
        let value =
            file.lines().find_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))?;
        let value = (value.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
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

/// the desktop an edition (`VARIANT_ID`) runs, lowercase, the same for its atomic image:
/// `atomic-gnome` is `gnome`
#[must_use]
pub fn desktop(edition: &str) -> String {
    let edition = edition.to_ascii_lowercase();
    edition.strip_prefix("atomic-").map(str::to_owned).unwrap_or(edition)
}

#[cfg(test)]
mod tests {
    use super::{desktop, parse_os_release};

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
        // PRETTY_NAME= and VARIANT= must not match NAME= or VARIANT_ID=
        assert_eq!(parse_os_release("PRETTY_NAME=x\nVARIANT=y\n"), ("Linux".into(), String::new()));
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
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

impl Taidan0Config {
    pub fn check(&self) {
        if self.continue_if_user_exists {
            tracing::info!("taidan0.continue_if_user_exists detected");
        } else if user_already_exists() {
            tracing::warn!("taidan0.continue_if_user_exists is false, exiting");
            std::process::exit(0);
        }
    }
}
