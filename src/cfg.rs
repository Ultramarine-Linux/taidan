use crate::backend::passwd::user_already_exists;

pub const ACTION_TYPES: usize = 5;

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(default)]
    pub edition: String,

    #[serde(default)]
    pub skip_pages: Vec<crate::Page>,
    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: crate::backend::i18n::I18nCfg,

    #[serde(default)]
    pub taidan0: Taidan0Config,

    #[serde(default = "_default_internet_retry_interval")]
    pub internet_retry_interval: u64,

    #[serde(default = "_default_internet_timeout")]
    pub internet_timeout: u64,
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
    ///
    /// # Panics
    /// If there are any missing information, the function will immediately panic.
    #[tracing::instrument]
    pub fn populate(&mut self) {
        self.taidan0.check();
        // distro
        let file = std::fs::read_to_string("/etc/os-release").expect("Cannot read /etc/os-release");
        let name = file
            .split('\n')
            .find_map(|line| line.strip_prefix("NAME="))
            .expect("Cannot find NAME=… in /etc/os-release");
        name.strip_prefix('"')
            .and_then(|name| name.strip_suffix('"'))
            .unwrap_or(name)
            .clone_into(&mut self.distro);

        if self.edition.is_empty() {
            let edition = file
                .split('\n')
                .find_map(|line| line.strip_prefix("VARIANT_ID="))
                .expect("Cannot find VARIANT_ID=… in /etc/os-release");
            edition
                .strip_prefix('"')
                .and_then(|name| name.strip_suffix('"'))
                .unwrap_or(edition)
                .clone_into(&mut self.edition);
        }

        tracing::trace!("Populated config: {self:#?}");
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
