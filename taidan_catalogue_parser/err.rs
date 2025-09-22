pub(crate) type Res<T = ()> = Result<T, CatalogueError>;
pub(crate) type E = CatalogueError;

#[derive(Debug, thiserror::Error)]
pub enum CatalogueError {
    #[error("YAML syntax error at {}: {err:?}", path.display())]
    Yml {
        err: serde_yaml_ng::Error,
        path: std::path::PathBuf,
    },
    #[error("Input/output error at {}: {err:?}", path.display())]
    Io {
        err: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("Syntax error in choice `{choice}` in category `{cat}`: {msg}")]
    Syntax {
        choice: String,
        cat: String,
        msg: String,
    },
}

impl CatalogueError {
    pub(crate) fn syntax<T>(choice: &str, cat: &str, msg: String) -> Result<T, Self> {
        Err(Self::Syntax {
            choice: choice.to_owned(),
            cat: cat.to_owned(),
            msg,
        })
    }
}
