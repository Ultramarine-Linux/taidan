use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Tweak {
    pub path: PathBuf,
}

pub const TWEAKS_DIR: &str = "/usr/share/taidan/tweaks/";

impl Tweak {
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
        Ok(Self { path })
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

    pub fn name(&self) -> &std::ffi::OsStr {
        self.path.file_name().unwrap()
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
