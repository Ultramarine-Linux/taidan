use color_eyre::{Section, eyre::eyre};
use itertools::Itertools;
pub use taidan_catalogue_parser::{ACTION_TYPES, Category, Choice, ChoiceActions, ChoiceOption};

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub distro: String,
    #[serde(skip)]
    pub catalogue: Vec<Category>,
    #[serde(default)]
    pub edition: String,

    #[serde(default)]
    pub skip_pages: Vec<crate::Page>,
    #[serde(default = "_default_org")]
    pub org: String,

    #[serde(default)]
    pub i18n: crate::backend::i18n::I18nCfg,

    #[serde(default = "_default_internet_retry_interval")]
    pub internet_retry_interval: u64,

    #[serde(default = "_default_internet_timeout")]
    pub internet_timeout: u64,
}

fn _default_org() -> String {
    "Fyra Labs".into()
}

fn _default_internet_retry_interval() -> u64 {
    5
}

fn _default_internet_timeout() -> u64 {
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

        // catalogue
        self.populate_catalogue()
            .expect("cannot populate catalogue");

        // remove choices by filter editions
        self.catalogue.iter_mut().for_each(|cat| {
            cat.choices.retain(|choice| {
                choice
                    .editions
                    .as_ref()
                    .is_none_or(|editions| editions.contains(&self.edition))
            });
        });

        tracing::trace!("Populated config: {self:#?}");
    }

    /// # Errors
    /// - io errors on reading dir / file
    /// - failure on parsing yml files
    #[tracing::instrument]
    fn catalogue_from_path(dir: &std::path::Path) -> color_eyre::Result<Vec<Category>> {
        tracing::debug!(?dir, "Reading catalogue");
        std::fs::read_dir(dir)
            .map_err(|e| {
                eyre!("Cannot read catalogue dir")
                    .wrap_err(e)
                    .note(format!("Catalogue dir: {}", dir.display()))
            })?
            .map(|f| -> color_eyre::Result<_> { Ok(Category::parse_path(&f?.path())?) })
            .try_collect()
    }

    #[tracing::instrument]
    fn populate_catalogue(&mut self) -> color_eyre::Result<()> {
        if let Ok(p) = std::env::var("TAIDAN_CATALOGUE_DIR") {
            let p = std::path::PathBuf::from(p);
            if p.exists() && p.is_dir() {
                self.catalogue = Self::catalogue_from_path(&p)?;
                return Ok(());
            }
            tracing::error!(?p, "TAIDAN_CATALOGUE_DIR is set but no such directory");
        }
        let dir = std::path::Path::new(
            option_env!("TAIDAN_CATALOGUE_DIR")
                .unwrap_or(const_format::formatcp!("/etc/{}/catalogue/", crate::APPID)),
        );
        self.catalogue = Self::catalogue_from_path(dir)?;
        Ok(())
    }
}
