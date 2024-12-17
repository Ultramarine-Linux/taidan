use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetTime;
impl super::Step for SetTime {
    #[tracing::instrument]
    async fn run(
        &self,
        _: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        let enable_timesyncd = std::process::Command::new("systemctl")
            .args(["enable", "systemd-timesyncd.service", "--now"])
            .status()
            .wrap_err("fail to run `systemctl`")?;
        if !enable_timesyncd.success() {
            return Err(eyre!("cannot enable `systemd-timesyncd.service`")
                .note(format!("Exit code: {:?}", enable_timesyncd.code())));
        }

        Ok(())
    }
}
