use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserAdd;
impl super::Step for UserAdd {
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)
            .wrap_err("fail to encrypt password")
            .map_err(|e| e.note("fail: crypt_gensalt()"))?;
        let pass =
            xcrypt::crypt(&settings.passwd, &crypt_setting).wrap_err("fail to encrypt password")?;

        let useradd = std::process::Command::new("useradd")
            .arg("-p")
            .arg(pass)
            .arg("-m") // --create-home
            .arg(&settings.username)
            .status()
            .wrap_err("fail to run `useradd`")?;
        if !useradd.success() {
            return Err(eyre!("`useradd` failed").note(format!("Exit code: {:?}", useradd.code())));
        }

        let usermod = std::process::Command::new("usermod")
            .arg("-aG")
            .arg("wheel")
            .arg(&settings.username)
            .status()
            .wrap_err("fail to run `usermod`")?;
        if !usermod.success() {
            return Err(eyre!("`usermod` failed").note(format!("Exit code: {:?}", usermod.code())));
        }

        Ok(())
    }
}
