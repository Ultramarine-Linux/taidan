use crate::{
    cfg::Desktop,
    home::{Home, HomeCmd},
    i18n,
    kconfig::KConfigEntry,
    prelude::*,
};

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let desktop = cfg.desktop();
    let Some(installed) = &with_installed_ims(settings, cfg, InstallPhase::After).await else {
        if let Err(e) = remove_stale_fcitx5_config(settings, cfg, desktop).await {
            tracing::warn!(?e, "cannot remove the Fcitx5 config of an earlier attempt");
        }
        return Ok(());
    };
    let res = async { apply(installed, &Home::user(settings, cfg).await?, desktop).await };
    if let Err(e) = res.await {
        tracing::warn!(?e, "cannot setup IMFs");
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallPhase {
    Before,
    After,
}

async fn with_installed_ims(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    phase: InstallPhase,
) -> Option<crate::settings::Settings> {
    if phase == InstallPhase::Before && !settings.nointernet && crate::steps::can_use_dnf(cfg).await
    {
        return (!settings.ims.is_empty()).then(|| settings.clone());
    }
    let mut ims = vec![];
    for name in &settings.ims {
        let Some(pkg) = i18n::find_im(name).and_then(|im| im.pkg(settings.imf)) else { continue };
        match tokio::process::Command::new("rpm").args(["-q", "--quiet", pkg]).status().await {
            Ok(status) if status.success() => ims.push(name.clone()),
            Ok(_) => tracing::info!(name, pkg, "input method isn't installed, not setting it up"),
            Err(e) => tracing::warn!(?e, "cannot run rpm"),
        }
    }
    (!ims.is_empty()).then(|| crate::settings::Settings { ims, ..settings.clone() })
}

const FCITX5_LAUNCHER: &str = "/usr/share/applications/fcitx5-wayland-launcher.desktop";

const FCITX5_PROFILE: &str = ".config/fcitx5/profile";

// https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg
fn kwinrc_select_fcitx5() -> [KConfigEntry; 2] {
    [
        KConfigEntry::value("kwinrc", "Wayland", "InputMethod", Some(FCITX5_LAUNCHER)),
        KConfigEntry::bool("kwinrc", "Wayland", "VirtualKeyboardEnabled", true),
    ]
}

const FCITX5_PROFILE_MARKER: &str = "# written by Taidan";

async fn remove_stale_fcitx5_config(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    desktop: Desktop,
) -> Res<()> {
    const SCRIPT: &str = r#"
        profile="$1/fcitx5/profile"
        [ "$(head -n 1 -- "$profile" 2>/dev/null)" = "$2" ] || exit 0
        rm -f -- "$profile"
        if [ "$3" = plasma ] && [ -e "$1/kwinrc" ] \
            && [ "$(kreadconfig6 --file "$1/kwinrc" --group Wayland --key InputMethod)" = "$4" ]; then
            kwriteconfig6 --file "$1/kwinrc" --group Wayland --key InputMethod --delete
            kwriteconfig6 --file "$1/kwinrc" --group Wayland --key VirtualKeyboardEnabled --delete
        fi
    "#;
    let home = Home::user(settings, cfg).await?;
    let plasma = if desktop == Desktop::Plasma { "plasma" } else { "" };
    let config = home.config();
    let config = config.to_string_lossy();
    let argv = ["sh", "-c", SCRIPT, "sh", &config, FCITX5_PROFILE_MARKER, plasma, FCITX5_LAUNCHER];
    home.run(vec![HomeCmd::new(argv)]).await
}

fn fcitx5_profile(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> String {
    let default_group_name = "Default"; // FIXME: this should be configurable? (was t!("default"))
    let var = kb_variant.as_ref().map(|variant| format!("-{variant}")).unwrap_or_default();
    let mut buf = format!(
        "\
            {FCITX5_PROFILE_MARKER}\n\
            [Groups/0]\n\
            # Group Name\n\
            Name={default_group_name}\n\
            # Layout\n\
            Default Layout={kb_layout}{var}\n\
            # Default Input Method\n\
            DefaultIM=keyboard-{kb_layout}{var}\n\n\
            [Groups/0/Items/0]\n\
            # Name\n\
            Name=keyboard-{kb_layout}{var}\n\
            # Layout\n\
            Layout=\n\n\
        ",
    );

    let latin_fallback = i18n::XkbKeymap::new(kb_layout, kb_variant.as_deref())
        .has_latin_fallback()
        .then_some("keyboard-us");
    // after the input methods, since fcitx5's Ctrl+Space toggles between the layout and the first
    let refs = ims.iter().filter_map(|im| i18n::find_im(im)?.fcitx5_ref).chain(latin_fallback);
    for (i, im) in refs.enumerate() {
        buf.push_str(&format!(
            "\
                [Groups/0/Items/{}]\n\
                # Name\n\
                Name={im}\n\
                # Layout\n\
                Layout=\n\n\
            ",
            i + 1,
        ));
    }

    buf.push_str(&format!("[GroupOrder]\n0={default_group_name}\n"));
    buf
}

fn im_cmds(settings: &crate::settings::Settings, home: &Home, desktop: Desktop) -> Vec<HomeCmd> {
    match settings.imf {
        i18n::I18nImf::IBus if !settings.ims.is_empty() && !has_ibus_engine(settings) => vec![],
        i18n::I18nImf::IBus => home
            .session(ibus_dconf_writes(settings))
            .map(HomeCmd::best_effort)
            .into_iter()
            .collect(),
        i18n::I18nImf::Fcitx5 if settings.ims.is_empty() => vec![home.remove_file(FCITX5_PROFILE)],
        i18n::I18nImf::Fcitx5 => {
            let mut cmds = vec![home.write_file(FCITX5_PROFILE, fcitx5_profile(settings))];
            if desktop == Desktop::Plasma {
                cmds.extend(home.kconfig(&kwinrc_select_fcitx5()));
            }
            cmds
        }
    }
}

async fn apply(settings: &crate::settings::Settings, home: &Home, desktop: Desktop) -> Res<bool> {
    let cmds = im_cmds(settings, home, desktop);
    let wrote = !cmds.is_empty();
    home.run(cmds).await?;
    Ok(wrote)
}

pub(in crate::steps) async fn apply_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &Home,
) -> Res<bool> {
    match &with_installed_ims(settings, cfg, InstallPhase::Before).await {
        Some(installed) => apply(installed, skel, cfg.desktop()).await,
        None => Ok(false),
    }
}

const EMPTY_STRING_ARRAY_GVARIANT: &str = "@as []";

fn has_ibus_engine(settings: &crate::settings::Settings) -> bool {
    settings.ims.iter().any(|s| i18n::find_im(s).is_some_and(|im| im.ibus_ref.is_some()))
}

fn ibus_dconf_writes(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> Vec<Vec<String>> {
    let engines: Vec<_> = ims.iter().filter_map(|s| i18n::find_im(s)?.ibus_ref).collect();
    let order = if engines.is_empty() {
        EMPTY_STRING_ARRAY_GVARIANT.to_owned()
    } else {
        format!("[{}]", engines.iter().map(|r| format!("'{r}'")).collect::<Vec<_>>().join(", "))
    };

    let layout = if kb_layout.is_empty() { "us" } else { kb_layout };
    let sources = i18n::XkbKeymap::new(layout, kb_variant.as_deref())
        .gnome_sources()
        .into_iter()
        .chain(engines.iter().map(|r| format!("('ibus', '{r}')")))
        .collect::<Vec<_>>()
        .join(", ");

    let write = |key: &str, value: String| {
        ["dbus-run-session", "--", "dconf", "write", key]
            .iter()
            .map(|&s| s.to_owned())
            .chain([value])
            .collect()
    };
    vec![
        write("/desktop/ibus/general/use-system-keyboard-layout", "true".to_owned()),
        write("/desktop/ibus/general/engines-order", order.clone()),
        write("/desktop/ibus/general/preload-engines", order),
        // GNOME and Budgie ignore `preload-engines` and drive IBus from `input-sources`
        write("/org/gnome/desktop/input-sources/sources", format!("[{sources}]")),
    ]
}

pub async fn fix_homed(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) {
    if !cfg.taidan0.systemd_homed || settings.ims.is_empty() {
        return;
    }
    let in_skel = with_installed_ims(settings, cfg, InstallPhase::Before).await;
    let installed = with_installed_ims(settings, cfg, InstallPhase::After).await;
    if in_skel.as_ref().map(|s| &s.ims) == installed.as_ref().map(|s| &s.ims) {
        return;
    }
    tracing::info!(?installed, "some input methods didn't install, fixing the home");
    let settings =
        &installed.unwrap_or_else(|| crate::settings::Settings { ims: vec![], ..settings.clone() });
    let res = async { apply(settings, &Home::user(settings, cfg).await?, cfg.desktop()).await };
    if let Err(e) = res.await {
        tracing::warn!(?e, "cannot fix the input methods in the systemd-homed home");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_lists_ims() {
        let settings = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Pinyin".into(), "bogus".into(), "Mozc".into()],
            ..Default::default()
        };
        let profile = fcitx5_profile(&settings);
        assert!(profile.starts_with(&format!("{FCITX5_PROFILE_MARKER}\n[Groups/0]\n")));
        assert!(profile.contains("Default Layout=us\n"));
        assert!(profile.contains("[Groups/0/Items/1]\n# Name\nName=pinyin\n"));
        assert!(profile.contains("[Groups/0/Items/2]\n# Name\nName=mozc\n"));
        assert!(!profile.contains("Items/3"));
    }

    #[test]
    fn retry_replaces_input_method_choices_even_offline() {
        let first = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Pinyin".into()],
            nointernet: true,
            ..Default::default()
        };
        let retry = crate::settings::Settings { ims: vec!["Mozc".into()], ..first.clone() };
        let first_profile = fcitx5_profile(&first);
        let retry_profile = fcitx5_profile(&retry);
        assert!(first_profile.contains("Name=pinyin\n"));
        assert!(retry_profile.contains("Name=mozc\n"));
        assert!(!retry_profile.contains("Name=pinyin\n"));
    }

    #[test]
    fn ibus_engines_go_to_gnome_input_sources() {
        let settings = crate::settings::Settings {
            kb_layout: "de".into(),
            kb_variant: Some("nodeadkeys".into()),
            ims: vec!["Pinyin".into(), "bogus".into(), "Mozc".into()],
            ..Default::default()
        };
        let script = ibus_dconf_writes(&settings).concat().join(" ");
        let refs: Vec<_> = ["Pinyin", "Mozc"]
            .iter()
            .map(|im| i18n::find_im(im).unwrap().ibus_ref.unwrap())
            .collect();
        let sources = format!(
            "/org/gnome/desktop/input-sources/sources [('xkb', 'de+nodeadkeys'), ('ibus', '{}'), ('ibus', '{}')]",
            refs[0], refs[1],
        );
        assert!(script.contains(&sources), "{script}");
        let preload = format!("preload-engines ['{}', '{}']", refs[0], refs[1]);
        assert!(script.contains(&preload), "{script}");
    }

    #[test]
    fn non_latin_layouts_get_us_too() {
        let settings = crate::settings::Settings {
            kb_layout: "ru".into(),
            ims: vec!["Mozc".into()],
            ..Default::default()
        };
        let profile = fcitx5_profile(&settings);
        assert!(profile.contains("[Groups/0/Items/1]\n# Name\nName=mozc\n"));
        assert!(profile.contains("[Groups/0/Items/2]\n# Name\nName=keyboard-us\n"));
        let script = ibus_dconf_writes(&settings).concat().join(" ");
        assert!(script.contains("[('xkb', 'ru'), ('xkb', 'us'), ('ibus', "), "{script}");
    }

    #[test]
    fn no_ibus_engines_writes_nothing() {
        let skel = Home::Skel("/skel".into());
        let settings = crate::settings::Settings {
            ims: vec!["bogus".into()],
            imf: i18n::I18nImf::IBus,
            ..Default::default()
        };
        assert!(im_cmds(&settings, &skel, Desktop::Gnome).is_empty());
        let none = crate::settings::Settings { ims: vec![], ..settings };
        assert_eq!(im_cmds(&none, &skel, Desktop::Gnome).len(), 1);
    }

    #[test]
    fn fcitx5_on_plasma_also_selects_it_in_kwin() {
        let skel = Home::Skel("/skel".into());
        let settings = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Mozc".into()],
            ..Default::default()
        };
        assert_eq!(im_cmds(&settings, &skel, Desktop::Plasma).len(), 3);
        assert_eq!(im_cmds(&settings, &skel, Desktop::Gnome).len(), 1);
        let none = crate::settings::Settings { ims: vec![], ..settings };
        assert_eq!(im_cmds(&none, &skel, Desktop::Plasma), [skel.remove_file(FCITX5_PROFILE)]);
    }
}
