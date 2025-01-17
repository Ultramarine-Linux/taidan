pub mod err;
pub(crate) use err::{Res, E};

use itertools::Itertools;
use std::collections::HashMap;

/// Denote a catalogue.
///
/// A catalogue contains many different app categories.
///
/// categoryid → [`Category`]
pub type Catalogue = HashMap<String, Category>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Category {
    #[serde(rename = "category")]
    pub name: String,
    // pub icon: String,
    pub choices: Vec<Choice>,
}

impl Category {
    pub fn parse_path(path: &std::path::Path) -> Res<Self> {
        let mut obj: Self =
            serde_yml::from_reader(std::fs::File::open(path).map_err(|err| E::Io {
                err,
                path: path.to_path_buf(),
            })?)
            .map_err(|err| E::Yml {
                err,
                path: path.to_path_buf(),
            })?;
        obj.choices.iter_mut().try_for_each(|choice| {
            choice.parse_after_yaml(
                path.file_name()
                    .expect("no filename")
                    .to_string_lossy()
                    .as_ref(),
            )
        })?;
        Ok(obj)
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
    _options: Box<[serde_yml::Value]>,
    #[serde(skip)]
    pub options: Box<[ChoiceOption]>,
    #[serde(rename = "actions")]
    _actions: serde_yml::Value,
    #[serde(skip)]
    pub actions: ChoiceActions,
    pub editions: Option<Vec<String>>,
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
    Copr(String),
}

pub const ACTION_TYPES: usize = 5;

impl Action {
    #[must_use]
    pub const fn as_int(&self) -> usize {
        match self {
            Self::EnableYumRepo(_) => 0,
            Self::Rpm(_) => 1,
            Self::Flatpak(_) => 2,
            Self::Shell(_) => 3,
            Self::Copr(_) => 4,
        }
    }
    #[must_use]
    pub fn as_inner_str(&self) -> &str {
        match self {
            Self::EnableYumRepo(s)
            | Self::Rpm(s)
            | Self::Flatpak(s)
            | Self::Shell(s)
            | Self::Copr(s) => s,
        }
    }
    #[must_use]
    pub fn consume_inner_str(self) -> String {
        match self {
            Self::EnableYumRepo(s)
            | Self::Rpm(s)
            | Self::Flatpak(s)
            | Self::Shell(s)
            | Self::Copr(s) => s,
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
            "copr" => Self::Copr(v.to_owned()),
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
    fn populate_options(&mut self, cat: &str) -> Res {
        use serde_yml::Value;
        self.options = self
            ._options
            .iter_mut()
            .map(|opt| {
                let Value::Mapping(map) = opt else {
                    return E::syntax(&self.name, cat, format!("Expected yaml mapping, found {opt:?}"));
                };
                let [first] = &mut map.iter_mut().collect_vec()[..] else {
                    return E::syntax(&self.name, cat, format!("Unexpected {}-key element in `options:`. {ONLY_ALLOW_OPT_KEY}", map.len()));
                };
                let (Value::String(key), choices) = first else {
                    let (k, v) = first;
                    return E::syntax(&self.name, cat, format!("Unexpected key `{k:?}`, value `{v:?}` in `options:`. {ONLY_ALLOW_OPT_KEY}. Only sequences are accepted as values."));
                };
                Ok(match &**key {
                    "checkbox" => ChoiceOption::Checkbox({
                        let Value::String(s) = choices else {
                            return E::syntax(&self.name, cat, format!("Expected string, found `{choices:?}` in `checkbox:`"));
                        };
                        std::mem::take(s)
                    }),
                    "radio" => ChoiceOption::Radio({
                        let Value::Sequence(choices) = choices else {
                            return E::syntax(&self.name, cat, format!("Expected sequence, found `{choices:?}` in `radio:`"));
                        };
                        choices
                            .iter_mut()
                            .map(|s| {
                                if let Value::String(s) = s {
                                    Ok(std::mem::take(s))
                                } else {
                                    E::syntax(&self.name, cat, format!("Expected string, found `{s:?}` in `radio:` sequence"))
                                }
                            })
                            .try_collect()?
                    }),
                    x => {
                        return E::syntax(&self.name, cat, format!("Unexpected key `{x}:` in `options:`. {ONLY_ALLOW_OPT_KEY}"));
                    }
                })
            })
            .try_collect()?;
        Ok(())
    }

    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    #[tracing::instrument]
    fn recurse_yml_seq(
        mut val: serde_yml::Value,
        mut counter: Vec<usize>,
        dimension: &[usize],
        depth: usize,
        // for error msgs
        choice: &str,
        cat: &str,
    ) -> Res<ChoiceActions> {
        if depth == counter.len() {
            // expect leaf or list
            let serde_yml::Value::String(s) = val else {
                return E::syntax(
                    choice,
                    cat,
                    format!("Expected string at depth {depth} of `actions:`, found {val:?}"),
                );
            };
            return ChoiceActions::try_from(&*s).map_err(|s| E::Syntax {
                choice: choice.to_string(),
                cat: cat.to_string(),
                msg: format!("Can't parse action: {s}"),
            });
        }
        // get inner seq
        let Some(seq) = val.as_sequence_mut() else {
            if matches!(&val, serde_yml::Value::String(s) if s == "todo") {
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
                            choice,
                            cat,
                        )
                    })
                    .try_collect()
                    .map(|v: Vec<ChoiceActions>| v.into_boxed_slice())
                    .map(ChoiceActions::Traverse);
            }
            return E::syntax(choice, cat, format!(
                "Expected yaml sequence at `actions:` with dimension {dimension:?} (currently depth {depth}), found {val:?}"
            ));
        };

        if seq.len() != dimension[depth] {
            return E::syntax(
                choice,
                cat,
                format!(
                    "Expected at depth {depth} of `actions:` a sequence of {}, found {}",
                    dimension[depth],
                    seq.len()
                ),
            );
        }

        let mut v = vec![];
        for element in seq {
            v.push(Self::recurse_yml_seq(
                std::mem::take(element),
                counter.clone(),
                dimension,
                depth + 1,
                choice,
                cat,
            )?);
            counter[depth] += 1;
        }

        Ok(ChoiceActions::Traverse(v.into_boxed_slice()))
    }

    #[allow(clippy::indexing_slicing)]
    #[tracing::instrument]
    fn populate_actions(&mut self, cat: &str) -> Res {
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
            &self.name,
            cat,
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
    fn parse_after_yaml(&mut self, cat: &str) -> Res {
        self.populate_options(cat)?;
        self.populate_actions(cat)?;
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
    fn test_parse_catalogue_browser() {
        let mut obj: Category =
            serde_yml::from_str(include_str!("../catalogue/browser.yml")).unwrap();
        obj.choices
            .iter_mut()
            .try_for_each(|choice| choice.parse_after_yaml("browser"))
            .unwrap();
        println!("{obj:#?}");
    }
}
