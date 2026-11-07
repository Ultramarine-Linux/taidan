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

const PLASMA: &str = "/usr/bin/plasma-apply-colorscheme";
const GSETTINGS: &str = "/usr/bin/gsettings";
const XFCONF: &str = "/usr/bin/xfconf-query";

/// `gsettings set org.gnome.desktop.interface <key> <value>`, through a D-Bus of its own since
/// dconf writes through its service
fn gsettings(key: &str, value: &str) -> Vec<String> {
    ["dbus-run-session", "--", "gsettings", "set", "org.gnome.desktop.interface", key, value]
        .map(str::to_owned)
        .to_vec()
}

/// the commands that switch the config under `HOME` to `dark` or light, with the accent if the
/// edition follows one, empty if there is no tool for it on this system
fn commands(dark: bool, accent: Option<Accent>, edition: &str) -> Vec<Vec<String>> {
    let accent = accent.filter(|_| crate::theme::accent_supported(edition));
    let mut cmds = vec![];
    if std::path::Path::new(PLASMA).exists() {
        // a `QGuiApplication`, so it needs a platform even though it shows nothing
        let scheme = if dark { "BreezeDark" } else { "BreezeLight" };
        let mut cmd = vec!["QT_QPA_PLATFORM=offscreen", "plasma-apply-colorscheme", scheme];
        if let Some(accent) = accent {
            cmd.extend(["--accent-color", accent.hex()]);
        }
        cmds.push(cmd.into_iter().map(str::to_owned).collect());
    } else if std::path::Path::new(GSETTINGS).exists() {
        cmds.push(gsettings("color-scheme", if dark { "prefer-dark" } else { "default" }));
        if let Some(accent) = accent {
            cmds.push(gsettings("accent-color", accent.id()));
        }
        if let Some(theme) = crate::theme::gtk_theme(edition, dark) {
            cmds.push(gsettings("gtk-theme", theme));
        }
    }
    if let Some(theme) = crate::theme::xfce_theme(edition, dark)
        && std::path::Path::new(XFCONF).exists()
    {
        // `-n` creates the property in case the defaults don't have it
        let args = ["dbus-run-session", "--", "xfconf-query", "-c", "xsettings"];
        let args =
            [&args[..], &["-p", "/Net/ThemeName", "-n", "-t", "string", "-s", theme]].concat();
        cmds.push(args.into_iter().map(str::to_owned).collect());
    }
    cmds
}

/// `env` arguments for each of [`commands`], running them with `home` as the home directory
fn env_args(
    home: &std::path::Path,
    settings: &crate::settings::Settings,
    edition: &str,
) -> Vec<Vec<String>> {
    let Some(dark) = settings.dark_mode else { return vec![] };
    (commands(dark, settings.accent, edition).into_iter())
        .map(|cmd| {
            let mut args = vec![
                // not the OOBE session's bus, or its KWin and apps would hear about the change
                "-u".to_owned(),
                "DBUS_SESSION_BUS_ADDRESS".to_owned(),
                format!("HOME={}", home.display()),
                format!("XDG_CONFIG_HOME={}", home.join(".config").display()),
            ];
            args.extend(cmd);
            args
        })
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
        let mut args = vec!["-u".to_owned(), settings.username.clone(), "--".to_owned()];
        args.push("env".to_owned());
        args.extend(env);
        if let Err(e) = crate::root("runuser", &args).await {
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
