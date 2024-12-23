use super::parseutil as pu;
use tokio::io::AsyncBufReadExt;

use crate::prelude::*;

/// # Errors
/// - if `dnf5` doesn't work correctly then maybe
///
/// # Panics
/// No shut up clippy this function is mathematically impossible to panic
#[allow(clippy::arithmetic_side_effects)]
#[allow(clippy::indexing_slicing)]
pub(super) async fn handle_dnf(
    sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    f: impl Fn(&mut tokio::process::Command) -> &mut tokio::process::Command + Send,
) -> color_eyre::Result<()> {
    let mut cmd = tokio::process::Command::new("dnf5");
    f(&mut cmd);
    let mut output = cmd
        .stdout(std::process::Stdio::piped())
        .spawn()
        .wrap_err("fail to run `dnf5`")?;
    let mut stdout_lines = tokio::io::BufReader::new(output.stdout.take().unwrap()).split(b'\n');
    futures::try_join!(
        async move {
            while let Some(line) =
                (stdout_lines.next_segment().await).wrap_err("cannot read stdout")?
            {
                let mut it = line.iter().copied();
                if it.next().is_none_or(|c| c != b'[') {
                    continue;
                }

                let slash = if let Some(slash) = pu::search(&mut it, |c| {
                    (c == b' ' || c.is_ascii_digit()).then_some(c == b'/')
                }) {
                    slash + 1
                } else {
                    continue;
                };

                let end = if let Some(end) =
                    pu::search(&mut it, |c| c.is_ascii_digit().then_some(c == b']'))
                {
                    end + slash + 1
                } else {
                    continue;
                };

                if end == slash + 1 || slash == 1 {
                    continue;
                }
                pu::send_frac(&sender, &line[1..slash], &line[slash + 1..end]);
            }
            Ok(())
        },
        pu::wait_for("dnf5", output)
    )?;
    Ok(())
}

#[derive(Debug)]
pub(super) struct EnableRepo {
    /// denotes the path to the file, the toml object and whether it is modified.
    files: std::collections::HashMap<std::path::PathBuf, (toml_edit::DocumentMut, bool)>,
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
        if let Some((inner_toml, b)) = self
            .files
            .iter_mut()
            .find_map(|(_, (doc, b))| doc.get_mut(repo).map(|toml| (toml, b)))
        {
            inner_toml["enabled"] = toml_edit::value(1);
            *b = true;
            return Ok(());
        }
        if repo.starts_with("https://") || repo.starts_with("http://") {
            let content = Self::get_repo_from_url(repo).await?;
            let path = std::path::Path::new("/etc/yum.repos.d/")
                .join(repo.rsplit_once('/').expect("cannot split url with `/`").1);
            tokio::fs::write(&path, content)
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
    async fn get_repo_from_url(url: &str) -> color_eyre::Result<String> {
        tracing::debug!("Downloading repo file");
        let r = (super::REQWEST_CLIENT.get(url).send().await).wrap_err("cannot send request")?;
        let r = (r.error_for_status()).wrap_err("server fails to provide repo file")?;
        Ok(r.text().await?)
    }
    #[tracing::instrument]
    pub(super) async fn new() -> color_eyre::Result<Self> {
        let mut readdir = tokio::fs::read_dir("/etc/yum.repos.d/").await?;
        let mut files = std::collections::HashMap::new();
        while let Some(f) = readdir.next_entry().await? {
            files.insert(
                f.path(),
                (
                    std::fs::read_to_string(f.path())?
                        .parse()
                        .wrap_err("invalid toml file")
                        .with_note(|| format!("path: {}", f.path().display()))?,
                    false,
                ),
            );
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
                .map(|(p, (doc, _))| tokio::fs::write(p, doc.to_string())),
        )
        .await?;
        Ok(())
    }
}
