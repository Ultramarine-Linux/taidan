use color_eyre::{
    eyre::{eyre, Context},
    Section,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SetTime;
impl super::Step for SetTime {
    fn run(&self, _: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
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
