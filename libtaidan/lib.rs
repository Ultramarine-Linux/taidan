//! libtaidan: Backend for Taidan (OOBE Application for Ultramarine Linux)
pub mod a11y;
pub mod cfg;
pub mod dnf;
pub mod err;
pub mod flatpak;
pub mod i18n;
pub mod passwd;
pub mod prelude;
pub mod proxy;
pub mod server_setup;
pub mod settings;
pub mod steps;
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
#[tracing::instrument(skip(callback))]
pub async fn start_install(
    settings: &mut settings::Settings,
    cfg: &cfg::Config,
    callback: &impl Callback,
) -> Res<()> {
    tracing::info!("Starting installation");
    proxy::set(settings.proxy.clone());
    let saved = match &settings.proxy {
        Some(proxy) => proxy.persist().await,
        // one saved by an earlier attempt would otherwise stay
        None => proxy::clear().await,
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
    // the user is theirs now, so no later run of Taidan should remove it
    if let Err(e) = passwd::clear_record().await {
        tracing::error!(?e, "cannot remove the created user record");
    }
    callback.send(InstallMsg::Finish);
    Ok(())
}

#[allow(clippy::arithmetic_side_effects)]
mod parseutil {
    use crate::prelude::*;

    /// # Errors
    /// - wait for `output` failed…?
    /// - process exited with non-zero error code
    pub async fn wait_for(s: &'static str, output: &mut tokio::process::Child) -> Res<()> {
        let status = output
            .wait()
            .await
            .map_err(Err::_fail_to_run(s))
            .wrap_message(|_| format!("waiting for `{s}` failed"))?;
        if status.success() {
            Ok(())
        } else {
            Err(Err::Message(format!("`{s}` failed with status: {status}")))
        }
    }

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

/// `name` with `args` as `user` through `pkexec`, with the environment and proxy passed on since
/// `pkexec` clears them. Root only gets the locale, the rest (`HOME`, `XDG_*`, the session bus) is
/// the OOBE session's and would have root write into it
fn pkexec_cmd<I>(user: &str, name: &'static str, args: I) -> tokio::process::Command
where
    I: IntoIterator,
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    let as_root = user == "root";
    let mut cmd = tokio::process::Command::new("pkexec");
    cmd.args(["--user", user, "env"])
        .args(
            std::env::vars()
                .filter(|(k, _)| !as_root || is_locale_var(k))
                .map(|(k, v)| format!("{k}={v}")),
        )
        .args(proxy::env())
        .arg(name)
        .args(args);
    cmd
}

fn is_locale_var(key: &str) -> bool {
    key == "LANG" || key == "LANGUAGE" || key.starts_with("LC_")
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
    cmd.stdin(if input.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::inherit()
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
        tracing::error!(%e, "command failed");
        return Err(e);
    }
    if !p.stderr.is_empty() {
        tracing::debug!(name, stderr = %String::from_utf8_lossy(&p.stderr), "command succeeded");
    }
    Ok(p.stdout)
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn pkexec<I: IntoIterator + std::fmt::Debug>(
    user: &str,
    name: &'static str,
    args: I,
) -> Res<()>
where
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    tracing::debug!(?name, ?args, "running pkexec");
    run_cmd(pkexec_cmd(user, name, args), name, None).await.map(drop)
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn root<I: IntoIterator + std::fmt::Debug>(name: &'static str, args: I) -> Res<()>
where
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    pkexec("root", name, args).await
}

/// like [`root`], but returns what the command printed
///
/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn root_output<I: IntoIterator + std::fmt::Debug>(
    name: &'static str,
    args: I,
) -> Res<String>
where
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    tracing::debug!(?name, ?args, "running pkexec");
    let out = run_cmd(pkexec_cmd("root", name, args), name, None).await?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// like [`root`], but feeds `input` to the command's stdin, for secrets that shouldn't be in its
/// arguments, which anyone can read
///
/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn root_with_stdin<I: IntoIterator + std::fmt::Debug>(
    name: &'static str,
    args: I,
    input: &[u8],
) -> Res<()>
where
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    tracing::debug!(?name, ?args, "running pkexec");
    run_cmd(pkexec_cmd("root", name, args), name, Some(input)).await.map(drop)
}

/// # Errors
/// - cannot run xhost because idk
pub async fn xhost_local() -> Res<()> {
    steps::acmd("xhost", &["+", "local:"])
        .await
        .wrap_msg("cannot run xhost to pass display; is the current user in group wheel?")
}
