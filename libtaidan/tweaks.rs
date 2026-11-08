use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use tokio::io::AsyncWriteExt;

use crate::err::{Res, TaidanErrHelper};

/// every valid tweak this CPU supports. What to offer or run is [`available()`], which also
/// checks [`Tweak::needs_dnf`] and [`Tweak::secure_boot`]
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

/// the [`TWEAKS`] this system can take, given whether dnf can install packages here (see
/// `steps::can_use_dnf`) and whether Secure Boot is enforced
pub async fn available(can_use_dnf: bool) -> Vec<&'static Tweak> {
    let secure_boot = if TWEAKS.iter().any(|t| t.secure_boot.is_some()) {
        crate::steps::secure_boot_enforced().await.unwrap_or_else(|| {
            tracing::warn!("cannot tell whether Secure Boot is on, taking it to be");
            true
        })
    } else {
        false
    };
    TWEAKS
        .iter()
        .filter(|t| {
            let allowed = t.is_allowed(can_use_dnf, secure_boot);
            if !allowed {
                tracing::info!(id=?t.id(), can_use_dnf, secure_boot, "hiding tweak this system can't take");
            }
            allowed
        })
        .collect()
}

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
/// `up 1` turns the tweak on. `up 0` turns it back off, and only runs for a tweak an earlier,
/// unfinished attempt turned on (see [`ON_RECORD`]) that the user has since turned off.
///
/// The `tweak.toml` file is optional, and so are all of its fields:
/// - `ftl_name` (alias `name`): the fluent ID for the name of the tweak, defaults to `<id>-name`
/// - `ftl_desc` (alias `desc`): the fluent ID for the description of the tweak, defaults to `<id>-desc`
/// - `x86_64_level`: the minimum x86-64 microarchitecture level (1 to 4) the tweak needs.
///   Tweaks that set it are hidden on CPUs below that level and on other architectures.
/// - `needs_internet`: whether `up` downloads anything, e.g. packages. Such tweaks aren't run
///   when setting up without internet, off by default
/// - `needs_dnf`: whether `up` installs packages with dnf. Such tweaks are hidden on atomic
///   (rpm-ostree) systems and with `taidan0.skip_dnf`, off by default
/// - `secure_boot`: if set, the tweak is only offered when whether Secure Boot is enforced matches
///   it, e.g. `false` hides it while Secure Boot is on
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
    pub needs_internet: bool,
    pub needs_dnf: bool,
    pub secure_boot: Option<bool>,
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
    /// A missing `tweak.toml` gives the defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if `tweak.toml` exists but cannot be read or parsed. Falling back to the
    /// defaults there would drop requirements like [`Self::x86_64_level`], and offer the tweak on
    /// machines it doesn't support.
    ///
    /// # Panics
    ///
    /// Panics if `dir` ends in `..`, see [`Path::file_name()`].
    #[tracing::instrument]
    fn from_cfg_path(dir: &Path) -> std::io::Result<Self> {
        let mut tweak = match Self::try_from_cfg_path(dir) {
            Ok(Ok(tweak)) => tweak,
            Ok(Err(err)) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("cannot parse `{}`: {err}", dir.join("tweak.toml").display()),
                ));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(?err, "no tweak.toml, using default");
                Self::default()
            }
            Err(err) => return Err(err),
        };
        let tweak_name = dir.file_name().unwrap().to_string_lossy();
        if tweak.ftl_name.is_empty() {
            tweak.ftl_name = format!("{tweak_name}-name");
        }
        if tweak.ftl_desc.is_empty() {
            tweak.ftl_desc = format!("{tweak_name}-desc");
        }
        Ok(tweak)
    }

    /// Whether this machine meets the tweak's requirements, see [`Self::x86_64_level`].
    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.x86_64_level.is_none_or(|level| x86_64_level() >= level)
    }
    /// Whether the tweak can be offered, given whether dnf can install packages and whether Secure
    /// Boot is enforced, see [`Self::needs_dnf`] and [`Self::secure_boot`].
    #[must_use]
    pub fn is_allowed(&self, can_use_dnf: bool, secure_boot: bool) -> bool {
        (can_use_dnf || !self.needs_dnf) && self.secure_boot.is_none_or(|sb| sb == secure_boot)
    }
    /// Obtain [`Tweak`] from the path to the tweak directory.
    /// This is the "safe" version of [`Self::from_cfg_path()`].
    ///
    /// # Errors
    ///
    /// The constructor expects the directory to contain an executable file named `up`.
    /// For more information, see the struct docs.
    /// [`std::io::ErrorKind::NotFound`], [`std::io::ErrorKind::PermissionDenied`] may be returned.
    /// A `tweak.toml` that exists but cannot be read or parsed is also an error, see [`Self::from_cfg_path()`].
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
        let cfg_self = Self::from_cfg_path(&path)?;
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

    /// Run `up` with the serialized `settings`, as root through the helper, which finds it in
    /// `/usr/share/taidan/tweaks` by the id. Whether it succeeded, failures are logged
    // not the settings, which have the user's name and the proxy in them
    #[tracing::instrument(skip(settings))]
    pub async fn run(&self, settings: &[u8], on: bool) -> bool {
        let verb = crate::Verb::RunTweak { id: self.id().to_string_lossy().into_owned(), on };
        // `up`'s output goes to the log as it is, rather than into an error
        let cmd = crate::helper::command(&verb).stdin(std::process::Stdio::piped()).spawn();
        let mut cmd = match cmd {
            Ok(cmd) => cmd,
            Err(err) => {
                tracing::error!(?err, "cannot run pkexec");
                return false;
            }
        };
        if let Some(mut stdin) = cmd.stdin.take()
            && let Err(err) = stdin.write_all(&crate::helper::stdin(&verb, settings)).await
        {
            // `up` may not read the settings at all, and exit before they're written
            tracing::debug!(?err, "cannot pass the settings to `up`");
        }
        match cmd.wait().await {
            Ok(x) if x.success() => true,
            Ok(x) => {
                tracing::error!(rc=?x.code(), "process failed with non-zero exit code");
                false
            }
            Err(err) => {
                tracing::error!(?err, "waiting for the `up` process failed");
                false
            }
        }
    }
}

