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
        if let Err(e) = apply_to_user(settings, cfg).await {
            tracing::warn!(?e, "cannot set the theme");
        }
        Ok(())
    }
}

const GSETTINGS: &str = "/usr/bin/gsettings";
const XFCONF: &str = "/usr/bin/xfconf-query";
const HEADLESS_QT_PLATFORM: &str = "QT_QPA_PLATFORM=offscreen";

fn gsettings(key: &str, value: &str) -> Vec<String> {
    crate::a11y::gsettings("org.gnome.desktop.interface", key, Some(value))
}

fn xfconf(channel: &str, property: &str, value: &str) -> Vec<String> {
    crate::a11y::xfconf(channel, property, "string", Some(value))
}

fn theme_commands(dark: bool, accent: Option<Accent>, edition: &str) -> Vec<Vec<String>> {
    let accent = accent.filter(|_| crate::theme::accent_supported(edition));
    let themes = crate::theme::themes(edition, dark);
    let mut cmds = vec![];
    if crate::a11y::is_plasma() {
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
        cmds.push(owned(&[HEADLESS_QT_PLATFORM, "plasma-apply-colorscheme", scheme]));
        // plasma-apply-colorscheme skips a scheme that's already the default, which the look and
        // feel may still override
        cmds.push(kdeglobals("ColorScheme", scheme));
        if let Some(accent) = accent {
            // with `--accent-color` it reapplies the saved scheme and doesn't save the accent
            cmds.push(owned(&[
                HEADLESS_QT_PLATFORM,
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

fn env_args(
    home: &std::path::Path,
    settings: &crate::settings::Settings,
    edition: &str,
) -> Option<Vec<String>> {
    let cmds = theme_commands(settings.dark_mode?, settings.accent, edition);
    crate::a11y::one_session(cmds).map(|cmd| crate::a11y::in_home(home, cmd))
}

async fn apply_to_user(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if settings.dark_mode.is_none() || cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let home = crate::a11y::home_of(&settings.username).await?;
    let Some(env) = env_args(&home, settings, &cfg.edition) else {
        tracing::info!("neither plasma-apply-colorscheme nor gsettings found; skipping the theme");
        return Ok(());
    };
    let mut argv = vec!["env".to_owned()];
    argv.extend(env);
    if let Err(e) = crate::as_user(&settings.username, argv, None).await {
        tracing::warn!(?e, "cannot set the theme");
    }
    Ok(())
}

pub async fn write_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(env) = env_args(skel, settings, &cfg.edition) else { return Ok(false) };
    tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
    let env = env.iter().map(String::as_str).collect_vec();
    if let Err(e) = crate::steps::acmd("env", &env).await {
        tracing::warn!(?e, "cannot set the theme");
    }
    Ok(true)
}
