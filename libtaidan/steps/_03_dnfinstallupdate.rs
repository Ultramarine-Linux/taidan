use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfInstallUpdate;
impl super::Step for DnfInstallUpdate {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if settings.nointernet {
            return Ok(());
        }
        // super::super::dnf::handle_dnf(sender, |dnf| dnf.args(["up", "-y"])).await
        Ok(())
    }
}
