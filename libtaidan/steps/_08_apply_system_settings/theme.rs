use crate::{cfg::Desktop, home::Home, kconfig::KConfigEntry, prelude::*, theme::Accent};

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if settings.dark_mode.is_none() || cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let res = async { apply(settings, &Home::user(settings, cfg).await?, cfg.desktop()).await };
    if let Err(e) = res.await {
        tracing::warn!(?e, "cannot set the theme");
    }
    Ok(())
}

const GSETTINGS: &str = "/usr/bin/gsettings";
const XFCONF: &str = "/usr/bin/xfconf-query";
const HEADLESS_QT_PLATFORM: &str = "QT_QPA_PLATFORM=offscreen";

fn gsettings(key: &str, value: &str) -> Vec<String> {
    crate::home::gsettings("org.gnome.desktop.interface", key, Some(value))
}

fn xfconf(channel: &str, property: &str, value: &str) -> Vec<String> {
    crate::home::xfconf(channel, property, "string", Some(value))
}

fn plasma_commands(dark: bool, accent: Option<Accent>) -> Vec<Vec<String>> {
    let scheme = if dark { "BreezeDark" } else { "BreezeLight" };
    let apply_colorscheme = |args: &[&str]| {
        [HEADLESS_QT_PLATFORM, "plasma-apply-colorscheme"]
            .iter()
            .chain(args)
            .map(|&s| s.to_owned())
            .collect()
    };
    let kdeglobals =
        |key, value| KConfigEntry::value("kdeglobals", "General", key, Some(value)).command(None);
    // plasma-apply-colorscheme skips a scheme that's already the default, which the look and
    // feel may still override
    let mut cmds = vec![apply_colorscheme(&[scheme]), kdeglobals("ColorScheme", scheme)];
    if let Some(accent) = accent {
        // with `--accent-color` it reapplies the saved scheme and doesn't save the accent
        cmds.push(apply_colorscheme(&["--accent-color", accent.hex()]));
        cmds.push(kdeglobals("AccentColor", accent.hex()));
    }
    cmds
}

fn theme_commands(dark: bool, accent: Option<Accent>, desktop: Desktop) -> Vec<Vec<String>> {
    let accent = accent.filter(|_| desktop.supports_accent());
    let themes = desktop.themes(dark);
    let mut cmds = vec![];
    if desktop == Desktop::Plasma {
        cmds.extend(plasma_commands(dark, accent));
    } else if std::path::Path::new(GSETTINGS).exists() {
        cmds.push(gsettings("color-scheme", if dark { "prefer-dark" } else { "default" }));
        if let Some(accent) = accent {
            cmds.push(gsettings("accent-color", accent.id()));
        }
        if let Some(themes) = themes.filter(|_| desktop != Desktop::Xfce) {
            cmds.push(gsettings("gtk-theme", themes.gtk));
            if let Some(icons) = themes.icons {
                cmds.push(gsettings("icon-theme", icons));
            }
        }
    }
    if let Some(themes) = themes.filter(|_| desktop == Desktop::Xfce)
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

pub(in crate::steps) async fn apply(
    settings: &crate::settings::Settings,
    home: &Home,
    desktop: Desktop,
) -> Res<bool> {
    let Some(dark) = settings.dark_mode else { return Ok(false) };
    let Some(cmd) = home.session(theme_commands(dark, settings.accent, desktop)) else {
        tracing::info!("neither plasma-apply-colorscheme nor gsettings found; skipping the theme");
        return Ok(false);
    };
    home.run(vec![cmd.best_effort()]).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn skipped_theme_leaves_the_home_untouched() {
        let settings = crate::settings::Settings::default();
        assert!(settings.dark_mode.is_none());
        // No lookup or write for the empty username should be attempted.
        run(&settings, &crate::cfg::Config::default()).await.unwrap();
    }

    #[test]
    fn retry_uses_the_new_theme_and_accent() {
        let first = theme_commands(true, Some(Accent::Blue), Desktop::Plasma);
        let retry = theme_commands(false, Some(Accent::Green), Desktop::Plasma);
        assert!(first.iter().flatten().any(|arg| arg == "BreezeDark"));
        assert!(retry.iter().flatten().any(|arg| arg == "BreezeLight"));
        assert!(!retry.iter().flatten().any(|arg| arg == "BreezeDark"));
        assert!(retry.iter().flatten().any(|arg| arg == Accent::Green.hex()));
        assert!(!retry.iter().flatten().any(|arg| arg == Accent::Blue.hex()));
    }
}
