//! libtaidan: Backend for Taidan (OOBE Application for Ultramarine Linux)
pub mod cfg;
pub mod dnf;
pub mod err;
pub mod flatpak;
pub mod i18n;
pub mod passwd;
pub mod prelude;
pub mod settings;
pub mod steps;
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
    UpdStage(crate::steps::Stage),
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
    mut settings: settings::Settings,
    cfg: &cfg::Config,
    callback: &impl Callback,
) -> Res<()> {
    tracing::info!("Starting installation");
    for stage in steps::Stage::all() {
        callback.send(InstallMsg::UpdStage(*stage));
        tracing::debug!(?stage, "Running pre()");
        stage.pre(&mut settings, cfg, callback).await?;
        tracing::info!(?stage, "Running stage");
        stage.run(&settings, cfg, callback).await?;
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
        let status = (output.wait().await.map_err(Err::_fail_to_run(s)))
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

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn pkexec<'u, I: IntoIterator + std::fmt::Debug>(
    user: &'u str,
    name: &'static str,
    args: I,
) -> Res<()>
where
    <I as std::iter::IntoIterator>::Item: std::convert::AsRef<std::ffi::OsStr>,
{
    tracing::debug!(?name, ?args, "running pkexec");
    // TODO: use logic from anda
    let stdio = std::process::Stdio::piped();
    let p = tokio::process::Command::new("pkexec")
        .args(["--user", user, "env"])
        .args(std::env::vars().map(|(k, v)| format!("{k}={v}")))
        .arg(name)
        .args(args)
        .stdout(stdio)
        .spawn()
        .map_err(|e| Err::FailToRunProgram(name, e))?;
    let p = p.wait_with_output().await.map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        // TODO: more logging
        return Err(Err::ProgramFail {
            name,
            output: String::from_utf8_lossy(&p.stdout).into_owned(),
            rc: p.status.code(),
        });
    }
    Ok(())
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn root<I: IntoIterator + std::fmt::Debug>(name: &'static str, args: I) -> Res<()>
where
    <I as std::iter::IntoIterator>::Item: std::convert::AsRef<std::ffi::OsStr>,
{
    pkexec("root", name, args).await
}

/// # Errors
/// - cannot run xhost because idk
pub async fn xhost_local() -> Res<()> {
    steps::acmd("xhost", &["+", "local:"])
        .await
        .wrap_msg("cannot run xhost to pass display; is the current user in group wheel?")
}
