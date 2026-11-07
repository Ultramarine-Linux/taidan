use crate::{i18n::I18nImf, prelude::*};

#[derive(Clone, Copy, Debug, Default)]
pub struct DnfDownloadApps;
impl super::Step for DnfDownloadApps {
    #[allow(clippy::indexing_slicing)]
    #[tracing::instrument(skip(settings))]
    async fn pre<C: crate::Callback>(
        &self,
        settings: &mut crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if cfg.taidan0.skip_dnf || settings.nointernet {
            return Ok(());
        }
        if settings.install_codecs_drivers {
            settings.actions[1].extend(
                super::_07_drivers_codecs::Codecs::codecs().iter().map(ToString::to_string),
            );
        }

        if !settings.ims.is_empty() {
            let pkgs: &[&str] = match settings.imf {
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
                .filter_map(|im| crate::i18n::IMS.values().find_map(|ims| ims.get(im)))
                .map(|im| im.get_pkg(settings.imf).to_owned())
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

    #[tracing::instrument(skip(settings, callback))]

    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &C,
    ) -> Res<()> {
        if cfg.taidan0.skip_dnf || settings.nointernet {
            return Ok(());
        }
        // one at a time, since each `dnf5 config-manager` rewrites the same config files
        for repo in &settings.actions[0] {
            super::super::dnf::enable_repo(repo).await?;
        }
        for copr in &settings.actions[4] {
            crate::pkexec("root", "dnf5", &["copr", "enable", "-y", copr]).await?;
        }

        match (settings.actions[1].is_empty(), settings.actions[2].is_empty()) {
            (true, true) => {}
            (true, false) => {
                super::super::flatpak::handle_flatpak(callback, |flatpak| {
                    flatpak
                        .args(["install", "-y", "--noninteractive", "--no-deploy"])
                        .args(&settings.actions[2])
                })
                .await?;
            }
            (false, true) => {
                super::super::dnf::handle_dnf(callback, |dnf| {
                    dnf.args(["in", "-y", "--downloadonly"]).args(&settings.actions[1])
                })
                .await?;
            }
            (false, false) => {
                // run flatpak and dnf in parallel
                // this should be safe, supposedly they don't affect each other
                futures::future::try_join(
                    super::super::flatpak::handle_flatpak(callback, |flatpak| {
                        flatpak
                            .args(["install", "-y", "--noninteractive", "--no-deploy"])
                            .args(&settings.actions[2])
                    }),
                    super::super::dnf::handle_dnf(callback, |dnf| {
                        dnf.args(["in", "-y", "--downloadonly"]).args(&settings.actions[1])
                    }),
                )
                .await?;
            }
        }
        Ok(())
    }
}
