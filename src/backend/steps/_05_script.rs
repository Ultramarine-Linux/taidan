use crate::prelude::*;

const SETTINGS_FILE: &str = "/tmp/taidan_settings.json";

#[derive(Clone, Copy, Debug, Default)]
pub struct Script;
impl super::Step for Script {
    #[tracing::instrument]
    async fn pre(
        &self,
        settings: &mut crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        let f = std::fs::File::create(SETTINGS_FILE).wrap_err(const_format::formatcp!(
            "cannot create file `{SETTINGS_FILE}`"
        ))?;
        serde_json::to_writer(f, settings)?;
        // `f` is moved into to_write() and is therefore dropped by now
        let mut process = tokio::process::Command::new("bash");
        process
            .arg("/usr/share/taidan/oobe.sh")
            .arg(SETTINGS_FILE)
            .stdout(std::process::Stdio::piped());
        let stdout = process.output().await?.stdout;
        for line in String::from_utf8_lossy(&stdout).as_ref().lines() {
            if let Some(pkg) = line.strip_prefix("pkg: ") {
                settings.actions[1].push(pkg.to_owned());
            }
        }
        Ok(())
    }
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        let sett = serde_json::to_vec(settings)?;
        for (&on, tweak) in settings.tweaks.iter().zip(&*crate::backend::tweaks::TWEAKS) {
            tweak.run(&sett, on).await;
        }
        Ok(())
    }
}
