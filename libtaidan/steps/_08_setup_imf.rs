use tokio::io::AsyncWriteExt;

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
        // written into the skeleton the home was created from, see `write_to_skel`
        if settings.ims.is_empty() || cfg.taidan0.systemd_homed {
            return Ok(());
        }
        let Some(settings) = &with_installed_ims(settings, cfg, true).await else { return Ok(()) };
        if let Err(e) = match settings.imf {
            i18n::I18nImf::IBus => write_ibus_profile(settings).await,
            i18n::I18nImf::Fcitx5 => write_fcitx5_profile(settings).await,
        } {
            tracing::warn!(?e, "cannot setup IMFs");
        }
        Ok(())
    }
}

/// `settings` with only the input methods that are installed, [`None`] if that leaves none. Ones
/// installed here may still be missing, as dnf skips broken packages, and offline or without dnf
/// only the image's are there. Before the installation (`installed` false, e.g. for homed's
/// skeleton), the ones dnf is about to install count too
async fn with_installed_ims(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    installed: bool,
) -> Option<crate::settings::Settings> {
    if !installed && !settings.nointernet && super::can_use_dnf(cfg).await {
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

/// `kwriteconfig6` arguments, after `--file`, that select Fcitx5 as KWin's input method
/// https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg?ref_type=heads
const KWINRC_FCITX5: [[&str; 6]; 2] = [
    [
        "--group",
        "Wayland",
        "--key",
        "InputMethod",
        "--",
        "/usr/share/applications/fcitx5-wayland-launcher.desktop",
    ],
    ["--group", "Wayland", "--key", "VirtualKeyboardEnabled", "--type=bool", "true"],
];

/// `~/.config/fcitx5/profile` with the keyboard layout and the chosen input methods
fn fcitx5_profile(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> String {
    let default_group_name = "Default"; // FIXME: this should be configurable? (was t!("default"))
    let var = kb_variant.as_ref().map(|variant| format!("-{variant}")).unwrap_or_default();
    let mut buf = format!(
        "\
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

    // a layout that can't type ASCII gets US English next to it, see `XkbKeymap::new`
    let latin = i18n::XkbKeymap::new(kb_layout, kb_variant.as_deref())
        .has_latin_fallback()
        .then_some("keyboard-us");
    // after the input methods, fcitx5's Ctrl+Space toggles between the layout and the first of
    // them
    let refs = ims.iter().filter_map(|im| i18n::find_im(im)?.fcitx5_ref).chain(latin);
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

/// writes `content` to `path` as `user`, replacing what was there, so the file is theirs
async fn write_as(user: &str, path: &str, content: &[u8]) -> Res<()> {
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", user, "tee", "--", path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("pkexec", e))?;
    if let Some(mut stdin) = p.stdin.take() {
        (stdin.write_all(content).await.map_err(Err::Io))
            .wrap_msg("cannot write to pkexec tee stdin")?;
    }

    let p = (p.wait_with_output().await)
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("cannot tee")?;

    if !p.status.success() {
        return Err(Err::ProgramFail {
            name: "pkexec (tee)",
            output: String::from_utf8_lossy(&p.stderr).to_string(),
            rc: p.status.code(),
        })
        .wrap_message(|_| format!("failed to write to {path}"));
    }
    Ok(())
}

async fn write_fcitx5_profile(settings: &crate::settings::Settings) -> Res<()> {
    let username = &settings.username;
    let config = crate::a11y::home_of(username).await?.join(".config");
    let fcitx5 = config.join("fcitx5");
    crate::pkexec(username, "mkdir", [std::ffi::OsStr::new("-p"), fcitx5.as_os_str()])
        .await
        .wrap_msg("cannot create ~/.config/fcitx5/")?;

    let profile_path = fcitx5.join("profile");
    write_as(username, &profile_path.to_string_lossy(), fcitx5_profile(settings).as_bytes())
        .await?;

    // KWin is only the user's compositor on Plasma
    if !crate::a11y::is_plasma() {
        return Ok(());
    }
    let kwinrc = config.join("kwinrc");
    let kwinrc = kwinrc.to_string_lossy();
    for entry in KWINRC_FCITX5 {
        // as the user, so the file they get is theirs
        let mut args = vec!["-u", username, "--", "kwriteconfig6", "--file", &kwinrc];
        args.extend(entry);
        crate::root("runuser", &args).await.wrap_msg("cannot select Fcitx5 in kwinrc")?;
    }
    Ok(())
}

/// writes the input method config into `skel`, a copy of `/etc/skel` the new user's home is made
/// from, for systemd-homed, where the home isn't mounted when [`SetupImf`] runs
///
/// # Errors
/// - cannot write the files
pub async fn write_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &std::path::Path,
) -> Res<bool> {
    if settings.ims.is_empty() {
        return Ok(false);
    }
    let Some(settings) = &with_installed_ims(settings, cfg, false).await else { return Ok(false) };
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
        for entry in KWINRC_FCITX5 {
            let mut args = vec!["--file", &*kwinrc];
            args.extend(entry);
            super::acmd("kwriteconfig6", &args).await?;
        }
    }
    Ok(true)
}

/// the `dconf write`s that hand IBus the chosen engines, for [`crate::a11y::one_session`] to run
/// through one D-Bus,
/// [`None`] if none of them have an IBus engine. IBus reads `preload-engines`, but GNOME (and
/// Budgie) drive IBus from their own `input-sources`, ignoring it, so the engines go in there too
/// after the layout. Arguments rather than a shell script, the layout comes from the settings
fn ibus_dconf_writes(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> Option<Vec<Vec<String>>> {
    let engines: Vec<_> = ims.iter().filter_map(|s| i18n::find_im(s)?.ibus_ref).collect();
    if engines.is_empty() {
        return None;
    }
    let order = engines.iter().map(|r| format!("'{r}'")).collect::<Vec<_>>().join(", ");

    // without an xkb source first GNOME would type through an IBus engine all the time
    let layout = if kb_layout.is_empty() { "us" } else { kb_layout };
    let sources = (i18n::XkbKeymap::new(layout, kb_variant.as_deref()).gnome_sources().into_iter())
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
    Some(vec![
        write("/desktop/ibus/general/use-system-keyboard-layout", "true".to_owned()),
        write("/desktop/ibus/general/engines-order", format!("[{order}]")),
        write("/desktop/ibus/general/preload-engines", format!("[{order}]")),
        write("/org/gnome/desktop/input-sources/sources", format!("[{sources}]")),
    ])
}

/// [`ibus_dconf_writes`] into the dconf database in `skel`, through a D-Bus of its own like the
/// theme's `gsettings`
async fn write_ibus_to_skel(
    settings: &crate::settings::Settings,
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(cmd) = ibus_dconf_writes(settings).and_then(crate::a11y::one_session) else {
        return Ok(false);
    };
    tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
    let env = crate::a11y::in_home(skel, cmd);
    let env = env.iter().map(String::as_str).collect::<Vec<_>>();
    // not worth losing the rest of the skeleton over
    if let Err(e) = super::acmd("env", &env).await {
        tracing::warn!(?e, "cannot set the IBus engines");
        return Ok(false);
    }
    Ok(true)
}

async fn write_ibus_profile(settings: &crate::settings::Settings) -> Res<()> {
    let username = &settings.username;
    // dconf writes through its D-Bus service, which a bus of its own starts
    let Some(cmd) = ibus_dconf_writes(settings).and_then(crate::a11y::one_session) else {
        return Ok(());
    };

    let p = tokio::process::Command::new("pkexec")
        .args(["--user", username])
        .args(&cmd)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("fail to spawn pkexec for dbus command")?;

    let p = (p.wait_with_output().await)
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("cannot run dbus-run-session with pkexec")?;

    if !p.status.success() {
        return Err(Err::ProgramFail {
            name: "pkexec (dbus-run-session)",
            output: String::from_utf8_lossy(&p.stderr).to_string(),
            rc: p.status.code(),
        })
        .wrap_msg("failed to set the IBus engines");
    }

    Ok(())
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
        let script = ibus_dconf_writes(&settings).unwrap().concat().join(" ");
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
        let script = ibus_dconf_writes(&settings).unwrap().concat().join(" ");
        assert!(script.contains("[('xkb', 'ru'), ('xkb', 'us'), ('ibus', "), "{script}");
    }

    #[test]
    fn no_ibus_engines_writes_nothing() {
        let settings =
            crate::settings::Settings { ims: vec!["bogus".into()], ..Default::default() };
        assert!(ibus_dconf_writes(&settings).is_none());
    }
}
