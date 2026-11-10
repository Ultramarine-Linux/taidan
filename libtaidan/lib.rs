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
pub mod time;
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
    /// some drivers couldn't be installed, the installation still finishes without them
    DriversFailed,
    /// the system language couldn't be set, usually since its language pack isn't installed
    /// (offline, or on an atomic edition), the installation still finishes in the old one
    LocaleFailed,
    /// updates were downloaded and install on the next restart
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
    let saved = match &settings.proxy {
        Some(proxy) if settings.proxy_persist => proxy.persist().await,
        // one saved by an earlier attempt would otherwise stay
        _ => proxy::clear().await,
    };
    if let Err(e) = saved {
        tracing::error!(?e, "cannot save the proxy");
    }
    i18n::fill_settings(settings, cfg);
    for stage in steps::Stage::all() {
        callback.send(InstallMsg::UpdStage(*stage));
        tracing::debug!(?stage, "Running pre()");
        stage.pre(settings, cfg, callback).await?;
        tracing::info!(?stage, "Running stage");
        stage.run(settings, cfg, callback).await?;
    }
    // the repos the install added, like the drivers' or a tweak's, may need overrides too
    if settings.proxy_persist
        && let Some(proxy) = &settings.proxy
        && let Err(e) = proxy.write_repo_overrides().await
    {
        tracing::warn!(?e, "cannot keep dnf from using the proxy for the bypassed hosts");
    }
    // the user may log in now, it was locked so a half-installed system couldn't be logged in to.
    steps::unlock_user(settings, cfg).await?;
    steps::fix_homed_ims(settings, cfg).await;
    // the user is theirs now, so no later run of Taidan should remove it. Not marked configured
    // if the record stays, since a machine that leaves the OOBE with it could later have the
    // account and its home removed by a rerun
    passwd::clear_record().await?;
    // the tweaks are the user's now too, a later rerun shouldn't undo them. After the user record,
    // which a retry needs, while this is only needed if the install doesn't finish. If it dies
    // before this, the repair in Taidan's startup clears it
    for attempt in 1..=3 {
        match tweaks::clear_on_record().await {
            Ok(()) => break,
            Err(e) => tracing::warn!(?e, attempt, "cannot remove the tweaks record"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    // right away rather than when the user leaves the last page, or a machine that dies on it
    // would boot into the OOBE again with the record gone and the user still there
    mark_configured().await;
    callback.send(InstallMsg::Finish);
    Ok(())
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

/// runs `cmd`, with `input` on its stdin if there's any, and returns its stdout
async fn run_cmd(
    mut cmd: tokio::process::Command,
    name: &'static str,
    input: Option<&[u8]>,
) -> Res<Vec<u8>> {
    use tokio::io::AsyncWriteExt;
    cmd.stdout(std::process::Stdio::piped());
    // for the error, which would say nothing about why otherwise
    cmd.stderr(std::process::Stdio::piped());
    // not the OOBE's, a tty nobody types into, where a command asking something would wait
    // forever. Without stdin, it fails
    cmd.stdin(if input.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    });
    let mut p = cmd.spawn().map_err(|e| Err::FailToRunProgram(name, e))?;
    if let Some(input) = input
        && let Some(mut stdin) = p.stdin.take()
    {
        // a failed write shows up as the program failing, so wait for it anyway
        if let Err(e) = stdin.write_all(input).await {
            tracing::error!(?e, name, "cannot write to stdin");
        }
        drop(stdin);
    }
    let p = p.wait_with_output().await.map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        let e = Err::program_fail(name, &p);
        // not an error, the output can name the user and the caller reports the failure itself.
        // The message ends with the program's last output line
        tracing::debug!(name, %e, "command failed");
        return Err(e);
    }
    if !p.stderr.is_empty() {
        tracing::debug!(name, stderr = %String::from_utf8_lossy(&p.stderr), "command succeeded");
    }
    Ok(p.stdout)
}

pub use helper::Verb;

/// runs `verb` as root through `pkexec` and [`helper`]
///
/// # Errors
/// - the helper failed to run
/// - it refused the request, or what it ran exited with a non-zero status code
pub async fn root(verb: Verb) -> Res<()> {
    root_with_stdin(verb, &[]).await
}

/// like [`root`], but returns what the helper printed
///
/// # Errors
/// - the helper failed to run
/// - it refused the request, or what it ran exited with a non-zero status code
pub async fn root_output(verb: Verb) -> Res<String> {
    let out = run_helper(&verb, &[]).await?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// like [`root`], but passes `input` on to the verb, for secrets that shouldn't be in arguments,
/// which anyone can read, and files
///
/// # Errors
/// - the helper failed to run
/// - it refused the request, or what it ran exited with a non-zero status code
pub async fn root_with_stdin(verb: Verb, input: &[u8]) -> Res<()> {
    run_helper(&verb, input).await.map(drop)
}

/// how long [`run_helper`] waits before each try again when `pkexec` itself fails
const PKEXEC_BACKOFF: [std::time::Duration; 2] =
    [std::time::Duration::from_secs(2), std::time::Duration::from_secs(4)];

/// runs the helper for `verb` with `input`, and returns its stdout. Tried again a few times when
/// `pkexec` refuses, which it can for a moment after a VT switch makes the session inactive
async fn run_helper(verb: &Verb, input: &[u8]) -> Res<Vec<u8>> {
    tracing::debug!(?verb, "running taidan-helper");
    let stdin = helper::stdin(verb, input);
    let run = || run_cmd(helper::command(verb), verb.name(), Some(&stdin));
    for (attempt, backoff) in PKEXEC_BACKOFF.into_iter().enumerate() {
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

/// whether `pkexec` refused to run the helper at all, i.e. it wasn't authorized or there was no
/// agent, not the helper refusing the request (exit 2) or what it ran failing
fn pkexec_refused(e: &Err) -> bool {
    // pkexec's messages aren't translated
    matches!(e, Err::ProgramFail { rc: Some(126 | 127), output, .. }
        if output.contains("Error executing command as another user"))
}

/// runs `argv` as `user`, with `input` on its stdin, directly if that's who this runs as already,
/// otherwise through the helper, which only runs things as the user the installation made
///
/// # Errors
/// - the command failed to run
/// - it exited with a non-zero status code
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

/// marks the system as configured so the OOBE doesn't run on the next boot, failures are logged.
/// Safe to run more than once
pub async fn mark_configured() {
    if let Err(e) = root(Verb::MarkConfigured).await {
        tracing::error!(?e, "cannot mark the system configured");
    }
}

/// whether the files that start the OOBE on boot are still there, see [`mark_configured`]
#[must_use]
pub fn marked_unconfigured() -> bool {
    ["/var/.unconfigured", "/.unconfigured"].iter().any(|m| std::path::Path::new(m).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkexec_refused_only_for_pkexec() {
        let fail = |rc, output: &str| Err::ProgramFail {
            name: "unlock-user",
            output: output.to_owned(),
            rc: Some(rc),
        };
        let denied = "Error executing command as another user: Not authorized\n\nThis incident has \
                      been reported.\n";
        assert!(pkexec_refused(&fail(127, denied)));
        let dismissed = "Error executing command as another user: Request dismissed\n";
        assert!(pkexec_refused(&fail(126, dismissed)));
        // the helper refusing the request, and what it ran failing
        assert!(!pkexec_refused(&fail(2, "taidan-helper: the input is over 4096 bytes\n")));
        assert!(!pkexec_refused(&fail(127, "sh: line 1: foo: command not found\n")));
        assert!(!pkexec_refused(&fail(1, denied)));
    }
}
