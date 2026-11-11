//! libtaidan: Backend for Taidan (OOBE Application for Ultramarine Linux)
pub mod a11y;
pub mod cfg;
pub mod dnf;
pub mod err;
pub mod flatpak;
pub mod helper;
pub mod i18n;
pub mod passwd;
pub mod prelude;
pub mod proxy;
pub mod server_setup;
pub mod settings;
pub mod steps;
pub mod theme;
pub mod tweaks;

use crate::prelude::*;
use steps::Step;

pub static TEMP_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    let dir =
        tempfile::Builder::new().prefix("libtaidan-").tempdir().expect("create tempdir").keep();
    std::fs::create_dir_all(&dir).expect("create tempdir");
    dir
});

#[derive(Debug, Clone)]
pub enum InstallMsg {
    UpdStage(steps::Stage),
    NeedsReboot,
    DriversFailed,
    LocaleFailed,
    UpdatesReady,
    Finish,
    StageProgress(f64),
}

pub trait Callback {
    fn send(&self, msg: InstallMsg);
}

impl<F: Fn(InstallMsg)> Callback for F {
    fn send(&self, msg: InstallMsg) {
        self(msg)
    }
}

/// Start the installation process.
///
/// # Panics
///
/// Panics if the given sender is dropped.
///
/// # Errors
///
/// Stage failures are propagated.
#[tracing::instrument(skip(settings, callback))]
pub async fn start_install(
    settings: &mut settings::Settings,
    cfg: &cfg::Config,
    callback: &impl Callback,
) -> Res<()> {
    tracing::info!("Starting installation");
    proxy::set(settings.proxy.clone());
    persist_proxy(settings).await;
    i18n::fill_settings(settings, cfg);
    for stage in steps::Stage::all() {
        callback.send(InstallMsg::UpdStage(*stage));
        tracing::debug!(?stage, "Running pre()");
        stage.pre(settings, cfg, callback).await?;
        tracing::info!(?stage, "Running stage");
        stage.run(settings, cfg, callback).await?;
    }
    write_proxy_repo_overrides(settings).await;
    steps::unlock_user(settings, cfg).await?;
    steps::fix_homed_ims(settings, cfg).await;
    // a rerun with the user record still there could remove the account and its home, so only
    // mark the system configured once the record is gone
    passwd::clear_record().await?;
    clear_tweaks_record().await;
    mark_configured().await;
    callback.send(InstallMsg::Finish);
    Ok(())
}

async fn persist_proxy(settings: &settings::Settings) {
    let saved = match &settings.proxy {
        Some(proxy) if settings.proxy_persist => proxy.persist().await,
        _ => proxy::clear().await,
    };
    if let Err(e) = saved {
        tracing::error!(?e, "cannot save the proxy");
    }
}

async fn write_proxy_repo_overrides(settings: &settings::Settings) {
    if settings.proxy_persist
        && let Some(proxy) = &settings.proxy
        && let Err(e) = proxy.write_repo_overrides().await
    {
        tracing::warn!(?e, "cannot keep dnf from using the proxy for the bypassed hosts");
    }
}

