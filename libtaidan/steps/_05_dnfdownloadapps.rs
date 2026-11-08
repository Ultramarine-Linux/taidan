use crate::{i18n::I18nImf, prelude::*};

/// how dnf installs the extras (codecs, input methods, the language pack), here and in
/// `DnfInstallApps`. None of them is worth failing the installation over: `--skip-unavailable`
/// skips packages that aren't in the repos, e.g. an input method the distro doesn't package, and
/// `--skip-broken` the ones whose dependencies don't resolve. dnf5 split the two, so neither
/// implies the other, see <https://dnf5.readthedocs.io/en/latest/changes_from_dnf4.7.html>
pub(super) const DNF_EXTRAS_ARGS: &[&str] = &["in", "-y", "--skip-broken", "--skip-unavailable"];

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
        if settings.nointernet || !super::can_use_dnf(cfg).await {
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
        if settings.nointernet {
            return Ok(());
        }
        // flatpaks still work without dnf, everything else here needs it
        let dnf = super::can_use_dnf(cfg).await;
        if dnf {
            // one at a time, since each `dnf5 config-manager` rewrites the same config files
            for repo in &settings.actions[0] {
                super::super::dnf::enable_repo(repo).await?;
            }
            for copr in &settings.actions[4] {
                // it'd be an option
                if copr.starts_with('-') {
                    return Err(Err::Message(format!("invalid copr `{copr}`")));
                }
                crate::pkexec("root", "dnf5", &["copr", "enable", "-y", copr]).await?;
            }
        }
        // RPM Fusion's `@multimedia` breaks for a while whenever it lags behind a Fedora update,
        // see `DNF_EXTRAS_ARGS`
        let pkgs: &[String] = if dnf { &settings.actions[1] } else { &[] };

        match (pkgs.is_empty(), settings.actions[2].is_empty()) {
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
                    dnf.args(DNF_EXTRAS_ARGS).arg("--downloadonly").args(pkgs)
                })
                .await?;
            }
            (false, false) => {
                // run flatpak and dnf in parallel
                // this should be safe, supposedly they don't affect each other
                super::join_both(
                    super::super::flatpak::handle_flatpak(callback, |flatpak| {
                        flatpak
                            .args(["install", "-y", "--noninteractive", "--no-deploy"])
                            .args(&settings.actions[2])
                    }),
                    super::super::dnf::handle_dnf(callback, |dnf| {
                        dnf.args(DNF_EXTRAS_ARGS).arg("--downloadonly").args(pkgs)
                    }),
                )
                .await?;
            }
        }
        Ok(())
    }
}