/// the settings `up` gets on stdin, with the proxy's username and password cut out. `up` already
/// has the whole proxy in `http_proxy` and friends for its own downloads, while what's on stdin is
/// what a tweak would write into a config file or log. A proxy whose credentials can't be cut
/// out is left out altogether
///
/// # Errors
/// The settings can't be serialized.
pub fn settings_json(settings: &crate::settings::Settings) -> serde_json::Result<Vec<u8>> {
    let mut settings = settings.clone();
    settings.proxy = settings.proxy.as_ref().and_then(crate::proxy::Proxy::without_credentials);
    serde_json::to_vec(&settings)
}

/// ids of the tweaks an unfinished install turned on, one per line, so a retry with one of them
/// turned off can turn it off again. One that was started but may not have finished, turning on
/// or off, has [`STARTED`] in front, a retry runs it again either way. Removed once the install
/// finishes, from then on they're the user's
pub const ON_RECORD: &str = "/var/lib/taidan/tweaks-on";

/// in front of a tweak in [`ON_RECORD`] that may be only partly turned on or off
pub const STARTED: char = '?';

/// what [`ON_RECORD`] says, empty if there's none
#[must_use]
pub fn recorded_on() -> Vec<String> {
    std::fs::read_to_string(ON_RECORD)
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// writes `ids` to [`ON_RECORD`], world-readable since the OOBE reads it as its own user, or
/// removes it when there are none
///
/// # Errors
/// Cannot write or remove the file.
pub async fn record_on(ids: &[String]) -> Res<()> {
    if ids.is_empty() {
        return clear_on_record().await;
    }
    crate::root(crate::Verb::RecordTweaks { ids: ids.to_vec() })
        .await
        .wrap_msg("cannot record the tweaks turned on")
}

/// what [`record_on`] does, run by the helper as root. Replaced whole, so a full disk can't
/// leave it empty, and on disk before what it records is done, a power cut mustn't lose it
pub(crate) fn write_on_record(ids: &[String]) -> std::io::Result<()> {
    let content: String = ids.iter().map(|id| format!("{id}\n")).collect();
    crate::helper::write_atomic(Path::new(ON_RECORD), content.as_bytes(), 0o644)
}

/// removes [`ON_RECORD`], if it's there
///
/// # Errors
/// Cannot remove the file.
pub async fn clear_on_record() -> Res<()> {
    if !tokio::fs::try_exists(ON_RECORD).await.unwrap_or(true) {
        return Ok(());
    }
    crate::root(crate::Verb::ClearTweakRecord).await.wrap_msg("cannot remove the tweaks record")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_json_hides_proxy_credentials() {
        let settings = crate::settings::Settings {
            proxy: crate::proxy::Proxy::new("http://bob:hunter2@proxy.example.com:3128", "lan"),
            proxy_persist: false,
            ..Default::default()
        };
        let json: serde_json::Value =
            serde_json::from_slice(&settings_json(&settings).unwrap()).unwrap();
        assert_eq!(json["proxy"]["url"], "http://proxy.example.com:3128");
        assert_eq!(json["proxy"]["no_proxy"], "lan");
        assert_eq!(json["proxy_persist"], false);
    }

    fn tweak_dir(toml: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let up = dir.path().join("up");
        std::fs::write(&up, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&up, std::fs::Permissions::from_mode(0o755)).unwrap();
        if let Some(toml) = toml {
            std::fs::write(dir.path().join("tweak.toml"), toml).unwrap();
        }
        dir
    }

    #[test]
    fn missing_toml_uses_defaults() {
        let dir = tweak_dir(None);
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert_eq!(tweak.x86_64_level, None);
        assert!(tweak.ftl_name.ends_with("-name"));
    }

    #[test]
    fn valid_toml_keeps_level() {
        let dir = tweak_dir(Some("x86_64_level = 3\n"));
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert_eq!(tweak.x86_64_level, Some(3));
        assert!(!tweak.needs_internet);
        let dir = tweak_dir(Some("needs_internet = true\n"));
        assert!(Tweak::from_dir(dir.path().to_path_buf()).unwrap().needs_internet);
    }

    // a broken tweak.toml must not fall back to no CPU requirement
    #[test]
    fn unparsable_toml_is_rejected() {
        for toml in ["x86_64_level = \"3\"\n", "x86_64_level = 3\nunknown = 1\n", "not toml ["] {
            let dir = tweak_dir(Some(toml));
            let err = Tweak::from_dir(dir.path().to_path_buf()).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{toml:?}");
        }
    }

    #[test]
    fn constraints_parse() {
        let dir = tweak_dir(Some("needs_dnf = true\nsecure_boot = false\n"));
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(tweak.needs_dnf);
        assert_eq!(tweak.secure_boot, Some(false));
        let dir = tweak_dir(None);
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(!tweak.needs_dnf);
        assert_eq!(tweak.secure_boot, None);
    }

    // (atomic, Secure Boot) combinations, `can_use_dnf` is false on atomic
    #[test]
    fn constraints_filter() {
        let constrained = Tweak { needs_dnf: true, secure_boot: Some(false), ..Tweak::default() };
        let free = Tweak::default();
        for (atomic, secure_boot, shown) in
            [(false, false, true), (false, true, false), (true, false, false), (true, true, false)]
        {
            assert_eq!(
                constrained.is_allowed(!atomic, secure_boot),
                shown,
                "atomic={atomic} secure_boot={secure_boot}"
            );
            assert!(
                free.is_allowed(!atomic, secure_boot),
                "atomic={atomic} secure_boot={secure_boot}"
            );
        }
        let dnf_only = Tweak { needs_dnf: true, ..Tweak::default() };
        assert!(dnf_only.is_allowed(true, true));
        assert!(!dnf_only.is_allowed(false, false));
        let sb_on_only = Tweak { secure_boot: Some(true), ..Tweak::default() };
        assert!(sb_on_only.is_allowed(false, true));
        assert!(!sb_on_only.is_allowed(true, false));
    }

    #[test]
    fn shipped_cachyos_kernel_constraints() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/tweaks/cachyos-kernel");
        let tweak = Tweak::from_dir(dir.into()).unwrap();
        assert!(tweak.needs_dnf);
        assert_eq!(tweak.secure_boot, Some(false));
    }

    #[test]
    fn shipped_tweaks_parse() {
        for entry in
            std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/tweaks")).unwrap()
        {
            let path = entry.unwrap().path();
            Tweak::from_dir(path.clone()).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        }
    }
}