async fn clear_tweaks_record() {
    for attempt in 1..=3 {
        match tweaks::clear_on_record().await {
            Ok(()) => break,
            Err(e) => tracing::warn!(?e, attempt, "cannot remove the tweaks record"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

#[allow(clippy::arithmetic_side_effects)]
mod parseutil {
    /// # Panics
    /// - cannot convert bytes to `&str`
    /// - cannot parse to `u32`
    pub fn send_frac(callback: &impl crate::Callback, num: u32, den: u32) {
        if den == 0 {
            return;
        }

        callback.send(crate::InstallMsg::StageProgress(f64::from(num) / f64::from(den)))
    }
}

async fn run_cmd(
    mut cmd: tokio::process::Command,
    name: &'static str,
    input: Option<&[u8]>,
) -> Res<Vec<u8>> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut p = cmd.spawn().map_err(|e| Err::FailToRunProgram(name, e))?;
    if let Some(input) = input
        && let Some(mut stdin) = p.stdin.take()
        && let Err(e) = stdin.write_all(input).await
    {
        tracing::error!(?e, name, "cannot write to stdin");
    }
    let p = p.wait_with_output().await.map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        let e = Err::program_fail(name, &p);
        tracing::debug!(name, %e, "command failed");
        return Err(e);
    }
    if !p.stderr.is_empty() {
        tracing::debug!(name, stderr = %String::from_utf8_lossy(&p.stderr), "command succeeded");
    }
    Ok(p.stdout)
}

pub use helper::Verb;

pub async fn root(verb: Verb) -> Res<()> {
    root_with_stdin(verb, &[]).await
}

pub async fn root_output(verb: Verb) -> Res<String> {
    let out = run_helper(&verb, &[]).await?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

pub async fn root_with_stdin(verb: Verb, input: &[u8]) -> Res<()> {
    run_helper(&verb, input).await.map(drop)
}

// pkexec refuses for a moment after a vt switch makes the session inactive
const PKEXEC_RETRY_BACKOFF: [std::time::Duration; 2] =
    [std::time::Duration::from_secs(2), std::time::Duration::from_secs(4)];

async fn run_helper(verb: &Verb, input: &[u8]) -> Res<Vec<u8>> {
    tracing::debug!(?verb, "running taidan-helper");
    let stdin = helper::stdin(verb, input);
    let run = || run_cmd(helper::command(verb), verb.name(), Some(&stdin));
    for (attempt, backoff) in PKEXEC_RETRY_BACKOFF.into_iter().enumerate() {
        match run().await {
            Err(e) if pkexec_refused(&e) => {
                tracing::warn!(?e, attempt, verb = verb.name(), "pkexec refused, retrying");
                tokio::time::sleep(backoff).await;
            }
            res => return res,
        }
    }
    run().await
}

fn pkexec_refused(e: &Err) -> bool {
    // pkexec's messages aren't translated
    matches!(e, Err::ProgramFail { rc: Some(126 | 127), output, .. }
        if output.contains("Error executing command as another user"))
}

pub async fn as_user(user: &str, argv: Vec<String>, input: Option<&[u8]>) -> Res<()> {
    let current = uzers::get_current_username();
    if current.as_ref().is_some_and(|c| c.as_encoded_bytes() == user.as_bytes()) {
        let Some((name, args)) = argv.split_first() else { return Ok(()) };
        let mut cmd = tokio::process::Command::new(name);
        cmd.args(args);
        return run_cmd(cmd, "as-user", input).await.map(drop);
    }
    root_with_stdin(Verb::AsUser { user: user.to_owned(), argv }, input.unwrap_or_default()).await
}

pub async fn mark_configured() {
    if let Err(e) = root(Verb::MarkConfigured).await {
        tracing::error!(?e, "cannot mark the system configured");
    }
}

#[must_use]
pub fn marked_unconfigured() -> bool {
    ["/var/.unconfigured", "/.unconfigured"].iter().any(|m| std::path::Path::new(m).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fail(rc: i32, output: &str) -> Err {
        Err::ProgramFail { name: "unlock-user", output: output.to_owned(), rc: Some(rc) }
    }

    const PKEXEC_DENIED: &str = "Error executing command as another user: Not authorized\n\nThis \
                                 incident has been reported.\n";

    #[test]
    fn pkexec_refused_for_pkexec() {
        assert!(pkexec_refused(&fail(127, PKEXEC_DENIED)));
        let dismissed = "Error executing command as another user: Request dismissed\n";
        assert!(pkexec_refused(&fail(126, dismissed)));
    }

    #[test]
    fn pkexec_refused_not_for_the_helper_or_what_it_ran() {
        assert!(!pkexec_refused(&fail(2, "taidan-helper: the input is over 4096 bytes\n")));
        assert!(!pkexec_refused(&fail(127, "sh: line 1: foo: command not found\n")));
        assert!(!pkexec_refused(&fail(1, PKEXEC_DENIED)));
    }
}
