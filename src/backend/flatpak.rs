use std::os::fd::{FromRawFd, IntoRawFd};

use super::parseutil as pu;
use crate::prelude::*;
use futures::{FutureExt, StreamExt};
use smol::io::{AsyncBufReadExt, AsyncWriteExt};

/// # Errors
/// - if `flatpak` doesn't work correctly then maybe
///
/// # Panics
/// No shut up clippy this function is mathematically impossible to panic
#[allow(clippy::arithmetic_side_effects)]
#[allow(clippy::indexing_slicing)]
pub(super) async fn handle_flatpak(
    _sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    f: impl Fn(&mut smol::process::Command) -> &mut smol::process::Command + Send,
) -> color_eyre::Result<()> {
    let mut cmd = smol::process::Command::new("pkexec");
    cmd.args(["--user", "root", "flatpak"]);
    f(&mut cmd);
    let (reader, writer) = std::io::pipe().expect("cannot create pipe");
    // SAFETY: trivial conversion
    let writer = unsafe { std::os::fd::OwnedFd::from_raw_fd(writer.into_raw_fd()) };
    let mut output = cmd
        .stdout(writer.try_clone().expect("cannot clone writer"))
        .stderr(writer)
        .spawn()
        .wrap_err("fail to run `flatpak`")?;
    let log_path = &*crate::TEMP_DIR.join("flatpak.stdout.log");
    let mut log = smol::fs::File::create(log_path)
        .await
        .expect("cannot create log file");
    let reader =
        smol::io::BufReader::new(smol::Async::new(reader).expect("cannot turn pipe async"));
    let mut lines = reader.lines();
    // SAFETY: trivial conversion
    loop {
        let line = futures::select! {
            line = async { lines.next().await.transpose().wrap_err("cannot read stdout") }.fuse() => line?,
            res = pu::wait_for("flatpak", &mut output).fuse() => break res,
        };

        let Some(line) = line else {
            tracing::debug!("stdout EOF, waiting for flatpak complete");
            break pu::wait_for("flatpak", &mut output).await;
        };

        crate::awrite!(log <- "{line}").expect("cannot write to log");
        println!("flatpak: {line}");
    }
    .with_section(|| {
        std::fs::read_to_string(log_path)
            .unwrap_or_else(|e| format!("Cannot read flatpak.stdout.log: {e}"))
            .header("Output:")
    })?;
    Ok(())
}
