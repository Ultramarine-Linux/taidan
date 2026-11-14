use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::err::{Res, TaidanErrHelper};

pub static TWEAKS: LazyLock<Box<[Tweak]>> = LazyLock::new(|| {
    let tweaks = Tweak::list()
        .inspect_err(|err| tracing::error!(?err, "cannot list tweaks"))
        .unwrap_or_default();
    tracing::info!("found {} tweaks", tweaks.len());
    tweaks
});

pub async fn available(can_use_dnf: bool) -> Vec<&'static Tweak> {
    let secure_boot = crate::steps::secure_boot_enforced().await;
    futures::future::join_all(TWEAKS.iter().map(|t| async move {
        if t.is_available(can_use_dnf, secure_boot).await {
            Some(t)
        } else {
            tracing::info!(id=?t.id(), "hiding tweak this system can't take");
            None
        }
    }))
    .await
    .into_iter()
    .flatten()
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
/// ┃ ├╴condition (optional, must be executable)
/// ┃ └╴up (required, must be executable)
/// ┗╸other_tweak/
///   ├╴tweak.toml
///   └╴up
/// ```
///
/// The ID of a tweak is determined by the directory name (`my_tweak`, `other_tweak`).
///
/// `up 1` turns the tweak on. `up 0` turns it back off after an unfinished attempt turned it on.
///
/// `condition` runs without root privileges in the tweak directory, with no stdin. Exit code 0
/// offers the tweak; any other exit code, execution failure or a five-second timeout hides it.
/// It must only inspect the system, not change it. It receives `TAIDAN_X86_64_LEVEL` (0 on other
/// architectures) and `TAIDAN_SECURE_BOOT` (`0`, `1`, or `unknown`).
///
/// every field of `tweak.toml` is optional:
/// - `ftl_name` (alias `name`): fluent ID of the name, defaults to `<id>-name`
/// - `ftl_desc` (alias `desc`): fluent ID of the description, defaults to `<id>-desc`
/// - `needs_internet`: `up` downloads something, skipped offline
/// - `needs_dnf`: `up` installs packages with dnf, hidden on atomic and with `taidan0.skip_dnf`
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tweak {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(alias = "name")]
    pub ftl_name: String,
    #[serde(alias = "desc")]
    pub ftl_desc: String,
    pub needs_internet: bool,
    pub needs_dnf: bool,
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

    async fn is_available(&self, can_use_dnf: bool, secure_boot: Option<bool>) -> bool {
        (!self.needs_dnf || can_use_dnf) && self.condition_allows(secure_boot).await
    }

    async fn condition_allows(&self, secure_boot: Option<bool>) -> bool {
        match tokio::fs::metadata(self.path.join("condition")).await {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return true,
            Err(err) => {
                tracing::warn!(?err, id=?self.id(), "cannot inspect tweak condition");
                return false;
            }
            Ok(_) => {}
        }
        let secure_boot = match secure_boot {
            Some(false) => "0",
            Some(true) => "1",
            None => "unknown",
        };
        let mut cmd = tokio::process::Command::new("./condition");
        cmd.current_dir(&self.path)
            .env("TAIDAN_X86_64_LEVEL", x86_64_level().to_string())
            .env("TAIDAN_SECURE_BOOT", secure_boot)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .kill_on_drop(true);
        match tokio::time::timeout(std::time::Duration::from_secs(5), cmd.status()).await {
            Ok(Ok(status)) => status.success(),
            Ok(Err(err)) => {
                tracing::warn!(?err, id=?self.id(), "cannot run tweak condition");
                false
            }
            Err(err) => {
                tracing::warn!(?err, id=?self.id(), "tweak condition timed out");
                false
            }
        }
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
        let cfg_self = Self::from_cfg_path(&path)?;
        match std::fs::metadata(path.join("condition")) {
            Ok(metadata) if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("expected executable file `condition` in `{}`", path.display()),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
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
            .filter_map(|dir_entry| {
                Self::from_dir(dir_entry.path())
                    .inspect_err(|err| tracing::error!(?err, "skipping invalid tweak"))
                    .ok()
            })
            .collect::<Vec<_>>();
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

    #[tracing::instrument(skip(settings))]
    pub async fn run(&self, settings: &[u8], on: bool) -> bool {
        let verb = crate::Verb::RunTweak { id: self.id().to_string_lossy().into_owned(), on };
        let inherit = std::process::Stdio::inherit;
        let mut cmd = match crate::spawn_helper(&verb, settings, inherit(), inherit()).await {
            Ok(cmd) => cmd,
            Err(err) => {
                tracing::error!(?err, "cannot run pkexec");
                return false;
            }
        };
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

pub fn settings_json(settings: &crate::settings::Settings) -> serde_json::Result<Vec<u8>> {
    let mut settings = settings.clone();
    settings.proxy = settings.proxy.as_ref().and_then(crate::proxy::Proxy::without_credentials);
    serde_json::to_vec(&settings)
}

pub const ON_RECORD: &str = "/var/lib/taidan/tweaks-on";

pub const STARTED: char = '?';

#[must_use]
pub fn recorded_on() -> Vec<String> {
    std::fs::read_to_string(ON_RECORD)
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

pub async fn record_on(ids: &[String]) -> Res<()> {
    if ids.is_empty() {
        return clear_on_record().await;
    }
    crate::root(crate::Verb::RecordTweaks { ids: ids.to_vec() })
        .await
        .wrap_msg("cannot record the tweaks turned on")
}

pub(crate) fn write_on_record(ids: &[String]) -> std::io::Result<()> {
    let content: String = ids.iter().map(|id| format!("{id}\n")).collect();
    crate::helper::write_atomic(Path::new(ON_RECORD), content.as_bytes(), 0o644)
}

pub async fn clear_on_record() -> Res<()> {
    let failure = "cannot remove the tweaks record";
    crate::root_if_any_exists(&[ON_RECORD], crate::Verb::ClearTweakRecord, failure).await
}

// see https://gitlab.com/x86-psABIs/x86-64-ABI for the features each level needs
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
        assert!(tweak.ftl_name.ends_with("-name"));
        assert!(!tweak.needs_dnf);
    }

    #[test]
    fn valid_toml_keeps_internet_requirement() {
        let dir = tweak_dir(Some("needs_internet = true\n"));
        assert!(Tweak::from_dir(dir.path().to_path_buf()).unwrap().needs_internet);
    }

    #[test]
    fn unparsable_toml_is_rejected() {
        for toml in ["needs_dnf = \"true\"\n", "needs_dnf = true\nunknown = 1\n", "not toml ["] {
            let dir = tweak_dir(Some(toml));
            let err = Tweak::from_dir(dir.path().to_path_buf()).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{toml:?}");
        }
    }

    #[test]
    fn constraints_parse() {
        let dir = tweak_dir(Some("needs_dnf = true\n"));
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(tweak.needs_dnf);
    }

    fn condition(dir: &Path, script: &str) {
        let path = dir.join("condition");
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[tokio::test]
    async fn absent_condition_allows_tweak() {
        let dir = tweak_dir(None);
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(tweak.condition_allows(None).await);
    }

    #[tokio::test]
    async fn dnf_requirement_still_filters_tweaks() {
        let dir = tweak_dir(Some("needs_dnf = true\n"));
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(tweak.is_available(true, None).await);
        assert!(!tweak.is_available(false, None).await);
        let free = Tweak { needs_dnf: false, ..tweak };
        assert!(free.is_available(false, None).await);
    }

    #[tokio::test]
    async fn condition_exit_status_filters_tweak() {
        let dir = tweak_dir(None);
        for (script, allowed) in [
            ("#!/bin/sh\nexit 0\n", true),
            ("#!/bin/sh\nexit 1\n", false),
            ("#!/bin/sh\nexit 42\n", false),
            ("#!/missing-interpreter\n", false),
        ] {
            condition(dir.path(), script);
            let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
            assert_eq!(tweak.condition_allows(None).await, allowed, "{script}");
        }
    }

    #[tokio::test]
    async fn condition_receives_system_state_and_runs_in_tweak_directory() {
        let dir = tweak_dir(None);
        condition(
            dir.path(),
            "#!/bin/sh\n[ -f up ] && [ -n \"$TAIDAN_X86_64_LEVEL\" ] && \
             [ \"$TAIDAN_SECURE_BOOT\" = 0 ] && [ -z \"$(cat)\" ]\n",
        );
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(tweak.condition_allows(Some(false)).await);
        assert!(!tweak.condition_allows(Some(true)).await);
        assert!(!tweak.condition_allows(None).await);
    }

    #[test]
    fn non_executable_condition_is_rejected() {
        let dir = tweak_dir(None);
        std::fs::write(dir.path().join("condition"), "#!/bin/sh\nexit 0\n").unwrap();
        let err = Tweak::from_dir(dir.path().to_path_buf()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[tokio::test]
    async fn condition_timeout_hides_tweak() {
        let dir = tweak_dir(None);
        condition(dir.path(), "#!/bin/sh\nexec sleep 30\n");
        let tweak = Tweak::from_dir(dir.path().to_path_buf()).unwrap();
        assert!(!tweak.condition_allows(None).await);
    }

    #[test]
    fn shipped_cachyos_condition_checks_cpu_and_secure_boot() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/tweaks/cachyos-kernel/condition");
        for (level, secure_boot, allowed) in [
            ("0", "0", false),
            ("2", "0", false),
            ("3", "0", true),
            ("4", "0", true),
            ("3", "1", false),
            ("3", "unknown", false),
        ] {
            let status = std::process::Command::new(path)
                .env("TAIDAN_X86_64_LEVEL", level)
                .env("TAIDAN_SECURE_BOOT", secure_boot)
                .status()
                .unwrap();
            assert_eq!(status.success(), allowed, "level={level} secure_boot={secure_boot}");
        }
    }

    #[test]
    fn shipped_cachyos_kernel_constraints() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/tweaks/cachyos-kernel");
        let tweak = Tweak::from_dir(dir.into()).unwrap();
        assert!(tweak.needs_dnf);
        assert!(is_executable(&tweak.path.join("condition")));
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
