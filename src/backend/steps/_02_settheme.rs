#[derive(Clone, Copy, Debug, Default)]
pub struct SetTheme;
impl super::Step for SetTheme {
    fn run(&self, settings: &crate::backend::settings::Settings) -> color_eyre::Result<()> {
        todo!()
    }
}
