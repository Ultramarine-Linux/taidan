use tokio::io::AsyncBufReadExt;

use crate::prelude::*;

/// # Errors
/// - if `dnf5` doesn't work correctly then maybe
///
/// # Panics
/// No shut up clippy this function is mathematically impossible to panic
#[allow(clippy::arithmetic_side_effects)]
pub(super) async fn handle_dnf(
    sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    f: impl Fn(&mut tokio::process::Command) + Send,
) -> color_eyre::Result<()> {
    let mut cmd = tokio::process::Command::new("dnf5");
    f(&mut cmd);
    let mut output = cmd
        .stdout(std::process::Stdio::piped())
        .spawn()
        .wrap_err("fail to run `dnf5`")?;
    let mut stdout_lines = tokio::io::BufReader::new(output.stdout.take().unwrap()).split(b'\n');
    for task in [
        tokio::spawn(async move {
            'line: while let Some(line) =
                (stdout_lines.next_segment().await).wrap_err("cannot read stdout")?
            {
                let mut it = line.iter().copied();
                if it.next().is_none_or(|c| c != b'[') {
                    continue;
                }

                let mut slash = 1;
                let mut c;
                #[allow(clippy::arithmetic_side_effects)]
                while {
                    let Some(n) = it.next() else {
                        continue 'line;
                    };
                    c = n;
                    if c != b' ' && !c.is_ascii_digit() {
                        continue 'line;
                    }
                    n != b'/'
                } {
                    slash += 1;
                }

                let mut end = slash + 1;
                let mut c;

                #[allow(clippy::arithmetic_side_effects)]
                while {
                    let Some(n) = it.next() else {
                        continue 'line;
                    };
                    c = n;
                    if !c.is_ascii_digit() {
                        continue 'line;
                    }
                    n != b']'
                } {
                    end += 1;
                }

                if end == slash + 1 || slash == 1 {
                    continue;
                }
                // SAFETY: it is certain that there are only ascii digits
                let numerator: u32 = unsafe {
                    core::str::from_utf8_unchecked(line.get_unchecked(1..slash))
                        .parse()
                        .unwrap_unchecked()
                };
                // SAFETY: it is certain that there are only ascii digits
                let denominator: u32 = unsafe {
                    core::str::from_utf8_unchecked(line.get_unchecked(slash + 1..end))
                        .parse()
                        .unwrap_unchecked()
                };

                if denominator == 0 {
                    continue;
                }

                sender
                    .send(crate::pages::_11_installing::InstallingPageMsg::UpdSubProg(
                        f64::from(numerator) / f64::from(denominator),
                    ))
                    .expect("ui sender fails");
            }
            Ok(())
        }),
        tokio::spawn(async move {
            let status = output.wait().await.wrap_err("waiting for `dnf5` failed")?;
            if status.success() {
                Ok(())
            } else {
                Err(eyre!("`dnf5` failed with status: {status}"))
            }
        }),
    ] {
        task.await??;
    }
    Ok(())
}
