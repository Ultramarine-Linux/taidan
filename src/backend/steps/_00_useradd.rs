use color_eyre::eyre::OptionExt;
use tokio::io::AsyncWriteExt;

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserAdd;
impl super::Step for UserAdd {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if CFG.taidan0.systemd_homed {
            homed(settings).await
        } else {
            useradd(settings).await
        }
    }
}

async fn useradd(settings: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)
        .wrap_err("fail to encrypt password")
        .map_err(|e| e.note("fail: crypt_gensalt()"))?;
    let pass =
        xcrypt::crypt(&settings.passwd, &crypt_setting).wrap_err("fail to encrypt password")?;

    // ignore err in case recreate user
    _ = super::super::pkexec(
        "root",
        "useradd",
        &[
            "-p",
            &pass,
            "-c",
            &settings.fullname,
            "-m",
            &settings.username,
        ],
    )
    .await;
    super::super::pkexec("root", "usermod", &["-aG", "wheel", &settings.username]).await?;

    Ok(())
}

async fn homed(settings: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
    tracing::debug!("running pkexec homectl");
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", "root", "script", "-c"])
        .arg(format!(
            "homectl create {} -G wheel -c {} --enforce-password-policy=no",
            &settings.username, &settings.fullname
        ))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .wrap_err("fail to run `homectl`")?;
    let pass = &settings.passwd;
    let mut stdin = p.stdin.take().ok_or_eyre("stdin")?;
    _ = stdin
        .write_all(format!("{pass}\n{pass}\n").as_bytes())
        .await
        .inspect_err(|e| tracing::error!(?e, "cannot write to stdin"));
    _ = (stdin.flush().await).inspect_err(|e| tracing::error!(?e, "cannot flush stdin"));
    std::mem::drop(stdin);
    let Ok(r) = (p.wait().await).inspect_err(|e| tracing::error!(?e, "cannot wait homectl")) else {
        _ = p.kill().await;
        return Ok(());
    };
    if !r.success() {
        tracing::error!(code=?r.code(), "`homectl` failed");
    }
    Ok(())
}
