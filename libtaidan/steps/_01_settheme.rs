//! light or dark mode for the new user, through `plasma-apply-colorscheme` on Plasma and
//! `gsettings` elsewhere
//!
//! both write into the home given in `HOME` and `XDG_CONFIG_HOME`, so this goes into the user's
//! home during setup, or on systemd-homed into the skeleton it's created from, see
//! [`write_to_skel`]
use crate::prelude::*;

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

/// the command that switches the config under `HOME` to `dark` or light, [`None`] if there is no
/// tool for it on this system
fn command(dark: bool) -> Option<Vec<&'static str>> {
    if std::path::Path::new(PLASMA).exists() {
        // a `QGuiApplication`, so it needs a platform even though it shows nothing
        let scheme = if dark { "BreezeDark" } else { "BreezeLight" };
        Some(vec!["QT_QPA_PLATFORM=offscreen", "plasma-apply-colorscheme", scheme])
    } else if std::path::Path::new(GSETTINGS).exists() {
        // dconf writes through its D-Bus service, which a bus of its own starts in this environment
        let scheme = if dark { "prefer-dark" } else { "default" };
        let args = ["set", "org.gnome.desktop.interface", "color-scheme", scheme];
        Some([&["dbus-run-session", "--", "gsettings"][..], &args].concat())
    } else {
        None
    }
}

/// `env` arguments that run [`command`] with `home` as the home directory
fn env_args(home: &std::path::Path, dark: bool) -> Option<Vec<String>> {
    let cmd = command(dark)?;
    let mut args = vec![
        // not the OOBE session's bus, or its KWin and apps would hear about the change
        "-u".to_owned(),
        "DBUS_SESSION_BUS_ADDRESS".to_owned(),
        format!("HOME={}", home.display()),
        format!("XDG_CONFIG_HOME={}", home.join(".config").display()),
    ];
    args.extend(cmd.into_iter().map(str::to_owned));
    Some(args)
}

/// sets the new user's theme, if they picked one, unless they're on systemd-homed, where it went
/// into the skeleton
///
/// # Errors
/// - cannot find the user's home directory
async fn apply_to_user(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let Some(dark) = settings.dark_mode else { return Ok(()) };
    if cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let home = crate::a11y::home_of(&settings.username).await?;
    let Some(env) = env_args(&home, dark) else {
        tracing::info!("neither plasma-apply-colorscheme nor gsettings found; skipping the theme");
        return Ok(());
    };
    // as the user, so the files they get are theirs
    let mut args = vec!["-u".to_owned(), settings.username.clone(), "--".to_owned()];
    args.push("env".to_owned());
    args.extend(env);
    if let Err(e) = crate::root("runuser", &args).await {
        tracing::warn!(?e, "cannot set the theme");
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
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(env) = settings.dark_mode.and_then(|dark| env_args(skel, dark)) else {
        return Ok(false);
    };
    tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
    let env = env.iter().map(String::as_str).collect_vec();
    if let Err(e) = crate::steps::acmd("env", &env).await {
        tracing::warn!(?e, "cannot set the theme");
    }
    Ok(true)
}
