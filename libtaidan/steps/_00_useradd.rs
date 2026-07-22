use tokio::io::AsyncWriteExt;

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserAdd;
impl super::Step for UserAdd {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if cfg.taidan0.systemd_homed { homed(settings).await } else { useradd(settings).await }
    }
}

async fn useradd(settings: &crate::settings::Settings) -> Res<()> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)?;
    let pass = xcrypt::crypt(&settings.passwd, &crypt_setting)?;

    // ignore err in case recreate user
    _ = super::super::pkexec(
        "root",
        "useradd",
        &["-p", &pass, "-c", &settings.fullname, "-m", &settings.username],
    )
    .await;
    super::super::pkexec("root", "usermod", &["-aG", "wheel", &settings.username]).await?;

    Ok(())
}

async fn homed(settings: &crate::settings::Settings) -> Res<()> {
    tracing::debug!("running pkexec homectl");
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", "root", "script", "-c"])
        .arg(format!(
            "homectl create {} -G wheel -c {} --enforce-password-policy=no",
            &settings.username, &settings.fullname
        ))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("pkexec", e))?;
    let pass = &settings.passwd;
    let mut stdin = p.stdin.take().expect("why is there no stdin?");
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
