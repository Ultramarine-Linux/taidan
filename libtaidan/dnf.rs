use std::sync::LazyLock;

use super::parseutil as pu;
use futures::FutureExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::prelude::*;

static PROGRESS_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\[\s*(\d+)/(\d+)]").unwrap());

const ERROR_TAIL_LINES: usize = 10;
const DRAIN_LINE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub(super) async fn run_logged(
    name: &'static str,
    verb: &crate::Verb,
    mut on_line: impl FnMut(&str),
) -> Res<()> {
    let (writer, reader) = tokio::net::unix::pipe::pipe().map_err(Err::_fail_to_run(name))?;
    let writer = writer.into_blocking_fd().map_err(Err::_fail_to_run(name))?;
    let mut child = crate::helper::command(verb)
        .stdin(std::process::Stdio::piped())
        .stdout(writer.try_clone().map_err(Err::_fail_to_run(name))?)
        .stderr(writer)
        .spawn()
        .map_err(Err::_fail_to_run(name))?;
    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(&crate::helper::stdin(verb, &[])).await
    {
        tracing::error!(?e, name, "cannot pass the request to the helper");
    }

    let log_path = crate::TEMP_DIR.join(format!("{name}.stdout.log"));
    let mut log =
        match tokio::fs::OpenOptions::new().create(true).append(true).open(&log_path).await {
            Ok(log) => Some(log),
            Err(e) => {
                tracing::warn!(?e, ?log_path, "cannot open log file");
                None
            }
        };
    let mut lines = tokio::io::BufReader::new(reader).split(b'\n');
    let mut tail = std::collections::VecDeque::with_capacity(ERROR_TAIL_LINES);
    let mut status = None;
    let mut drain_deadline: Option<tokio::time::Instant> = None;
    loop {
        let line = if let Some(end) = drain_deadline {
            let deadline = end.min(tokio::time::Instant::now() + DRAIN_LINE_TIMEOUT);
            match tokio::time::timeout_at(deadline, lines.next_segment()).await {
                Ok(line) => line,
                Err(_) => break,
            }
        } else {
            futures::select! {
                line = lines.next_segment().fuse() => line,
                s = child.wait().fuse() => {
                    status = Some(s.map_err(Err::_fail_to_run(name))?);
                    drain_deadline = Some(tokio::time::Instant::now() + DRAIN_TIMEOUT);
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
            if tail.len() == ERROR_TAIL_LINES {
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
    let e = Err::ProgramFail { name, output: Vec::from(tail).join("\n"), rc: status.code() };
    tracing::error!(%e, "command failed");
    Err(e)
}

pub(super) async fn handle_dnf(callback: &impl crate::Callback, verb: crate::Verb) -> Res<()> {
    run_logged("dnf5", &verb, |line| {
        if let Some(m) = PROGRESS_REGEX.captures(line)
            && let (Ok(num), Ok(den)) = (m[1].parse(), m[2].parse())
        {
            pu::send_frac(callback, num, den);
        }
    })
    .await
}

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
