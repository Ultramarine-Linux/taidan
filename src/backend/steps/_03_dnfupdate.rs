#[derive(Clone, Copy, Debug, Default)]
pub struct DnfUpdate;
impl super::Step for DnfUpdate {
    fn run(&self, settings: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
        todo!()
    }
}
