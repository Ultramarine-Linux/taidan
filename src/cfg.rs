use color_eyre::{eyre::eyre, Section};
use itertools::Itertools;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub distro: String,
}

impl Config {
    /// Populate the [`Config`] struct.
    ///
    /// # Panics
    /// If there are any missing information, the function will immediately panic.
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

        tracing::debug!("Populated config: {self:#?}");
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Choice {
    pub name: String,
    pub provider: String,
    pub description: String,
    pub note: Option<String>,
    #[serde(default)]
    #[serde(rename = "options")]
    _options: Box<[serde_yaml::Value]>,
    #[serde(skip)]
    pub options: Option<Box<[ChoiceOption]>>,
    #[serde(rename = "actions")]
    _actions: serde_yaml::Value,
    #[serde(skip)]
    pub actions: Option<ChoiceActions>,
}

#[derive(Debug, Clone)]
pub enum ChoiceOption {
    Checkbox(Box<[String]>),
    Radio(Box<[String]>),
}
impl ChoiceOption {
    fn as_dimension(&self) -> usize {
        match self {
            ChoiceOption::Checkbox(v) => v.len() + 1,
            ChoiceOption::Radio(v) => v.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChoiceActions {
    Traverse(Box<[Self]>),
    List(Box<[Self]>),
    Todo,
    EnableYumRepo(String),
    Rpm(String),
    Flatpak(String),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Category {
    #[serde(rename = "category")]
    pub name: String,
    pub choices: Box<[Choice]>,
}

const ONLY_ALLOW_OPT_KEY: &str = "Only one of `radio:`/`checkbox:` is allowed.";

impl Choice {
    /// # Errors
    /// - you did not input your yaml properly smh
    fn populate_options(&mut self) -> color_eyre::Result<()> {
        self.options = Some(
            self._options
                .iter_mut()
                .map(|opt| {
                    let serde_yaml::Value::Mapping(map) = opt else {
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
                    let (serde_yaml::Value::String(key), serde_yaml::Value::Sequence(choices)) =
                        first
                    else {
                        let (k, v) = first;
                        return Err(eyre!("Unexpected key `{k:?}`, value `{v:?}` in `options:`")
                            .suggestion(ONLY_ALLOW_OPT_KEY)
                            .suggestion("Only sequences are accepted as values."));
                    };
                    let choices = choices
                        .iter_mut()
                        .filter_map(|v| {
                            if let serde_yaml::Value::String(s) = v {
                                Some(std::mem::take(s))
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(match &**key {
                        "checkbox" => ChoiceOption::Checkbox(choices),
                        "radio" => ChoiceOption::Radio(choices),
                        x => {
                            return Err(eyre!("Unexpected key `{x}:` in `options:`")
                                .suggestion(ONLY_ALLOW_OPT_KEY))
                        }
                    })
                })
                .try_collect()?,
        );
        Ok(())
    }

    fn populate_actions(&mut self) -> color_eyre::Result<()> {
        let Some(opts) = &self.options else {
            return Err(eyre!("BUG: BrowserChoice.options is not populated."));
        };
        let dimension = opts.iter().map(ChoiceOption::as_dimension).collect_vec();
        todo!();
        Ok(())
    }

    fn parse_after_yaml(&mut self) -> color_eyre::Result<()> {
        self.populate_options()?;
        Ok(())
    }
}
