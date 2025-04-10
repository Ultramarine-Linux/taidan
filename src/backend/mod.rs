use relm4::Sender;
use steps::Step;

use crate::pages::InstallingPageMsg;

pub mod dnf;
pub mod flatpak;
pub mod i18n;
pub mod settings;
pub mod steps;
pub mod theme;

use crate::prelude::*;

#[tracing::instrument]
pub async fn start_install(
    mut settings: settings::Settings,
    sender: Sender<InstallingPageMsg>,
) -> color_eyre::Result<()> {
    tracing::info!("Starting installation");
    for stage in steps::Stage::all() {
        sender
            .send(InstallingPageMsg::UpdStage(*stage))
            .expect("sender dropped?");
        tracing::debug!(?stage, "Running pre()");
        stage.pre(&mut settings, sender.clone()).await?;
        tracing::info!(?stage, "Running stage");
        (stage.run(&settings, sender.clone()).await)
            .wrap_err("stage failed")
            .with_note(|| format!("Stage: {stage:?}"))?;
    }
    sender
        .send(InstallingPageMsg::Finish)
        .expect("sender dropped?");
    Ok(())
}

#[allow(clippy::default_trait_access)]
#[tracing::instrument]
pub async fn start_simple_install(
    settings: settings::Settings,
    sender: Sender<InstallingPageMsg>,
) -> color_eyre::Result<()> {
    tracing::info!("Starting installation");
    tracing::info!("Running UserAdd");
    steps::Stage::UserAdd(Default::default())
        .run(&settings, sender.clone())
        .await?;
    tracing::info!("Running Script");
    steps::Stage::Script(Default::default())
        .run(&settings, sender.clone())
        .await?;
    tracing::info!("Running DriversCodecs");
    steps::Stage::DriversCodecs(Default::default())
        .run(&settings, sender.clone())
        .await?;
    sender
        .send(InstallingPageMsg::Finish)
        .expect("sender dropped?");
    Ok(())
}

#[allow(clippy::arithmetic_side_effects)]
mod parseutil {
    use crate::prelude::*;

    /// # Errors
    /// - wait for `output` failed…?
    /// - process exited with non-zero error code
    pub async fn wait_for(
        s: &'static str,
        mut output: tokio::process::Child,
    ) -> color_eyre::Result<()> {
        let status = output
            .wait()
            .await
            .wrap_err(format!("waiting for `{s}` failed"))?;
        if status.success() {
            Ok(())
        } else {
            Err(eyre!("`{s}` failed with status: {status}"))
        }
    }

    /// # Panics
    /// - cannot convert bytes to `&str`
    /// - cannot parse to `u32`
    pub fn send_frac(sender: &relm4::Sender<crate::pages::InstallingPageMsg>, num: u32, den: u32) {
        if den == 0 {
            return;
        }
        sender
            .send(crate::pages::InstallingPageMsg::UpdFlatpakProg(
                f64::from(num) / f64::from(den),
            ))
            .expect("ui sender fails");
    }
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn pkexec(user: &str, name: &str, args: &[&str]) -> color_eyre::Result<()> {
    tracing::debug!(?name, ?args, "running pkexec");
    let p = tokio::process::Command::new("pkexec")
        .args(["--user", user, name])
        .args(args)
        .status()
        .await
        .wrap_err(format!("fail to run `{name}`"))?;
    if !p.success() {
        return Err(eyre!("`{name}` failed").note(format!("Exit code: {:?}", p.code())));
    }
    Ok(())
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn root(name: &str, args: &[&str]) -> color_eyre::Result<()> {
    tracing::debug!(?name, ?args, "running pkexec");
    let p = tokio::process::Command::new("pkexec")
        .args(["--user", "root", name])
        .args(args)
        .status()
        .await
        .wrap_err(format!("fail to run `{name}`"))?;
    if !p.success() {
        return Err(eyre!("`{name}` failed").note(format!("Exit code: {:?}", p.code())));
    }
    Ok(())
}
