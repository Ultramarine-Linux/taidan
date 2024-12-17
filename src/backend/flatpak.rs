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
    sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    f: impl Fn(&mut tokio::process::Command) -> &mut tokio::process::Command + Send,
) -> color_eyre::Result<()> {
    let mut cmd = tokio::process::Command::new("flatpak");
    f(&mut cmd);
    let mut output = cmd
        .stdout(std::process::Stdio::piped())
        .spawn()
        .wrap_err("fail to run `flatpak`")?;
    let mut stdout_lines = tokio::io::BufReader::new(output.stdout.take().unwrap()).split(b'\n');
    for task in [
        tokio::spawn(async move {
            'line: while let Some(line) =
                (stdout_lines.next_segment().await).wrap_err("cannot read stdout")?
            {
                let mut it = line.iter().copied();
                let mut afterspace = 0;
                while {
                    let Some(n) = it.next() else {
                        continue 'line;
                    };
                    n != b' '
                } {
                    afterspace += 1;
                }
                let Some(mut c) = it.next() else {
                    continue 'line;
                };
                afterspace += 1;
                if !c.is_ascii_digit() {
                    continue 'line;
                }
                let mut slash = afterspace + 1;
                while {
                    let Some(n) = it.next() else {
                        continue 'line;
                    };
                    n != b'/'
                } {
                    slash += 1;
                }

                let mut end = slash + 1;
                while {
                    let Some(n) = it.next() else {
                        continue 'line;
                    };
                    c = n;
                    c.is_ascii_digit()
                } {
                    end += 1;
                }
                if !(c == 0x20 && it.next() == Some(0x26)) {
                    // not the `…` character
                    continue 'line;
                }

                let numerator: u32 = (core::str::from_utf8(&line[afterspace..slash])
                    .unwrap()
                    .parse())
                .unwrap();
                let denominator: u32 =
                    (core::str::from_utf8(&line[slash + 1..end]).unwrap().parse()).unwrap();

                if denominator == 0 {
                    continue;
                }

                sender
                    .send(
                        crate::pages::_11_installing::InstallingPageMsg::UpdFlatpakProg(
                            f64::from(numerator) / f64::from(denominator),
                        ),
                    )
                    .expect("ui sender fails");
            }
            Ok(())
        }),
        tokio::spawn(async move {
            let status = output
                .wait()
                .await
                .wrap_err("waiting for `flatpak` failed")?;
            if status.success() {
                Ok(())
            } else {
                Err(eyre!("`flatpak` failed with status: {status}"))
            }
        }),
    ] {
        task.await??;
    }
    Ok(())
}
