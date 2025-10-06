use super::parseutil as pu;
use crate::prelude::*;
use futures::FutureExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// # Errors
/// - if `flatpak` doesn't work correctly then maybe
///
/// # Panics
/// No shut up clippy this function is mathematically impossible to panic
#[allow(clippy::arithmetic_side_effects)]
#[allow(clippy::indexing_slicing)]
pub(super) async fn handle_flatpak(
    sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    f: impl Fn(&mut tokio::process::Command) -> &mut tokio::process::Command + Send,
) -> color_eyre::Result<()> {
    let mut cmd = tokio::process::Command::new("pkexec");
    cmd.args(["--user", "root", "flatpak"]);
    f(&mut cmd);
    let (writer, reader) = tokio::net::unix::pipe::pipe().expect("cannot create pipe");
    let writer = (writer.into_blocking_fd()).expect("cannot set blocking mode to pipe writer");
    let mut output = cmd
        .stdout(writer.try_clone().expect("cannot clone writer"))
        .stderr(writer)
        .spawn()
        .wrap_err("fail to run `flatpak`")?;
    let log_path = &*crate::TEMP_DIR.join("flatpak.stdout.log");
    let mut log = tokio::fs::File::create(log_path)
        .await
        .expect("cannot create log file");
    let mut stdout_lines = tokio::io::BufReader::new(reader).lines();
    loop {
        let line = futures::select! {
            line = async { (stdout_lines.next_line().await).wrap_err("cannot read stdout") }.fuse() => line?,
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
    pu::send_frac(
        &sender,
        100,
        100,
        crate::pages::InstallingPageMsg::UpdFlatpakProg,
    );
    Ok(())
}
