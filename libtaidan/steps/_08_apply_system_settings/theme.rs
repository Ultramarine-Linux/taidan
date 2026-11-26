use std::path::Path;

use crate::{cfg::Desktop, home::Home, kconfig::KConfigEntry, prelude::*, theme::Accent};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {}

impl crate::steps::InstallStep for Theme {
    fn stage(&self) -> crate::steps::Stage {
        crate::steps::Stage::ApplySystemSettings
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _callback: &impl crate::Callback,
    ) -> Res<()> {
        run(settings, cfg).await
    }
}

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if settings.dark_mode.is_none() || cfg.systemd_homed() {
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
const PLASMA_DARK: &str = "org.ultramarine-linux.ultramarine.desktop";
const PLASMA_LIGHT: &str = "org.ultramarine-linux.ultramarinelight.desktop";

fn gsettings(key: &str, value: &str) -> Vec<String> {
    crate::home::gsettings("org.gnome.desktop.interface", key, Some(value))
}

fn xfconf(channel: &str, property: &str, value: &str) -> Vec<String> {
    crate::home::xfconf(channel, property, "string", Some(value))
}

const LOOKANDFEEL_DIR: &str = "/usr/share/plasma/look-and-feel";
const COLOR_SCHEMES_DIR: &str = "/usr/share/color-schemes";
const ULTRAMARINE_DARK: &str = "UltramarineDark";

#[derive(Debug, PartialEq, Eq)]
struct PlasmaLook {
    lookandfeel: Option<&'static str>,
    colors: &'static str,
    icons: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct PlasmaInstalled {
    lookandfeel_dark: bool,
    lookandfeel_light: bool,
    ultramarine_dark: bool,
}

impl PlasmaInstalled {
    fn detect() -> Self {
        let installed = Self {
            lookandfeel_dark: Path::new(LOOKANDFEEL_DIR).join(PLASMA_DARK).is_dir(),
            lookandfeel_light: Path::new(LOOKANDFEEL_DIR).join(PLASMA_LIGHT).is_dir(),
            ultramarine_dark: Path::new(COLOR_SCHEMES_DIR)
                .join(format!("{ULTRAMARINE_DARK}.colors"))
                .is_file(),
        };
        tracing::debug!(?installed, "Plasma themes found");
        installed
    }
}

const fn plasma_look(dark: bool, breeze_dark: bool, installed: PlasmaInstalled) -> PlasmaLook {
    let ultramarine_dark = !breeze_dark && installed.ultramarine_dark;
    match (dark, ultramarine_dark) {
        (false, _) => PlasmaLook {
            lookandfeel: if installed.lookandfeel_light { Some(PLASMA_LIGHT) } else { None },
            colors: "BreezeLight",
            icons: "breeze",
        },
        (true, true) => PlasmaLook {
            lookandfeel: if installed.lookandfeel_dark { Some(PLASMA_DARK) } else { None },
            colors: ULTRAMARINE_DARK,
            icons: "breeze-dark",
        },
        (true, false) => PlasmaLook {
            lookandfeel: if installed.lookandfeel_dark { Some(PLASMA_DARK) } else { None },
            colors: "BreezeDark",
            icons: "breeze-dark",
        },
    }
}

fn plasma_commands(
    dark: bool,
    breeze_dark: bool,
    accent: Option<Accent>,
    installed: PlasmaInstalled,
) -> Vec<Vec<String>> {
    let look = plasma_look(dark, breeze_dark, installed);
    tracing::info!(?look, ?accent, "Applying the Plasma theme");
    let headless = |program: &str, args: &[&str]| {
        [HEADLESS_QT_PLATFORM, program].iter().chain(args).map(|&s| s.to_owned()).collect()
    };
    let apply_colorscheme = |args: &[&str]| headless("plasma-apply-colorscheme", args);
    let kdeglobals = |group, key, value| {
        KConfigEntry::value("kdeglobals", group, key, Some(value)).command(None)
    };
    // plasma-apply-colorscheme skips a scheme that's already the default, which the look and
    // feel may still override
    let mut cmds: Vec<Vec<String>> = look
        .lookandfeel
        .map(|lookandfeel| headless("plasma-apply-lookandfeel", &["--apply", lookandfeel]))
        .into_iter()
        .collect();
    cmds.extend([
        apply_colorscheme(&[look.colors]),
        kdeglobals("General", "ColorScheme", look.colors),
        kdeglobals("Icons", "Theme", look.icons),
    ]);
    if let Some(accent) = accent {
        // with `--accent-color` it reapplies the saved scheme and doesn't save the accent
        cmds.push(apply_colorscheme(&["--accent-color", accent.hex()]));
        cmds.push(kdeglobals("General", "AccentColor", accent.hex()));
    }
    cmds
}

fn theme_commands(
    dark: bool,
    breeze_dark: bool,
    accent: Option<Accent>,
    desktop: Desktop,
    plasma: PlasmaInstalled,
) -> Vec<Vec<String>> {
    let accent = accent.filter(|_| desktop.supports_accent());
    let themes = desktop.themes(dark);
    let mut cmds = vec![];
    if desktop == Desktop::Plasma {
        cmds.extend(plasma_commands(dark, breeze_dark, accent, plasma));
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
    let Some(cmd) = home.session(theme_commands(
        dark,
        settings.breeze_dark,
        settings.accent,
        desktop,
        PlasmaInstalled::detect(),
    )) else {
        tracing::info!("neither plasma-apply-colorscheme nor gsettings found; skipping the theme");
        return Ok(false);
    };
    home.run(vec![cmd.best_effort()]).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_INSTALLED: PlasmaInstalled =
        PlasmaInstalled { lookandfeel_dark: true, lookandfeel_light: true, ultramarine_dark: true };
    const ONLY_BREEZE: PlasmaInstalled = PlasmaInstalled {
        lookandfeel_dark: true,
        lookandfeel_light: false,
        ultramarine_dark: false,
    };

    #[tokio::test]
    async fn skipped_theme_leaves_the_home_untouched() {
        let settings = crate::settings::Settings::default();
        assert!(settings.dark_mode.is_none());
        // No lookup or write for the empty username should be attempted.
        run(&settings, &crate::cfg::Config::default()).await.unwrap();
    }

    #[test]
    fn retry_uses_the_new_theme_and_accent() {
        let first = theme_commands(true, false, Some(Accent::Blue), Desktop::Plasma, ALL_INSTALLED);
        let retry =
            theme_commands(false, false, Some(Accent::Green), Desktop::Plasma, ALL_INSTALLED);
        assert!(first.iter().flatten().any(|arg| arg == "UltramarineDark"));
        assert!(first.iter().flatten().any(|arg| arg == PLASMA_DARK));
        assert!(retry.iter().flatten().any(|arg| arg == "BreezeLight"));
        assert!(retry.iter().flatten().any(|arg| arg == PLASMA_LIGHT));
        assert!(!retry.iter().flatten().any(|arg| arg == "UltramarineDark"));
        assert!(retry.iter().flatten().any(|arg| arg == Accent::Green.hex()));
        assert!(!retry.iter().flatten().any(|arg| arg == Accent::Blue.hex()));
    }

    #[test]
    fn breeze_dark_keeps_the_ultramarine_look_and_feel() {
        let look = |dark, breeze_dark| {
            let look = plasma_look(dark, breeze_dark, ALL_INSTALLED);
            (look.lookandfeel, look.colors, look.icons)
        };
        assert_eq!(look(true, true), (Some(PLASMA_DARK), "BreezeDark", "breeze-dark"));
        assert_eq!(look(true, false), (Some(PLASMA_DARK), ULTRAMARINE_DARK, "breeze-dark"));
        assert_eq!(look(false, true), (Some(PLASMA_LIGHT), "BreezeLight", "breeze"));
    }

    #[test]
    fn missing_ultramarine_themes_fall_back_to_breeze() {
        assert_eq!(
            plasma_look(true, false, ONLY_BREEZE),
            PlasmaLook {
                lookandfeel: Some(PLASMA_DARK),
                colors: "BreezeDark",
                icons: "breeze-dark"
            }
        );
        assert_eq!(plasma_look(false, false, ONLY_BREEZE).lookandfeel, None);
        let cmds = theme_commands(false, false, None, Desktop::Plasma, ONLY_BREEZE);
        assert!(!cmds.iter().flatten().any(|arg| arg == "plasma-apply-lookandfeel"));
        assert!(cmds.iter().flatten().any(|arg| arg == "BreezeLight"));
    }

    #[test]
    fn every_desktop_sets_icons_for_the_mode() {
        let icons = |dark, desktop| {
            let cmds = theme_commands(dark, false, None, desktop, ALL_INSTALLED);
            cmds.into_iter().flatten().collect::<Vec<_>>()
        };
        assert!(icons(true, Desktop::Plasma).iter().any(|arg| arg == "breeze-dark"));
        assert!(icons(false, Desktop::Plasma).iter().any(|arg| arg == "breeze"));
        assert!(!icons(false, Desktop::Plasma).iter().any(|arg| arg == "breeze-dark"));
        for desktop in [Desktop::Budgie, Desktop::Xfce] {
            for dark in [false, true] {
                assert!(desktop.themes(dark).unwrap().icons.is_some(), "{desktop:?} {dark}");
            }
        }
    }
}
