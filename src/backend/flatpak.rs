use super::parseutil as pu;
use tokio::io::AsyncBufReadExt;

use crate::prelude::*;

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
    let mut output = cmd
        .stdout(std::process::Stdio::piped())
        .spawn()
        .wrap_err("fail to run `flatpak`")?;
    let mut stdout_lines = tokio::io::BufReader::new(output.stdout.take().unwrap()).split(b'\n');
    futures::try_join!(
        async move {
            while let Some(line) =
                (stdout_lines.next_segment().await).wrap_err("cannot read stdout")?
            {
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
            }
            Ok(())
        },
        pu::wait_for("flatpak", output)
    )?;
    Ok(())
}
