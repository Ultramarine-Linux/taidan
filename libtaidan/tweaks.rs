use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use tokio::io::AsyncWriteExt;

pub static TWEAKS: LazyLock<Box<[Tweak]>> = LazyLock::new(|| {
    let tweaks = Tweak::list()
        .inspect_err(|err| tracing::error!(?err, "cannot list tweaks"))
        .unwrap_or_default()
        .into_iter()
        // hidden here rather than failing in `up` later on
        .filter(|t| {
            let supported = t.is_supported();
            if !supported {
                tracing::info!(id=?t.id(), level=?t.x86_64_level, "hiding tweak unsupported on this CPU");
            }
            supported
        })
        .collect::<Box<[_]>>();
    tracing::info!("found {} tweaks", tweaks.len());
    tweaks
});

/// A distro-specified setting read by Taidan during runtime.
///
/// The tweaks must be stored in [`TWEAKS_DIR`] as folders, as so:
///
/// ```text
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
/// The `tweak.toml` file is optional, and so are all of its fields:
/// - `ftl_name` (alias `name`): the fluent ID for the name of the tweak, defaults to `<id>-name`
/// - `ftl_desc` (alias `desc`): the fluent ID for the description of the tweak, defaults to `<id>-desc`
/// - `x86_64_level`: the minimum x86-64 microarchitecture level (1 to 4) the tweak needs.
///   Tweaks that set it are hidden on CPUs below that level and on other architectures.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tweak {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(alias = "name")]
    pub ftl_name: String,
    #[serde(alias = "desc")]
    pub ftl_desc: String,
    pub x86_64_level: Option<u8>,
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
        let mut tweak = Self::try_from_cfg_path(dir)
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
            .unwrap_or_default();
        let tweak_name = dir.file_name().unwrap().to_string_lossy();
        if tweak.ftl_name.is_empty() {
            tweak.ftl_name = format!("{tweak_name}-name");
        }
        if tweak.ftl_desc.is_empty() {
            tweak.ftl_desc = format!("{tweak_name}-desc");
        }
        tweak
    }

    /// Whether this machine meets the tweak's requirements, see [`Self::x86_64_level`].
    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.x86_64_level.is_none_or(|level| x86_64_level() >= level)
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
            f.ok().filter(|f| f.path().is_file() && f.file_name().as_encoded_bytes() == b"up")
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
        let mut tweaks = std::fs::read_dir(TWEAKS_DIR)?
            .filter_map(|dir_entry| {
                dir_entry
                    .inspect_err(|err| tracing::error!(?err, "cannot read file in {TWEAKS_DIR}"))
                    .ok()
                    .filter(|entry| entry.path().is_dir())
            })
            // a broken tweak is left out, rather than taking the others with it
            .filter_map(|dir_entry| {
                Self::from_dir(dir_entry.path())
                    .inspect_err(|err| tracing::error!(?err, "skipping invalid tweak"))
                    .ok()
            })
            .collect::<Vec<_>>();
        // the order directories are read in isn't stable
        tweaks.sort_by(|a, b| a.id().cmp(b.id()));
        Ok(tweaks.into_boxed_slice())
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

    /// Run `up` with the serialized `settings`, failures are logged
    ///
    /// Expects `pkexec` to be available.
    // not the settings, which have the proxy in them, and its password
    #[tracing::instrument(skip(settings))]
    pub async fn run(&self, settings: &[u8], on: bool) {
        // `pkexec` clears the environment, and tweaks may download things too
        let cmd = tokio::process::Command::new("pkexec")
            .args(["--user", "root", "env"])
            .args(crate::proxy::wrapper())
            .arg(self.path.join("up"))
            .arg(if on { "1" } else { "0" })
            .stdin(std::process::Stdio::piped())
            .spawn();
        let mut cmd = match cmd {
            Ok(cmd) => cmd,
            Err(err) => return tracing::error!(?err, "cannot run pkexec"),
        };
        if let Some(mut stdin) = cmd.stdin.take()
            && let Err(err) = stdin.write_all(settings).await
        {
            // `up` may not read the settings at all, and exit before they're written
            tracing::debug!(?err, "cannot pass the settings to `up`");
        }
        match cmd.wait().await {
            Ok(x) if x.success() => {}
            Ok(x) => tracing::error!(rc=?x.code(), "process failed with non-zero exit code"),
            Err(err) => tracing::error!(?err, "waiting for the `up` process failed"),
        }
    }
}

/// The x86-64 microarchitecture level of the running CPU, or 0 on other architectures.
///
/// See <https://gitlab.com/x86-psABIs/x86-64-ABI> for the features each level needs.
#[must_use]
pub fn x86_64_level() -> u8 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::is_x86_feature_detected as has;
        // LAHF/SAHF isn't detectable through std, every CPU with the rest has it anyway
        let v2 = has!("cmpxchg16b")
            && has!("popcnt")
            && has!("sse3")
            && has!("ssse3")
            && has!("sse4.1")
            && has!("sse4.2");
        // `avx` is only detected when the OS saves the AVX state, which covers OSXSAVE
        let v3 = v2
            && has!("avx")
            && has!("avx2")
            && has!("bmi1")
            && has!("bmi2")
            && has!("f16c")
            && has!("fma")
            && has!("lzcnt")
            && has!("movbe")
            && has!("xsave");
        let v4 = v3
            && has!("avx512f")
            && has!("avx512bw")
            && has!("avx512cd")
            && has!("avx512dq")
            && has!("avx512vl");
        1 + u8::from(v2) + u8::from(v3) + u8::from(v4)
    }
    #[cfg(not(target_arch = "x86_64"))]
    0
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
