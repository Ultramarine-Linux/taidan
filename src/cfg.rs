use color_eyre::{eyre::eyre, Section};
use itertools::Itertools;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub distro: String,
    pub catalogue: Vec<Category>,
}

impl Config {
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

        // catalogue
        self.populate_catalogue()
            .expect("cannot populate catalogue");

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
                    .note(format!("Catalogue dir: {dir:?}"))
            })?
            .map(|f| -> color_eyre::Result<_> {
                Ok(serde_yaml::from_reader(std::fs::File::open(f?.path())?)?)
            })
            .map_ok(|mut category: Category| {
                category
                    .choices
                    .iter_mut()
                    .try_for_each(Choice::parse_after_yaml)
                    .map(|()| category)
            })
            .try_collect()?
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

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct Choice {
    pub name: String,
    pub provider: String,
    pub description: String,
    pub note: Option<String>,
    #[serde(default)]
    #[serde(rename = "options")]
    _options: Box<[serde_yaml::Value]>,
    #[serde(skip)]
    pub options: Box<[ChoiceOption]>,
    #[serde(rename = "actions")]
    _actions: serde_yaml::Value,
    #[serde(skip)]
    pub actions: ChoiceActions,
}

#[derive(Debug, Clone)]
pub enum ChoiceOption {
    Checkbox(String),
    Radio(Box<[String]>),
}
impl ChoiceOption {
    const fn as_dimension(&self) -> usize {
        match self {
            Self::Checkbox(_) => 2,
            Self::Radio(v) => v.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    EnableYumRepo(String),
    Rpm(String),
    Flatpak(String),
    Shell(String),
}

pub const ACTION_TYPES: usize = 4;

impl Action {
    #[must_use]
    pub const fn as_int(&self) -> usize {
        match self {
            Self::EnableYumRepo(_) => 0,
            Self::Rpm(_) => 1,
            Self::Flatpak(_) => 2,
            Self::Shell(_) => 3,
        }
    }
    #[must_use]
    pub fn as_inner_str(&self) -> &str {
        match self {
            Self::EnableYumRepo(s) | Self::Rpm(s) | Self::Flatpak(s) | Self::Shell(s) => s,
        }
    }
    #[must_use]
    pub fn consume_inner_str(self) -> String {
        match self {
            Self::EnableYumRepo(s) | Self::Rpm(s) | Self::Flatpak(s) | Self::Shell(s) => s,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ChoiceActions {
    Traverse(Box<[Self]>),
    List(Box<[Action]>),
    #[default]
    Todo,
    Action(Action),
}

impl ChoiceActions {
    #[must_use]
    pub fn get_action<'a>(
        &'a self,
        opts: &[usize],
    ) -> Option<Box<dyn Iterator<Item = &'a Action> + 'a>> {
        match opts.iter().try_fold(self, |this, &idx| {
            let Self::Traverse(inner) = this else {
                return None;
            };
            inner.get(idx)
        })? {
            Self::Todo | Self::Traverse(_) => None,
            Self::List(actions) => Some(Box::new(actions.iter())),
            Self::Action(action) => Some(Box::new(std::iter::once(action))),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Category {
    #[serde(rename = "category")]
    pub name: String,
    pub choices: Box<[Choice]>,
}

impl TryFrom<&str> for ChoiceActions {
    type Error = String;

    #[tracing::instrument]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "todo" {
            tracing::warn!("Found todo. This should not be propagated to production!");
            return Ok(Self::Todo);
        }
        if value.contains(';') {
            return Ok(Self::List(
                value
                    .split(';')
                    .filter_map(|s| s.split_once(':'))
                    .map(TryInto::try_into)
                    .try_collect()?,
            ));
        }
        let Some((id, val)) = value.split_once(':') else {
            tracing::warn!("Found action `{value}` (no type), treating as shell script");
            return Ok(Self::Action(Action::Shell(value.to_owned())));
        };
        (id, val).try_into().map(ChoiceActions::Action)
    }
}

impl TryFrom<(&str, &str)> for Action {
    type Error = String;

    fn try_from((k, v): (&str, &str)) -> Result<Self, Self::Error> {
        Ok(match k {
            "enable_yum_repo" => Self::EnableYumRepo(v.to_owned()),
            "rpm" => Self::Rpm(v.to_owned()),
            "flatpak" => Self::Flatpak(v.to_owned()),
            "shell" => Self::Shell(v.to_owned()),
            x => return Err(format!("Unknown action type `{x}` (value `{v}`)")),
        })
    }
}

const ONLY_ALLOW_OPT_KEY: &str = "Only one of `radio:`/`checkbox:` is allowed.";

#[allow(clippy::used_underscore_binding)]
impl Choice {
    /// # Errors
    /// - you did not input your yaml properly smh
    #[tracing::instrument]
    fn populate_options(&mut self) -> color_eyre::Result<()> {
        use serde_yaml::Value;
        self.options = self
            ._options
            .iter_mut()
            .map(|opt| {
                let Value::Mapping(map) = opt else {
                    return Err(eyre!("Expected yaml mapping, found {opt:?}"));
                };
                if map.len() != 1 {
                    return Err(eyre!("Unexpected {}-key element in `options:`", map.len())
                        .suggestion(ONLY_ALLOW_OPT_KEY));
                }
                let Some(first) = map.iter_mut().next() else {
                    return Err(eyre!("Unexpected 0-key element in `options:`")
                        .suggestion(ONLY_ALLOW_OPT_KEY)
                        .note("This should be unreachable, please report this bug."));
                };
                let (Value::String(key), choices) = first else {
                    let (k, v) = first;
                    return Err(eyre!("Unexpected key `{k:?}`, value `{v:?}` in `options:`")
                        .suggestion(ONLY_ALLOW_OPT_KEY)
                        .suggestion("Only sequences are accepted as values."));
                };
                Ok(match &**key {
                    "checkbox" => ChoiceOption::Checkbox({
                        let Value::String(s) = choices else {
                            return Err(eyre!(
                                "Expected string, found `{choices:?}` in `checkbox:`"
                            ));
                        };
                        std::mem::take(s)
                    }),
                    "radio" => ChoiceOption::Radio({
                        let Value::Sequence(choices) = choices else {
                            return Err(eyre!(
                                "Expected sequence, found `{choices:?}` in `radio:`"
                            ));
                        };
                        choices
                            .iter_mut()
                            .map(|s| {
                                if let Value::String(s) = s {
                                    Ok(std::mem::take(s))
                                } else {
                                    Err(eyre!(
                                        "Expected string, found `{s:?}` in `radio:` sequence"
                                    ))
                                }
                            })
                            .try_collect()?
                    }),
                    x => {
                        return Err(eyre!("Unexpected key `{x}:` in `options:`")
                            .suggestion(ONLY_ALLOW_OPT_KEY))
                    }
                })
            })
            .try_collect()?;
        Ok(())
    }

    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    #[tracing::instrument]
    fn recurse_yml_seq(
        mut val: serde_yaml::Value,
        mut counter: Vec<usize>,
        dimension: &[usize],
        depth: usize,
    ) -> color_eyre::Result<ChoiceActions> {
        if depth == counter.len() {
            // expect leaf or list
            let serde_yaml::Value::String(s) = val else {
                return Err(eyre!(
                    "Expected string at depth {depth} of `actions:`, found {val:?}"
                ));
            };
            return ChoiceActions::try_from(&*s)
                .map_err(|s| eyre!("Cannot parse action").wrap_err(s));
        }
        // get inner seq
        let Some(seq) = val.as_sequence_mut() else {
            if matches!(&val, serde_yaml::Value::String(s) if s == "todo") {
                // assume inner arrays with all todos
                return (0..dimension[depth])
                    .map(|i| {
                        Self::recurse_yml_seq(
                            val.clone(),
                            {
                                let mut counter = counter.clone();
                                counter[depth] = i;
                                counter
                            },
                            dimension,
                            depth + 1,
                        )
                    })
                    .try_collect()
                    .map(|v: Vec<ChoiceActions>| v.into_boxed_slice())
                    .map(ChoiceActions::Traverse);
            }
            return Err(eyre!(
                "Expected yaml sequence at `actions:` with dimension {dimension:?} (currently depth {depth}), found {val:?}"
            ));
        };

        if seq.len() != dimension[depth] {
            return Err(eyre!(
                "Expected at depth {depth} of `actions:` a sequence of {}, found {}",
                dimension[depth],
                seq.len()
            ));
        }

        let mut v = vec![];
        for element in seq {
            v.push(Self::recurse_yml_seq(
                std::mem::take(element),
                counter.clone(),
                dimension,
                depth + 1,
            )?);
            counter[depth] += 1;
        }

        Ok(ChoiceActions::Traverse(v.into_boxed_slice()))
    }

    #[allow(clippy::indexing_slicing)]
    #[tracing::instrument]
    fn populate_actions(&mut self) -> color_eyre::Result<()> {
        let dimension = self
            .options
            .iter()
            .map(ChoiceOption::as_dimension)
            .collect_vec();
        self.actions = Self::recurse_yml_seq(
            std::mem::take(&mut self._actions),
            vec![0; dimension.len()],
            &dimension,
            0,
        )?;
        Ok(())
    }

    fn mangle_description_and_note(&mut self) {
        self.description
            .replace('\n', " ")
            .trim_end()
            .clone_into(&mut self.description);
        if let Some(note) = self.note.as_mut() {
            note.replace('\n', " ").trim_end().clone_into(note);
        }
    }

    #[tracing::instrument]
    fn parse_after_yaml(&mut self) -> color_eyre::Result<()> {
        self.populate_options()?;
        self.populate_actions()?;
        self.mangle_description_and_note();
        Ok(())
    }
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_catalogue_browser() -> color_eyre::Result<()> {
        let mut obj: Category = serde_yaml::from_str(include_str!("../catalogue/browser.yml"))?;
        obj.choices
            .iter_mut()
            .try_for_each(super::Choice::parse_after_yaml)?;
        println!("{obj:#?}");
        Ok(())
    }
}
