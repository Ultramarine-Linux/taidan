//! light or dark mode and the accent color for the new user, through `plasma-apply-colorscheme` on
//! Plasma and `gsettings` elsewhere, plus the GTK theme that goes with the mode, see [`crate::theme`]
//!
//! both write into the home given in `HOME` and `XDG_CONFIG_HOME`, so this goes into the user's
//! home during setup, or on systemd-homed into the skeleton it's created from, see
//! [`write_to_skel`]
use crate::prelude::*;
use crate::theme::Accent;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetTheme;
impl super::Step for SetTheme {
    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // a desktop in the wrong mode isn't worth failing setup over
        if let Err(e) = apply_to_user(settings, cfg).await {
            tracing::warn!(?e, "cannot set the theme");
        }
        Ok(())
    }
}

const GSETTINGS: &str = "/usr/bin/gsettings";
const XFCONF: &str = "/usr/bin/xfconf-query";

fn gsettings(key: &str, value: &str) -> Vec<String> {
    crate::a11y::gsettings("org.gnome.desktop.interface", key, Some(value))
}

fn xfconf(channel: &str, property: &str, value: &str) -> Vec<String> {
    crate::a11y::xfconf(channel, property, "string", Some(value))
}

/// the commands that switch the config under `HOME` to `dark` or light, with the accent if the
/// edition follows one, empty if there is no tool for it on this system
fn commands(dark: bool, accent: Option<Accent>, edition: &str) -> Vec<Vec<String>> {
    let accent = accent.filter(|_| crate::theme::accent_supported(edition));
    let themes = crate::theme::themes(edition, dark);
    let mut cmds = vec![];
    // plasma-apply-colorscheme comes with plasmashell, in plasma-workspace
    if crate::a11y::is_plasma() {
        // a `QGuiApplication`, so it needs a platform even though it shows nothing
        let scheme = if dark { "BreezeDark" } else { "BreezeLight" };
        let owned = |cmd: &[&str]| cmd.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
        let kdeglobals = |key: &str, value: &str| {
            owned(&[
                "kwriteconfig6",
                "--file",
                "kdeglobals",
                "--group",
                "General",
                "--key",
                key,
                value,
            ])
        };
        cmds.push(owned(&["QT_QPA_PLATFORM=offscreen", "plasma-apply-colorscheme", scheme]));
        // it does nothing when the scheme is already the default, which the look and feel's
        // defaults may still override, so it's written down either way
        cmds.push(kdeglobals("ColorScheme", scheme));
        if let Some(accent) = accent {
            // with `--accent-color`, it ignores a scheme given with it and reapplies the saved
            // one, and it doesn't save the accent itself
            cmds.push(owned(&[
                "QT_QPA_PLATFORM=offscreen",
                "plasma-apply-colorscheme",
                "--accent-color",
                accent.hex(),
            ]));
            cmds.push(kdeglobals("AccentColor", accent.hex()));
        }
    } else if std::path::Path::new(GSETTINGS).exists() {
        cmds.push(gsettings("color-scheme", if dark { "prefer-dark" } else { "default" }));
        if let Some(accent) = accent {
            cmds.push(gsettings("accent-color", accent.id()));
        }
        if let Some(themes) = themes.filter(|_| !crate::theme::uses_xfconf(edition)) {
            cmds.push(gsettings("gtk-theme", themes.gtk));
            if let Some(icons) = themes.icons {
                cmds.push(gsettings("icon-theme", icons));
            }
        }
    }
    if let Some(themes) = themes.filter(|_| crate::theme::uses_xfconf(edition))
        && std::path::Path::new(XFCONF).exists()
    {
        cmds.push(xfconf("xsettings", "/Net/ThemeName", themes.gtk));
        if let Some(icons) = themes.icons {
            cmds.push(xfconf("xsettings", "/Net/IconThemeName", icons));
        }
        if let Some(wm) = themes.wm {
            cmds.push(xfconf("xfwm4", "/general/theme", wm));
        }
    }
    cmds
}

/// `env` arguments running [`commands`] in one go with `home` as the home directory, none if
/// there's nothing to run
fn env_args(
    home: &std::path::Path,
    settings: &crate::settings::Settings,
    edition: &str,
) -> Vec<Vec<String>> {
    let Some(dark) = settings.dark_mode else { return vec![] };
    // in one go, see `one_session`
    crate::a11y::one_session(commands(dark, settings.accent, edition))
        .map(|cmd| crate::a11y::in_home(home, cmd))
        .into_iter()
        .collect()
}

/// sets the new user's theme, if they picked one, unless they're on systemd-homed, where it went
/// into the skeleton
///
/// # Errors
/// - cannot find the user's home directory
async fn apply_to_user(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if settings.dark_mode.is_none() || cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let home = crate::a11y::home_of(&settings.username).await?;
    let cmds = env_args(&home, settings, &cfg.edition);
    if cmds.is_empty() {
        tracing::info!("neither plasma-apply-colorscheme nor gsettings found; skipping the theme");
    }
    for env in cmds {
        // as the user, so the files they get are theirs
        let mut argv = vec!["env".to_owned()];
        argv.extend(env);
        if let Err(e) = crate::as_user(&settings.username, argv, None).await {
            tracing::warn!(?e, "cannot set the theme");
        }
    }
    Ok(())
}

/// sets the theme in `skel`, a copy of `/etc/skel` the new user's home is made from, returns
/// whether there was anything to write
///
/// # Errors
/// - cannot create `skel/.config`
pub async fn write_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &std::path::Path,
) -> Res<bool> {
    let cmds = env_args(skel, settings, &cfg.edition);
    if cmds.is_empty() {
        return Ok(false);
    }
    tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
    for env in cmds {
        let env = env.iter().map(String::as_str).collect_vec();
        if let Err(e) = crate::steps::acmd("env", &env).await {
            tracing::warn!(?e, "cannot set the theme");
        }
    }
    Ok(true)
}
