use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use smol::io::AsyncWriteExt;

use crate::SETTINGS;

pub static TWEAKS: LazyLock<Box<[Tweak]>> = LazyLock::new(|| {
    let tweaks = Tweak::list()
        .inspect_err(|err| tracing::error!(?err, "cannot list tweaks"))
        .unwrap_or_default();
    SETTINGS.write().tweaks = vec![false; tweaks.len()];
    tracing::info!("found {} tweaks", tweaks.len());
    tweaks
});

/// A distro-specified setting read by Taidan during runtime.
///
/// The tweaks must be stored in [`TWEAKS_DIR`] as folders, as so:
///
/// ```no_run
/// /usr/share/taidan/tweaks/ (TWEAKS_DIR)
/// ┣╸my_tweak/
/// ┃ ├╴tweak.toml (optional)
/// ┃ └╴up (required, must be executable)
/// ┗╸other_tweak/
///   ├╴tweak.toml
///   └╴up
/// ```
///
/// The ID of a tweak is determined by the directory name (`my_tweak`, `other_tweak`).
///
/// The `tweak.toml` file, if present, MUST specify the following two fields:
/// - `ftl_name` (alias `name`): the fluent ID for the name of the tweak
/// - `ftl_desc` (alias `desc`): the fluent ID for the description of the tweak
///
/// If the file is unpresent, the above two fields default to `<id>-name` and `<id>-desc`.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Tweak {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(alias = "name")]
    pub ftl_name: String,
    #[serde(alias = "desc")]
    pub ftl_desc: String,
}

#[cfg(debug_assertions)]
pub const TWEAKS_DIR: &str = "data/tweaks/";
// WARN: this is also hardcoded in install.sh
#[cfg(not(debug_assertions))]
pub const TWEAKS_DIR: &str = "/usr/share/taidan/tweaks/";

impl Tweak {
    /// Attempt to read and parse the `tweak.toml` file from the given directory.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if the file cannot be read, or a [`basic_toml::Error`] if the file cannot be parsed.
    pub fn try_from_cfg_path(dir: &Path) -> std::io::Result<Result<Self, basic_toml::Error>> {
        let f = std::fs::read(dir.join("tweak.toml"))?;
        Ok(basic_toml::from_slice(&f))
    }
    /// Obtain [`Tweak`] from the path to the tweak directory, assuming the entry is valid.
    ///
    /// # Panics
    ///
    /// Panics if `dir` ends in `..`, see [`Path::file_name()`].
    #[tracing::instrument]
    fn from_cfg_path(dir: &Path) -> Self {
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
                        tracing::error!(?err, "cannot parse tweak.toml, using default");
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
    /// Obtain [`Tweak`] from the path to the tweak directory.
    /// This is the "safe" version of [`Self::from_cfg_path()`].
    ///
    /// # Errors
    ///
    /// The constructor expects the directory to contain an executable file named `up`.
    /// For more information, see the struct docs.
    /// [`std::io::ErrorKind::NotFound`], [`std::io::ErrorKind::PermissionDenied`] may be returned.
    #[tracing::instrument]
    pub fn from_dir(path: PathBuf) -> std::io::Result<Self> {
        debug_assert!(path.is_dir());
        let Some(up) = std::fs::read_dir(&path)?.find_map(|f| {
            f.ok()
                .filter(|f| f.path().is_file() && f.file_name().as_encoded_bytes() == b"up")
        }) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("expected file `up` in `{}`", path.display()),
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

    /// Obtain a list of tweaks from the directory containing different tweaks.
    /// See the struct docs for more information.
    ///
    /// # Errors
    ///
    /// See [`Self::from_dir()`].
    #[tracing::instrument]
    pub fn list() -> std::io::Result<Box<[Self]>> {
        std::fs::read_dir(TWEAKS_DIR)?
            .filter_map(|dir_entry| {
                dir_entry
                    .inspect_err(|err| tracing::error!(?err, "cannot read file in {TWEAKS_DIR}"))
                    .ok()
                    .filter(|entry| entry.path().is_dir())
            })
            .map(|dir_entry| Self::from_dir(dir_entry.path()))
            .collect()
    }

    /// Return the tweak's ID, which is the directory name.
    ///
    /// # Panics
    ///
    /// Panics if the tweak's path does not have a file name.
    #[must_use]
    pub fn id(&self) -> &std::ffi::OsStr {
        self.path.file_name().unwrap()
    }

    pub fn name(&self) -> String {
        crate::t!(&self.ftl_name)
    }
    pub fn desc(&self) -> String {
        crate::t!(&self.ftl_desc)
    }

    /// Run `up` with the serialized `settings`
    ///
    /// # Panics
    ///
    /// Expects `pkexec` to be available.
    #[tracing::instrument]
    pub async fn run(&self, settings: &[u8], on: bool) {
        let mut cmd = smol::process::Command::new("pkexec")
            .args(["--user", "root"])
            .arg(self.path.join("up"))
            .arg(if on { "1" } else { "0" })
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("fail to run pkexec");
        let stdin = cmd.stdin.as_mut().unwrap();
        stdin.write_all(settings).await.unwrap();
        stdin.flush().await.unwrap();
        match cmd.wait().await {
            Ok(x) if x.success() => {}
            Ok(x) => tracing::error!(rc=?x.code(), "process failed with non-zero exit code"),
            Err(err) => tracing::error!(?err, "waiting for the `up` process failed"),
        }
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
