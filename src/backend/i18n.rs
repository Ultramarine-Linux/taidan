use gettextrs::gettext;
use itertools::Itertools;

use super::theme::pkexec;

taidan_proc_macros::keymap!(LAYOUTS);

/// List of languages that might require IMEs.
///
/// The list is obtained from <https://wiki.ultramarine-linux.org/en/usage/l10n/#list-of-imes-and-ims>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IMELanguages {
    Chinese,
    Japanese,
    Korean,
    Vietnamese,
    Indic,
    Thai,
}
pub struct InputMethod {
    pub native_name: &'static str,
    pub ibus_ref: &'static str,
    pub ibus_pkg: &'static str,
    pub fcitx5_ref: &'static str,
    pub fcitx5_pkg: &'static str,
}
impl IMELanguages {
    #[must_use]
    pub const fn native_append(self) -> &'static str {
        // Wikipedia translation from the language selection list
        // there's basically no way they could get this wrong so
        match self {
            Self::Chinese => "中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Vietnamese => "tiếng Việt",
            Self::Indic => "", // this is not really a language by itself so
            Self::Thai => "ไทย",
        }
    }
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Chinese => gettext("Chinese"),
            Self::Japanese => gettext("Japanese"),
            Self::Korean => gettext("Korean"),
            Self::Vietnamese => gettext("Vietnamese"),
            Self::Indic => gettext("Indic"),
            Self::Thai => gettext("Thai"),
        }
    }
    #[must_use]
    pub fn display(self) -> String {
        format!("{self}")
    }
}

impl std::fmt::Display for IMELanguages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let native = self.native_append();
        f.write_str(&if native.is_empty() {
            self.name()
        } else {
            gettext("%lang_name (%native_lang_name)")
                .replace("%lang_name", &self.name())
                .replace("%native_lang_name", native)
        })
    }
}

async fn set_kde_keymap(user: &str, layout: &str, variant: Option<&str>) -> color_eyre::Result<()> {
    let args = [
        "--file",
        "kxkbrc",
        "--group",
        "Layout",
        "--key",
        "LayoutList",
        layout,
    ];
    pkexec(user, "kwriteconfig6", &args).await?;
    let variant = variant.unwrap_or("");
    let args = [
        "--file",
        "kxkbrc",
        "--group",
        "Layout",
        "--key",
        "VariantList",
        variant,
    ];
    pkexec(user, "kwriteconfig6", &args).await?;
    let args = [
        ["--session", "--type=signal"],
        ["--reply-timeout=100", "--reply-timeout=100"],
        ["/Layouts", "org.kde.keyboard.reloadConfig"],
    ];
    pkexec(user, "dbus-send", &args.concat()).await?;
    Ok(())
}
async fn set_gsettings_keymap(
    user: &str,
    layout: &str,
    variant: Option<&str>,
) -> color_eyre::Result<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let name = format!(
        "{layout}{}",
        variant.map(|v| format!("+{v}")).unwrap_or_default()
    );
    let args = [
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('xkb', '{name}')]")],
    ];
    pkexec(user, "gsettings", &args.concat()).await?;
    Ok(())
}

async fn set_kde_all(
    user: &str,
    layout: &str,
    variant: Option<&str>,
    im: &[&str],
) -> color_eyre::Result<()> {
    set_kde_keymap(user, layout, variant).await?;
    Ok(todo!())
}
async fn set_gsettings_all(
    user: &str,
    layout: &str,
    variant: Option<&str>,
    im: &[&str],
) -> color_eyre::Result<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let name = format!(
        "{layout}{}",
        variant.map(|v| format!("+{v}")).unwrap_or_default()
    );
    let ims = im.iter().map(|s| format!(", ('ibus', '{s}')")).join("");
    let args = [
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('xck', '{name}'){ims}]",)],
    ];
    pkexec(user, "gsettings", &args.concat()).await?;
    Ok(())
}

pub async fn set_keymap(
    user: Option<&str>,
    layout: &str,
    variant: Option<&str>,
) -> color_eyre::Result<()> {
    let mut tmp = std::ffi::OsString::default();
    let user = user.unwrap_or_else(|| {
        tmp = uzers::get_current_username().expect("can't get current username");
        tmp.to_str().unwrap()
    });
    if (tokio::fs::try_exists("/usr/share/kwriteconfig6").await).is_ok() {
        set_kde_keymap(user, layout, variant).await
    } else {
        set_gsettings_keymap(user, layout, variant).await
    }
}

pub async fn set_all(
    user: Option<&str>,
    layout: &str,
    variant: Option<&str>,
    im: &[&str],
) -> color_eyre::Result<()> {
    let mut tmp = std::ffi::OsString::default();
    let user = user.unwrap_or_else(|| {
        tmp = uzers::get_current_username().expect("can't get current username");
        tmp.to_str().unwrap()
    });
    if (tokio::fs::try_exists("/usr/share/kwriteconfig6").await).is_ok() {
        set_kde_all(user, layout, variant, im).await
    } else {
        set_gsettings_all(user, layout, variant, im).await
    }
}
