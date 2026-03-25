use crate::{backend::i18n::I18nImf, prelude::*};

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadApps;
impl super::Step for DnfDownloadApps {
    #[allow(clippy::indexing_slicing)]
    #[tracing::instrument]
    async fn pre(
        &self,
        settings: &mut crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if CFG.taidan0.skip_dnf || settings.nointernet {
            return Ok(());
        }
        settings.actions[1].extend(
            super::_07_drivers_codecs::Codecs::codecs()
                .iter()
                .map(ToString::to_string),
        );

        if !settings.ims.is_empty() {
            let pkgs: &[&str] = match CFG.i18n.imf {
                I18nImf::Fcitx5 => &[
                    ["fcitx5-autostart", "fcitx5-qt5", "fcitx5-qt6"],
                    ["fcitx5-gtk", "kcm-fcitx5", "fcitx5-configtool"],
                ]
                .concat(),
                I18nImf::IBus => &["ibus-wayland", "ibus-qt", "ibus-gtk3", "ibus-gtk4"],
            };
            settings.actions[1].extend(pkgs.iter().map(|&s| s.to_owned()));
        }
        settings.actions[1].extend(
            (settings.ims.iter())
                .filter_map(|im| {
                    crate::backend::i18n::IMS
                        .values()
                        .find_map(|ims| ims.get(im))
                })
                .map(|im| im.get_pkg().to_owned())
                .unique(),
        );
        // Fedora didn't package `fcitx5-table-extra` properly, it's missing the
        // `fcitx5-chinese-addons` dependency, so we manually add it here.
        if settings.actions[1].contains(&"fcitx5-table-extra".to_owned())
            && !settings.actions[1].contains(&"fcitx5-chinese-addons".to_owned())
        {
            settings.actions[1].push("fcitx5-chinese-addons".to_owned());
        }

        Ok(())
    }
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if settings.nointernet {
            return Ok(());
        }
        let mut enable_repo = super::super::dnf::EnableRepo::new().await?;
        // NOTE: we unfortunately cannot execute this in parallel because `enable_repo()` borrows
        // `enable_repo` mutably, and it's impossible to mutate the object in parallel safely.
        for repo in &settings.actions[0] {
            enable_repo.enable_repo(repo).await?;
        }
        enable_repo.save().await?;
        for copr in &settings.actions[4] {
            crate::backend::pkexec("root", "dnf5", &["copr", "enable", "-y", copr]).await?;
        }

        match (
            settings.actions[1].is_empty(),
            settings.actions[2].is_empty(),
        ) {
            (true, true) => {}
            (true, false) => {
                super::super::flatpak::handle_flatpak(sender.clone(), |flatpak| {
                    flatpak
                        .args(["install", "-y", "--noninteractive", "--no-deploy"])
                        .args(&settings.actions[2])
                })
                .await?;
            }
            (false, true) => {
                super::super::dnf::handle_dnf(sender, |dnf| {
                    dnf.args(["in", "-y", "--downloadonly"])
                        .args(&settings.actions[1])
                })
                .await?;
            }
            (false, false) => {
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
            }
        }
        Ok(())
    }
}
