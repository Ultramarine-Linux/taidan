use std::sync::LazyLock;

use super::parseutil as pu;
use futures::FutureExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::prelude::*;

static PROGRESS_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\[\s*(\d+)/(\d+)]").unwrap());

/// how many of the last lines a failed [`run_logged`] puts in its error
const TAIL_LINES: usize = 10;
/// how long [`run_logged`] waits for another line once the program exited
const DRAIN_LINE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
/// how long [`run_logged`] reads output in all once the program exited, so something it started
/// that keeps printing can't hold it up forever
const DRAIN_TOTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// runs `verb` through the helper, and passes each line what it runs prints to `on_line`. The
/// output also goes to stdout and is appended to `<name>.stdout.log` in [`crate::TEMP_DIR`]
///
/// # Errors
/// - the program cannot be run
/// - it exits with a non-zero status, the error has the last lines it printed
pub(super) async fn run_logged(
    name: &'static str,
    verb: &crate::Verb,
    mut on_line: impl FnMut(&str),
) -> Res<()> {
    let mut cmd = crate::helper::command(verb);
    let (writer, reader) = tokio::net::unix::pipe::pipe().map_err(Err::_fail_to_run(name))?;
    let writer = writer.into_blocking_fd().map_err(Err::_fail_to_run(name))?;
    let mut child = cmd
        // only the request, the helper doesn't pass stdin on to what it runs, and nobody types
        // into the OOBE's tty, a question would wait forever
        .stdin(std::process::Stdio::piped())
        .stdout(writer.try_clone().map_err(Err::_fail_to_run(name))?)
        .stderr(writer)
        .spawn()
        .map_err(Err::_fail_to_run(name))?;
    // it holds the write end too, the pipe would never reach EOF otherwise
    drop(cmd);
    if let Some(mut stdin) = child.stdin.take() {
        // a failed write shows up as the helper refusing, so wait for it anyway
        if let Err(e) = stdin.write_all(&crate::helper::stdin(verb, &[])).await {
            tracing::error!(?e, name, "cannot pass the request to the helper");
        }
    }

    let log_path = crate::TEMP_DIR.join(format!("{name}.stdout.log"));
    // appended, so a later run doesn't wipe the log of an earlier step
    let mut log =
        match tokio::fs::OpenOptions::new().create(true).append(true).open(&log_path).await {
            Ok(log) => Some(log),
            Err(e) => {
                tracing::warn!(?e, ?log_path, "cannot open log file");
                None
            }
        };
    let mut lines = tokio::io::BufReader::new(reader).split(b'\n');
    let mut tail = std::collections::VecDeque::with_capacity(TAIL_LINES);
    let mut status = None;
    // set once it exits
    let mut drain_until: Option<tokio::time::Instant> = None;
    loop {
        let line = if let Some(drain_until) = drain_until {
            // it exited, but something it started may still have the pipe open
            let deadline = drain_until.min(tokio::time::Instant::now() + DRAIN_LINE_TIMEOUT);
            match tokio::time::timeout_at(deadline, lines.next_segment()).await {
                Ok(line) => line,
                Err(_) => break,
            }
        } else {
            futures::select! {
                line = lines.next_segment().fuse() => line,
                s = child.wait().fuse() => {
                    status = Some(s.map_err(Err::_fail_to_run(name))?);
                    drain_until = Some(tokio::time::Instant::now() + DRAIN_TOTAL_TIMEOUT);
                    continue;
                }
            }
        };
        let line = match line {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                tracing::warn!(?e, name, "cannot read output");
                break;
            }
        };
        let line = String::from_utf8_lossy(&line);
        let line = line.trim_end_matches('\r');

        if let Some(f) = &mut log
            && let Err(e) = f.write_all(format!("{line}\n").as_bytes()).await
        {
            tracing::warn!(?e, ?log_path, "cannot write to log file");
            log = None;
        }
        println!("{name}: {line}");
        if !line.trim().is_empty() {
            if tail.len() == TAIL_LINES {
                tail.pop_front();
            }
            tail.push_back(line.to_owned());
        }
        on_line(line);
    }
    let status = match status {
        Some(status) => status,
        None => child.wait().await.map_err(Err::_fail_to_run(name))?,
    };
    if status.success() {
        return Ok(());
    }
    let e = Err::ProgramFail {
        name,
        output: tail.into_iter().collect::<Vec<_>>().join("\n"),
        rc: status.code(),
    };
    tracing::error!(%e, "command failed");
    Err(e)
}

/// # Errors
/// - if `dnf5` doesn't work correctly then maybe
pub(super) async fn handle_dnf(callback: &impl crate::Callback, verb: crate::Verb) -> Res<()> {
    run_logged("dnf5", &verb, |line| {
        let Some(matches) = PROGRESS_REGEX.captures(line) else {
            return;
        };
        let (Some(num), Some(den)) = (matches.get(1), matches.get(2)) else {
            return;
        };
        if let (Ok(num), Ok(den)) = (num.as_str().parse(), den.as_str().parse()) {
            pu::send_frac(callback, num, den);
        }
    })
    .await
}

/// enables `repo`, either the URL of a `.repo` file or the id of a repo that's already defined
/// but disabled. The helper only takes this from a caller that's root already, see
/// [`crate::helper::Verb::EnableRepo`]
///
/// # Errors
/// - `dnf5 config-manager` fails, e.g. the repo doesn't exist or the file can't be downloaded
#[tracing::instrument]
pub(super) async fn enable_repo(repo: &str) -> Res<()> {
    tracing::debug!("Enabling repo");
    crate::root(crate::Verb::EnableRepo { repo: repo.to_owned() })
        .await
        .wrap_message(|_| format!("cannot enable repo `{repo}`"))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn dnf_progress_regex() {
        let line = "[ 1/50] idk";
        let matches = PROGRESS_REGEX.captures(line).unwrap();
        let numerator: u32 = matches[1].parse().unwrap();
        let denominator: u32 = matches[2].parse().unwrap();
        assert_eq!(numerator, 1);
        assert_eq!(denominator, 50);
    }
}
