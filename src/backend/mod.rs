use relm4::Sender;
use steps::Step;

use crate::pages::InstallingPageMsg;

pub mod dnf;
pub mod flatpak;
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
        tracing::debug!(?stage, "Running pre()");
        stage.pre(&mut settings, sender.clone()).await?;
        tracing::info!(?stage, "Running stage");
        sender
            .send(InstallingPageMsg::UpdStage(*stage))
            .expect("sender dropped?");
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
    tracing::info!("Running SetTime");
    steps::Stage::SetTime(Default::default())
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

    pub fn search<F: FnMut(u8) -> Option<bool>, I: Iterator<Item = u8>>(
        it: &mut I,
        mut f: F,
    ) -> Option<usize> {
        let mut idx = 0;
        while !f(it.next()?)? {
            idx += 1;
        }
        Some(idx)
    }

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
    pub fn send_frac(
        sender: &relm4::Sender<crate::pages::InstallingPageMsg>,
        num: &[u8],
        den: &[u8],
    ) {
        let denominator: u32 = core::str::from_utf8(den).unwrap().parse().unwrap();
        if denominator == 0 {
            return;
        }
        let numerator: u32 = core::str::from_utf8(num).unwrap().parse().unwrap();
        sender
            .send(crate::pages::InstallingPageMsg::UpdFlatpakProg(
                f64::from(numerator) / f64::from(denominator),
            ))
            .expect("ui sender fails");
    }
}
