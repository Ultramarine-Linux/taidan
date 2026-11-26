//! libtaidan: Backend for Taidan (OOBE Application for Ultramarine Linux)
pub mod a11y;
pub mod analytics;
pub mod cfg;
pub mod command;
pub mod dnf;
pub mod err;
pub mod helper;
pub mod home;
pub mod i18n;
pub mod kconfig;
pub mod passwd;
pub mod prelude;
pub mod proxy;
pub mod server_setup;
pub mod settings;
pub mod steps;
pub mod theme;
pub mod tweaks;

use crate::prelude::*;

pub static TEMP_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    let dir =
        tempfile::Builder::new().prefix("libtaidan-").tempdir().expect("create tempdir").keep();
    std::fs::create_dir_all(&dir).expect("create tempdir");
    dir
});

#[derive(Debug, Clone)]
pub enum InstallMsg {
    UpdStep(steps::Step),
    Log(String),
    Warning(String),
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
    if !settings.nointernet {
        sync_clock().await;
    }
    use steps::InstallStep as _;
    let modules = steps::configured_steps(cfg);
    for module in &modules {
        let stage = module.stage();
        tracing::info!(?stage, "Preparing stage");
        let started = std::time::Instant::now();
        module.pre(settings, cfg).await?;
        tracing::debug!(?stage, elapsed = ?started.elapsed(), "Prepared stage");
    }
    tracing::info!(actions = ?settings.actions, "Planning the stages");
    let plan = steps::plan(settings, cfg).await;
    tracing::debug!(?plan, "Planned the stages");
    let mut shown = None;
    for (module, step) in modules.iter().zip(plan) {
        if shown != Some(step) {
            callback.send(InstallMsg::UpdStep(step));
            shown = Some(step);
        }
        let stage = module.stage();
        tracing::info!(?stage, "Running stage");
        let started = std::time::Instant::now();
        module
            .run(settings, cfg, callback)
            .await
            .inspect_err(|e| tracing::error!(?stage, %e, "Stage failed"))?;
        tracing::info!(?stage, elapsed = ?started.elapsed(), "Finished stage");
    }
    tracing::info!("Finishing up");
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

const CLOCK_SYNC_WAIT: std::time::Duration = std::time::Duration::from_secs(30);
const CLOCK_SYNC_POLL: std::time::Duration = std::time::Duration::from_secs(1);

async fn sync_clock() {
    tracing::info!(clock = %jiff::Timestamp::now(), "Turning on network time sync");
    if let Err(e) = root(Verb::EnableNtp).await {
        tracing::warn!(?e, "cannot turn on network time sync");
        return;
    }
    let deadline = tokio::time::Instant::now() + CLOCK_SYNC_WAIT;
    while tokio::time::Instant::now() < deadline {
        if clock_synchronized().await {
            tracing::info!(clock = %jiff::Timestamp::now(), "The clock is synchronized");
            return;
        }
        tokio::time::sleep(CLOCK_SYNC_POLL).await;
    }
    tracing::warn!(
        clock = %jiff::Timestamp::now(),
        "the clock isn't synchronized yet, downloads may fail if it's far off"
    );
}

async fn clock_synchronized() -> bool {
    tokio::process::Command::new("timedatectl")
        .args(["show", "--property=NTPSynchronized", "--value"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .is_ok_and(|out| out.stdout.trim_ascii() == b"yes")
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

async fn spawn(
    mut cmd: tokio::process::Command,
    name: &'static str,
    input: Option<&[u8]>,
    stdout: impl Into<std::process::Stdio>,
    stderr: impl Into<std::process::Stdio>,
) -> Res<tokio::process::Child> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    cmd.stdout(stdout).stderr(stderr);
    cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let program = cmd.as_std().get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .spawn()
        .inspect_err(|e| crate::command::report_spawn_error(&program, e))
        .map_err(Err::_fail_to_run(name))?;
    if let Some(input) = input
        && let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(input).await
    {
        tracing::error!(?e, name, "cannot write to stdin");
    }
    Ok(child)
}

pub(crate) async fn spawn_helper(
    verb: &Verb,
    input: &[u8],
    stdout: impl Into<std::process::Stdio>,
    stderr: impl Into<std::process::Stdio>,
) -> Res<tokio::process::Child> {
    let request = helper::stdin(verb, input);
    spawn(helper::command(verb), verb.name(), Some(&request), stdout, stderr).await
}

async fn run_cmd(
    cmd: tokio::process::Command,
    name: &'static str,
    input: Option<&[u8]>,
) -> Res<Vec<u8>> {
    use std::process::Stdio;
    let started = std::time::Instant::now();
    let child = spawn(cmd, name, input, Stdio::piped(), Stdio::piped()).await?;
    let p = child.wait_with_output().await.map_err(Err::_fail_to_run(name))?;
    let elapsed = started.elapsed();
    let output =
        format!("{}{}", String::from_utf8_lossy(&p.stdout), String::from_utf8_lossy(&p.stderr));
    crate::command::report_exit(name, p.status, &output);
    log_output(name, "stdout", &p.stdout);
    log_output(name, "stderr", &p.stderr);
    if !p.status.success() {
        let e = Err::program_fail(name, &p);
        tracing::warn!(name, rc = ?p.status.code(), ?elapsed, "command failed");
        return Err(e);
    }
    tracing::debug!(name, ?elapsed, "command succeeded");
    Ok(p.stdout)
}

fn log_output(name: &str, stream: &str, output: &[u8]) {
    for line in String::from_utf8_lossy(output).lines().filter(|l| !l.trim().is_empty()) {
        tracing::debug!("{name} {stream}: {line}");
    }
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
    tracing::info!(verb = verb.name(), "Running taidan-helper");
    tracing::debug!(?verb, "taidan-helper request");
    let request = helper::stdin(verb, input);
    let run = || run_cmd(helper::command(verb), verb.name(), Some(&request));
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

pub(crate) async fn run_here(argv: &[String], input: Option<&[u8]>) -> Res<()> {
    let Some((name, args)) = argv.split_first() else { return Ok(()) };
    let mut cmd = tokio::process::Command::new(name);
    cmd.args(args);
    run_cmd(cmd, "as-user", input).await.map(drop)
}

pub async fn as_user(
    user: &str,
    argv: impl IntoIterator<Item: AsRef<str>>,
    input: Option<&[u8]>,
) -> Res<()> {
    let argv = argv.into_iter().map(|a| a.as_ref().to_owned()).collect_vec();
    let current = uzers::get_current_username();
    if current.as_ref().is_some_and(|c| c.as_encoded_bytes() == user.as_bytes()) {
        return run_here(&argv, input).await;
    }
    root_with_stdin(Verb::AsUser { user: user.to_owned(), argv }, input.unwrap_or_default()).await
}

pub async fn mark_configured() {
    if let Err(e) = root(Verb::MarkConfigured).await {
        tracing::error!(?e, "cannot mark the system configured");
    }
}

pub(crate) const UNCONFIGURED_MARKERS: [&str; 2] = ["/var/.unconfigured", "/.unconfigured"];

#[must_use]
pub fn marked_unconfigured() -> bool {
    UNCONFIGURED_MARKERS.iter().any(|m| std::path::Path::new(m).exists())
}

pub(crate) async fn root_if_any_exists(
    paths: &[&str],
    verb: Verb,
    failure: &'static str,
) -> Res<()> {
    for path in paths {
        if tokio::fs::try_exists(path).await.unwrap_or(true) {
            return root(verb).await.wrap_msg(failure);
        }
    }
    Ok(())
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
