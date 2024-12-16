#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstall;
impl super::Step for DnfInstall {
    fn run(&self, settings: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
        todo!()
    }
}
