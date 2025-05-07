use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Tweak {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(alias = "name")]
    pub ftl_name: String,
    #[serde(alias = "desc")]
    pub ftl_desc: String,
}

pub const TWEAKS_DIR: &str = "/usr/share/taidan/tweaks/";

impl Tweak {
    pub fn try_from_cfg_path(dir: &Path) -> std::io::Result<Result<Self, basic_toml::Error>> {
        let f = std::fs::read(dir.join("tweak.toml"))?;
        Ok(basic_toml::from_slice(&f))
    }
    #[tracing::instrument]
    pub fn from_cfg_path(dir: &Path) -> Self {
        Self::try_from_cfg_path(dir)
            .inspect_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    tracing::debug!(?err, "no tweak.toml, using default");
                } else {
                    tracing::warn!(?err, "cannot read tweak.toml, using default");
                }
            })
            .ok()
            .and_then(|inner_toml| {
                inner_toml
                    .inspect_err(|err| {
                        tracing::error!(?err, "cannot parse tweak.toml, using default")
                    })
                    .ok()
            })
            .unwrap_or_else(|| {
                let tweak_name = dir.file_name().unwrap().to_string_lossy();
                Self {
                    ftl_name: format!("{tweak_name}-name"),
                    ftl_desc: format!("{tweak_name}-desc"),
                    ..Self::default()
                }
            })
    }
    #[tracing::instrument]
    pub fn from_dir(path: PathBuf) -> std::io::Result<Self> {
        debug_assert!(path.is_dir());
        let Some(up) = std::fs::read_dir(&path)?.find_map(|f| {
            f.ok()
                .filter(|f| f.path().is_file() && f.file_name().as_encoded_bytes() == b"up")
        }) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("expected file `up` in `{path:?}`"),
            ));
        };
        if !is_executable(&up.path()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{up:?} is not executable"),
            ));
        }
        let cfg_self = Self::from_cfg_path(&path);
        Ok(Self { path, ..cfg_self })
    }

    #[tracing::instrument]
    pub fn list() -> std::io::Result<Vec<Self>> {
        std::fs::read_dir("/usr/share/taidan/tweaks/")?
            .filter_map(|dir_entry| {
                dir_entry
                    .inspect_err(|err| tracing::error!(?err, "cannot read file in {TWEAKS_DIR}"))
                    .ok()
                    .filter(|entry| entry.path().is_dir())
            })
            .map(|dir_entry| Ok(Self::from_dir(dir_entry.path())?))
            .collect()
    }

    pub fn id(&self) -> &std::ffi::OsStr {
        self.path.file_name().unwrap()
    }

    pub fn name(&self) -> String {
        crate::t!(&self.ftl_name)
    }
    pub fn desc(&self) -> String {
        crate::t!(&self.ftl_desc)
    }
}

#[tracing::instrument]
pub fn is_executable(f: &Path) -> bool {
    match f.metadata() {
        Ok(metadata) => metadata.permissions().mode() & 0o111 != 0,
        Err(err) => {
            tracing::error!(?err, "fail to obtain metadata");
            false
        }
    }
}
