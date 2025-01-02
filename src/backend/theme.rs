#![allow(clippy::equatable_if_let)]
use crate::prelude::*;

macro_rules! gen_accent_color_enum {
    ($($x:ident)*) => { ::paste::paste! {
        #[derive(Clone, Copy, Debug, Default)]
        pub enum AccentColor { #[default] $([<$x:camel>]),* }
        impl From<AccentColor> for &str {
            fn from(value: AccentColor) -> Self {
                match value {$(AccentColor::[<$x:camel>] => stringify!([<$x:snake>]),)*}
            }
        }
        impl AccentColor {
            #[must_use] pub const fn all() -> &'static [Self] {
                &[$(Self::[<$x:camel>]),*]
            }
        }
    }};
}

//? https://gitlab.gnome.org/GNOME/gsettings-desktop-schemas/-/merge_requests/63/diffs#65231a5ac1ed586909f5137f1e9bdfe879aaa67b_314_318
gen_accent_color_enum!(blue teal green yellow orange red pink purple slate);

impl AccentColor {
    /// # Errors
    /// - cannot apply color-scheme/accent-color/gtk-theme via pkexec
    pub async fn gsettings(self, user: &str, is_dark: bool) -> color_eyre::Result<()> {
        let args = [
            ["set", "org.gnome.desktop.interface"],
            ["accent-color", self.into()],
        ];
        pkexec(user, "gsettings", &args.concat()).await?;

        let args = [
            "set",
            "org.gnome.desktop.interface",
            "color-scheme",
            if is_dark { "prefer-dark" } else { "default" },
        ];
        pkexec(user, "gsettings", &args).await?;

        let args = [
            "set",
            "org.gnome.desktop.interface",
            "gtk-theme",
            if is_dark { "Adwaita-dark" } else { "Adwaita" },
        ];
        pkexec(user, "gsettings", &args).await?;

        Ok(())
    }

    #[must_use]
    pub fn w3_color_keywords(self) -> &'static str {
        match self {
            Self::Slate => "slategrey",
            x => x.into(),
        }
    }
    /// # Errors
    /// - cannot apply color scheme / accent via pkexec
    pub async fn plasma(self, user: &str, is_dark: bool) -> color_eyre::Result<()> {
        let theme = if is_dark { "BreezeDark" } else { "BreezeLight" };
        let color = self.w3_color_keywords();
        pkexec(user, "plasma-apply-colorscheme", &[theme, "-a", color]).await?;
        Ok(())
    }
}

/// # Errors
/// - cannot apply color scheme via pkexec
pub async fn plasma_set_theme_only(user: &str, is_dark: bool) -> color_eyre::Result<()> {
    let theme = if is_dark { "BreezeDark" } else { "BreezeLight" };
    pkexec(user, "plasma-apply-colorscheme", &[theme]).await?;
    Ok(())
}

/// # Errors
/// - cannot set theme via pkexec
///
/// # Panics
/// - neither `plasma-apply-colorscheme` and `gsettings` are installed.
#[allow(clippy::module_name_repetitions)]
pub async fn set_theme(
    user: Option<&str>,
    is_dark: bool,
    accent: Option<AccentColor>,
) -> color_eyre::Result<()> {
    let mut tmp = std::ffi::OsString::default();
    let user = user.unwrap_or_else(|| {
        tmp = uzers::get_current_username().expect("can't get current username");
        tmp.to_str().unwrap()
    });
    tracing::debug!(?user);
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/plasma-apply-colorscheme").await {
        if let Some(accent) = accent {
            (accent.plasma(user, is_dark).await)
                .wrap_err("cannot set accent and theme for plama")?;
        } else {
            (plasma_set_theme_only(user, is_dark).await).wrap_err("cannot set theme for plasma")?;
        }
    } else if let Ok(true) = tokio::fs::try_exists("/usr/bin/gsettings").await {
        (accent.unwrap_or_default().gsettings(user, is_dark).await)
            .wrap_err("cannot set accent/theme using gsettings")?;
    } else {
        panic!("Neither plasma-apply-colorscheme and gsettings are found in /usr/bin");
    }
    Ok(())
}

/// # Errors
/// - if executing processes fail, an error will be returned
///
/// # Panics
/// - cannot get current username (only when `user` is not supplied)
pub async fn set_night_light(user: Option<&str>, enabled: bool) -> color_eyre::Result<()> {
    let mut tmp = std::ffi::OsString::default();
    let user = user.unwrap_or_else(|| {
        tmp = uzers::get_current_username().expect("can't get current username");
        tmp.to_str().unwrap()
    });
    tracing::debug!(?user);
    if let Ok(true) = tokio::fs::try_exists("kwriteconfig6").await {
        let args = [
            ["--file", "~/.config/kwinrc", "--group"],
            ["NightColor", "--key", "Active"],
            ["--type", "bool", if enabled { "1" } else { "0" }],
        ];
        pkexec(user, "kwriteconfig6", &args.concat()).await?;
    } else {
        let args = [
            ["set", "org.gnome.settings-daemon.plugins.color"],
            ["night-light-enabled", if enabled { "1" } else { "0" }],
        ];
        pkexec(user, "gsettings", &args.concat()).await?;
    }
    Ok(())
}

/// # Errors
/// - command failed to run
/// - command exited with non-zero status code
pub async fn pkexec(user: &str, name: &str, args: &[&str]) -> color_eyre::Result<()> {
    let p = tokio::process::Command::new("pkexec")
        .args(["--user", user, name])
        .args(args)
        .status()
        .await
        .wrap_err(format!("fail to run `{name}`"))?;
    if !p.success() {
        return Err(eyre!("`{name}` failed").note(format!("Exit code: {:?}", p.code())));
    }
    Ok(())
}
