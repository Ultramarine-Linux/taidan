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
}
