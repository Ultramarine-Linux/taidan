use super::super::i18n;
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetupImf;
impl super::Step for SetupImf {
    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if cfg.taidan0.systemd_homed {
            return Ok(());
        }
        let Some(settings) = &with_installed_ims(settings, cfg, InstallPhase::After).await else {
            if let Err(e) = remove_stale_fcitx5_config(settings).await {
                tracing::warn!(?e, "cannot remove the Fcitx5 config of an earlier attempt");
            }
            return Ok(());
        };
        if let Err(e) = match settings.imf {
            i18n::I18nImf::IBus => write_ibus_profile(settings).await,
            i18n::I18nImf::Fcitx5 => write_fcitx5_profile(settings).await,
        } {
            tracing::warn!(?e, "cannot setup IMFs");
        }
        Ok(())
    }
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
    if phase == InstallPhase::Before && !settings.nointernet && super::can_use_dnf(cfg).await {
        return (!settings.ims.is_empty()).then(|| settings.clone());
    }
    let mut ims = vec![];
    for name in &settings.ims {
        let im = i18n::find_im(name);
        let pkg = match settings.imf {
            i18n::I18nImf::Fcitx5 => im.and_then(|im| im.fcitx5_pkg),
            i18n::I18nImf::IBus => im.and_then(|im| im.ibus_pkg),
        };
        let Some(pkg) = pkg else { continue };
        match tokio::process::Command::new("rpm").args(["-q", "--quiet", pkg]).status().await {
            Ok(status) if status.success() => ims.push(name.clone()),
            Ok(_) => tracing::info!(name, pkg, "input method isn't installed, not setting it up"),
            Err(e) => tracing::warn!(?e, "cannot run rpm"),
        }
    }
    (!ims.is_empty()).then(|| crate::settings::Settings { ims, ..settings.clone() })
}

const FCITX5_LAUNCHER: &str = "/usr/share/applications/fcitx5-wayland-launcher.desktop";

// https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg
const KWINRC_SELECT_FCITX5_ARGS: [[&str; 6]; 2] = [
    ["--group", "Wayland", "--key", "InputMethod", "--", FCITX5_LAUNCHER],
    ["--group", "Wayland", "--key", "VirtualKeyboardEnabled", "--type=bool", "true"],
];

const FCITX5_PROFILE_MARKER: &str = "# written by Taidan";

async fn remove_stale_fcitx5_config(settings: &crate::settings::Settings) -> Res<()> {
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
    let username = &settings.username;
    let config = crate::a11y::home_of(username).await?.join(".config");
    let plasma = if crate::a11y::is_plasma() { "plasma" } else { "" };
    let argv = [
        "sh",
        "-c",
        SCRIPT,
        "sh",
        &config.to_string_lossy(),
        FCITX5_PROFILE_MARKER,
        plasma,
        FCITX5_LAUNCHER,
    ];
    crate::as_user(username, argv.map(str::to_owned).to_vec(), None).await
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

fn write_file_argv(path: String) -> Vec<String> {
    let script = r#"mkdir -p "${1%/*}" && exec cat > "$1""#;
    ["sh", "-c", script, "sh"].map(str::to_owned).into_iter().chain([path]).collect()
}

async fn write_fcitx5_profile(settings: &crate::settings::Settings) -> Res<()> {
    let username = &settings.username;
    let config = crate::a11y::home_of(username).await?.join(".config");
    let profile = config.join("fcitx5/profile").to_string_lossy().into_owned();
    crate::as_user(username, write_file_argv(profile), Some(fcitx5_profile(settings).as_bytes()))
        .await
        .wrap_msg("cannot write ~/.config/fcitx5/profile")?;

    if !crate::a11y::is_plasma() {
        return Ok(());
    }
    let kwinrc = config.join("kwinrc");
    let kwinrc = kwinrc.to_string_lossy();
    for entry in KWINRC_SELECT_FCITX5_ARGS {
        let mut argv = vec!["kwriteconfig6", "--file", &kwinrc];
        argv.extend(entry);
        let argv = argv.into_iter().map(str::to_owned).collect();
        crate::as_user(username, argv, None).await.wrap_msg("cannot select Fcitx5 in kwinrc")?;
    }
    Ok(())
}

pub async fn write_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(settings) = &with_installed_ims(settings, cfg, InstallPhase::Before).await else {
        return Ok(false);
    };
    if settings.imf.is_ibus() {
        return write_ibus_to_skel(settings, skel).await;
    }
    let config = skel.join(".config");
    tokio::fs::create_dir_all(config.join("fcitx5")).await.map_err(Err::Io)?;
    tokio::fs::write(config.join("fcitx5/profile"), fcitx5_profile(settings))
        .await
        .map_err(Err::Io)?;
    if crate::a11y::is_plasma() {
        let kwinrc = config.join("kwinrc");
        let kwinrc = kwinrc.to_string_lossy();
        for entry in KWINRC_SELECT_FCITX5_ARGS {
            let mut args = vec!["--file", &*kwinrc];
            args.extend(entry);
            super::acmd("kwriteconfig6", &args).await?;
        }
    }
    Ok(true)
}

const EMPTY_STRING_ARRAY_GVARIANT: &str = "@as []";

fn ibus_session_if_any_engine(settings: &crate::settings::Settings) -> Option<Vec<String>> {
    let any = settings.ims.iter().any(|s| i18n::find_im(s).is_some_and(|im| im.ibus_ref.is_some()));
    any.then(|| ibus_dconf_writes(settings)).and_then(crate::a11y::one_session)
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
    let home = match crate::a11y::home_of(&settings.username).await {
        Ok(home) => home,
        Err(e) => return tracing::warn!(?e, "cannot find the user's home"),
    };
    let cmds = if settings.imf.is_ibus() {
        let Some(cmd) = crate::a11y::one_session(ibus_dconf_writes(settings)) else { return };
        let mut env = vec!["env".to_owned()];
        env.extend(crate::a11y::in_home(&home, cmd));
        vec![(env, None)]
    } else {
        let profile = home.join(".config/fcitx5/profile").to_string_lossy().into_owned();
        if settings.ims.is_empty() {
            vec![(vec!["rm".to_owned(), "-f".to_owned(), profile], None)]
        } else {
            vec![(write_file_argv(profile), Some(fcitx5_profile(settings)))]
        }
    };
    crate::a11y::run_in_home(settings, cfg, cmds).await;
}

async fn write_ibus_to_skel(
    settings: &crate::settings::Settings,
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(cmd) = ibus_session_if_any_engine(settings) else { return Ok(false) };
    tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
    let env = crate::a11y::in_home(skel, cmd);
    let env = env.iter().map(String::as_str).collect::<Vec<_>>();
    if let Err(e) = super::acmd("env", &env).await {
        tracing::warn!(?e, "cannot set the IBus engines");
        return Ok(false);
    }
    Ok(true)
}

async fn write_ibus_profile(settings: &crate::settings::Settings) -> Res<()> {
    let Some(cmd) = ibus_session_if_any_engine(settings) else { return Ok(()) };
    crate::as_user(&settings.username, cmd, None).await.wrap_msg("failed to set the IBus engines")
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
        let settings =
            crate::settings::Settings { ims: vec!["bogus".into()], ..Default::default() };
        assert!(ibus_session_if_any_engine(&settings).is_none());
    }
}
