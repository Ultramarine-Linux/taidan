use crate::{i18n::I18nImf, prelude::*};

// dnf5 split these two, neither implies the other
// https://dnf5.readthedocs.io/en/latest/changes_from_dnf4.7.html
pub(crate) const DNF_EXTRAS_ARGS: &[&str] = &["in", "-y", "--skip-broken", "--skip-unavailable"];

pub(crate) const fn imf_packages(imf: I18nImf) -> &'static [&'static str] {
    match imf {
        I18nImf::Fcitx5 => &[
            "fcitx5-autostart",
            "fcitx5-qt5",
            "fcitx5-qt6",
            "fcitx5-gtk",
            "kcm-fcitx5",
            "fcitx5-configtool",
        ],
        I18nImf::IBus => &["ibus-wayland", "ibus-qt", "ibus-gtk3", "ibus-gtk4"],
    }
}

pub(super) async fn run(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    callback: &impl crate::Callback,
) -> Res<()> {
    if settings.nointernet {
        return Ok(());
    }
    let dnf = super::can_use_dnf(cfg).await;
    if dnf {
        for repo in &settings.actions[0] {
            super::super::dnf::enable_repo(repo).await?;
        }
        for copr in &settings.actions[4] {
            crate::root(crate::Verb::CoprEnable { copr: copr.clone() })
                .await
                .wrap_message(|_| format!("cannot enable copr `{copr}`"))?;
        }
    }
    let pkgs: &[String] = if dnf { &settings.actions[1] } else { &[] };
    super::flatpaks_and_packages(callback, &settings.actions[2], pkgs, true).await
}

pub(super) async fn pre(
    settings: &mut crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Res<()> {
    if settings.nointernet || !super::can_use_dnf(cfg).await {
        return Ok(());
    }
    if settings.install_codecs_drivers {
        settings.actions[1]
            .extend(super::_07_drivers_codecs::Codecs::codecs().iter().map(ToString::to_string));
    }

    if !settings.ims.is_empty() {
        settings.actions[1].extend(imf_packages(settings.imf).iter().map(|&s| s.to_owned()));
    }
    settings.actions[1].extend(
        settings
            .ims
            .iter()
            .filter_map(|im| crate::i18n::find_im(im)?.pkg(settings.imf))
            .map(str::to_owned)
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
