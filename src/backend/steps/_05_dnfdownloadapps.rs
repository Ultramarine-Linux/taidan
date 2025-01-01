use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadApps;
impl super::Step for DnfDownloadApps {
    #[allow(clippy::indexing_slicing)]
    #[tracing::instrument]
    async fn pre(
        &self,
        settings: &mut crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if settings.nointernet {
            return Ok(());
        }
        let mut it = settings.catalogue.iter().flat_map(|(cat_name, category)| {
            let app_list = CFG.catalogue.iter().find(|cat| &cat.name == cat_name);
            let app_list = &*app_list.expect("cannot find category").choices;
            let it = category.iter().map(move |(&appidx, opts)| {
                (app_list[appidx].actions.get_action(opts))
                    .map(Iterator::cloned)
                    .ok_or_else(|| {
                        eyre!("cannot get action").note(format!(
                            "appidx={appidx}, category={cat_name}, opts={opts:?}"
                        ))
                    })
            });
            it.flatten_ok()
        });
        it.try_for_each(|action| {
            action.map(|action| {
                settings.actions[action.as_int()].push(action.consume_inner_str());
            })
        })
    }
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::_11_installing::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        let mut enable_repo = super::super::dnf::EnableRepo::new().await?;
        // NOTE: we unfortunately cannot execute this in parallel because `enable_repo()` borrows
        // `enable_repo` mutably, and it's impossible to mutate the object in parallel safely.
        for repo in &settings.actions[0] {
            enable_repo.enable_repo(repo).await?;
        }
        enable_repo.save().await?;
        for copr in &settings.actions[4] {
            let dnf = tokio::process::Command::new("dnf")
                .args(["copr", "enable", copr])
                .status()
                .await
                .wrap_err("fail to run `dnf`")?;
            if !dnf.success() {
                return Err(eyre!("`dnf copr enable {copr}` failed")
                    .note(format!("Exit code: {:?}", dnf.code())));
            }
        }
        // as per jade's request, we need to remove firefox first for the browser category
        super::cmd("dnf5", &["rm", "-y", "--noninteractive", "firefox"])?;

        // run flatpak and dnf in parallel
        // this should be safe, supposedly they don't affect each other
        futures::future::try_join(
            super::super::flatpak::handle_flatpak(sender.clone(), |flatpak| {
                flatpak
                    .args(["install", "-y", "--noninteractive", "--no-deploy"])
                    .args(&settings.actions[2])
            }),
            super::super::dnf::handle_dnf(sender, |dnf| {
                dnf.args(["in", "-y", "--downloadonly"])
                    .args(&settings.actions[1])
            }),
        )
        .await?;
        Ok(())
    }
}
