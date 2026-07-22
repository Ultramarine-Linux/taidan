use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadUpdate;
impl super::Step for DnfDownloadUpdate {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if settings.nointernet {
            return Ok(());
        }
        // super::super::dnf::handle_dnf(sender, |dnf| dnf.args(["up", "-y", "--downloadonly"])).await
        Ok(())
    }
}
