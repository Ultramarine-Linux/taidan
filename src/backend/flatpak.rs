use super::parseutil as pu;
use crate::prelude::*;
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
    let output = cmd
        .stdout(writer.try_clone().expect("cannot clone writer"))
        .stderr(writer)
        .spawn()
        .wrap_err("fail to run `flatpak`")?;
    let log_path = &*crate::TEMP_DIR.join("flatpak.stdout.log");
    let mut log = tokio::fs::File::create(log_path)
        .await
        .expect("cannot create log file");
    let mut stdout_lines = tokio::io::BufReader::new(reader).lines();
    futures::try_join!(
        async move {
            while let Some(line) =
                (stdout_lines.next_line().await).wrap_err("cannot read stdout")?
            {
                crate::awrite!(log <- "{line}").expect("cannot write to log");
                /*
                let mut it = line.iter().copied();
                let Some(space) = pu::search(&mut it, |c| Some(c == b' ')) else {
                    continue;
                };
                if !it.next().is_some_and(|c| c.is_ascii_digit()) {
                    continue;
                }
                let afterspace = space + 1;
                let Some(slash) = pu::search(&mut it, |c| Some(c == b'/')) else {
                    continue;
                };
                let slash = afterspace + slash + 1;
                let mut c = 42; // random number
                let Some(end) = pu::search(&mut it, |n| {
                    c = n;
                    Some(!c.is_ascii_digit())
                }) else {
                    continue;
                };
                let end = end + slash + 1;
                if !(c == 0x20 && it.next() == Some(0x26)) {
                    // not the `…` character
                    continue;
                }

                pu::send_frac(&sender, &line[afterspace..slash], &line[slash + 1..end]);
                */
            }
            Ok(())
        },
        pu::wait_for("flatpak", output)
    )
    .with_section(|| {
        std::fs::read_to_string(log_path)
            .unwrap_or_else(|e| format!("Cannot read flatpak.stdout.log: {e}"))
            .header("Output:")
    })?;
    pu::send_frac(&sender, 100, 100);
    Ok(())
}
