use std::{
    os::fd::{FromRawFd, IntoRawFd},
    sync::LazyLock,
};

use super::parseutil as pu;
use crate::prelude::*;
use futures::{FutureExt, StreamExt};
use smol::io::{AsyncBufReadExt, AsyncWriteExt};

static PROGRESS_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\[\s*(\d+)/(\d+)\]").unwrap());

/// # Errors
/// - if `dnf5` doesn't work correctly then maybe
///
/// # Panics
/// No shut up clippy this function is mathematically impossible to panic
#[allow(clippy::arithmetic_side_effects)]
#[allow(clippy::indexing_slicing)]
pub(super) async fn handle_dnf(
    sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    f: impl Fn(&mut smol::process::Command) -> &mut smol::process::Command + Send,
) -> color_eyre::Result<()> {
    let mut cmd = smol::process::Command::new("pkexec");
    cmd.args(["--user", "root", "dnf5"]);
    f(&mut cmd);
    let (reader, writer) = std::io::pipe().expect("cannot create pipe");
    // SAFETY: trivial conversion
    let writer = unsafe { std::os::fd::OwnedFd::from_raw_fd(writer.into_raw_fd()) };
    let mut output = cmd
        .stdout(writer.try_clone().expect("cannot clone writer"))
        .stderr(writer)
        .spawn()
        .wrap_err("fail to run `dnf5`")?;
    let log_path = &*crate::TEMP_DIR.join("dnf5.stdout.log");
    let mut log = smol::fs::File::create(log_path)
        .await
        .expect("cannot create log file");
    let reader =
        smol::io::BufReader::new(smol::Async::new(reader).expect("cannot turn pipe async"));
    let mut lines = reader.lines();
    loop {
        let line = futures::select! {
            line = async { lines.next().await.transpose().wrap_err("cannot read stdout") }.fuse() => line?,
            res = pu::wait_for("dnf5", &mut output).fuse() => break res,
        };

        let Some(line) = line else {
            tracing::debug!("stdout EOF, waiting for dnf5 complete");
            break pu::wait_for("dnf5", &mut output).await;
        };

        crate::awrite!(log <- "{line}").expect("cannot write to log");
        println!("dnf5: {line}");
        let Some(matches) = PROGRESS_REGEX.captures(&line) else {
            continue;
        };
        let Ok(numerator) = matches[1].parse() else {
            continue;
        };
        let Ok(denominator) = matches[2].parse() else {
            continue;
        };
        pu::send_frac(
            &sender,
            numerator,
            denominator,
        );
    }
    .with_section(|| {
        std::fs::read_to_string(log_path)
            .unwrap_or_else(|e| format!("Cannot read dnf5.stdout.log: {e}"))
            .header("Output:")
    })?;
    Ok(())
}

#[derive(Debug)]
pub(super) struct EnableRepo {
    /// denotes the path to the file, the toml object and whether it is modified.
    files: std::collections::HashMap<std::path::PathBuf, (Vec<u8>, bool)>,
}

impl EnableRepo {
    /// Enable a dnf/yum repository.
    ///
    /// `repo` can be a url or a repo that has not been enabled but already defined in `/etc/yum.repos.d/`.
    ///
    /// You must run [`EnableRepo::save()`] after calling this function, since the toml editing
    /// is only done in memory.
    #[tracing::instrument]
    pub(super) async fn enable_repo(&mut self, repo: &str) -> color_eyre::Result<()> {
        tracing::debug!("Enabling repo");
        if let Some((doc, b)) = self.files.iter_mut().find_map(|(_, (doc, b))| {
            doc.starts_with(format!("[{repo}]\n").as_bytes())
                .then_some((doc, b))
        }) {
            const TARGET: &[u8] = b"\nenabled=";
            if memchr::memmem::find(doc, b"\nenabled=1").is_some() {
                return Ok(());
            }
            if let Some(i) = memchr::memmem::find(doc, TARGET) {
                doc[i + TARGET.len()] = b'1';
                let mut j = 1;
                while doc[i + TARGET.len() + j] != b'\n' {
                    doc[i + TARGET.len() + j] = b' ';
                    j += 1;
                }
            } else {
                let mut newdoc = doc[..repo.len() + 3].to_vec();
                newdoc.extend(b"enabled=1\n");
                newdoc.extend(&doc[repo.len() + 3..]);
                std::mem::swap(&mut newdoc, doc);
            }
            *b = true;
            return Ok(());
        }
        if repo.starts_with("https://") || repo.starts_with("http://") {
            let content = Self::get_repo_from_url(repo).await?;
            let path = std::path::Path::new("/etc/yum.repos.d/")
                .join(repo.rsplit_once('/').expect("cannot split url with `/`").1);
            smol::fs::write(&path, content)
                .await
                .wrap_err("cannot write to file")
                .with_note(|| path.display().to_string())?;
            return Ok(());
        }
        Err(eyre!("unknown repo `{repo}`")
            .note("this does not seem like a url")
            .note("this repo is not installed in /etc/yum.repos.d/"))
    }
    #[tracing::instrument]
    async fn get_repo_from_url(url: &str) -> color_eyre::Result<Vec<u8>> {
        tracing::debug!("Downloading repo file");
        let req = http_types::Request::get(url);
        let mut r = (crate::a::https_req(req).await).wrap_err("can't send req")?;
        if !r.status().is_success() {
            color_eyre::eyre::bail!("server fails to provide repo file: {:?}", r.status());
        }
        (r.body_bytes().await).map_err(|err| eyre!("can't get body: {err}"))
    }
    #[tracing::instrument]
    pub(super) async fn new() -> color_eyre::Result<Self> {
        let mut readdir = smol::fs::read_dir("/etc/yum.repos.d/").await?;
        let mut files = std::collections::HashMap::new();
        while let Some(f) = readdir.next().await.transpose()? {
            files.insert(f.path(), (std::fs::read(f.path())?, false));
        }
        Ok(Self { files })
    }
    #[tracing::instrument]
    pub(super) async fn save(&self) -> color_eyre::Result<()> {
        tracing::debug!("Saving repos");
        futures::future::try_join_all(
            self.files
                .iter()
                .filter(|(_, (_, b))| *b)
                .map(|(p, (doc, _))| smol::fs::write(p, doc)),
        )
        .await?;
        Ok(())
    }
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
