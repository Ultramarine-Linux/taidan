use super::{pkexec, root};
use crate::prelude::*;

taidan_proc_macros::keymap!(LAYOUTS);

macro_rules! im {
    ($native:ident $ibus:tt$(=>$ibus_pkg:literal)?, $fcitx5:tt$(=>$fcitx5_pkg:literal)?) => {
        InputMethod {
            native_name: stringify!($native),
            ibus_ref: im!(#$ibus),
            ibus_pkg: im!(@ibus$ibus$($ibus_pkg)?),
            fcitx5_ref: im!(#$fcitx5),
            fcitx5_pkg: im!(@fcitx5$fcitx5$($fcitx5_pkg)?),
        }
    };
    (#()) => {None};
    (#$a:literal) => {Some($a)};
    (@$imf:ident ()) => {None};
    (@$imf:ident$a:literal) => {Some(const_format::formatcp!("{}-{}", stringify!($imf), $a))};
    (@$imf:ident$a:literal $b:literal) => {Some(const_format::formatcp!("{}-{}", stringify!($imf), $b))};
}

// lang → `InputMethod`
// NOTE: some characters may not display properly on your device, please make
// sure you have the corresponding fonts installed.
// BUG: What do we do with the difference between Traditional Chinese and Simplified Chinese native names?
// BUG: And what do we do with the naming of Rime?
// FIXME: package names should not be hardcoded?
pub const IMS: phf::OrderedMap<&'static str, phf::OrderedMap<&'static str, InputMethod>> = phf::phf_ordered_map! {
    //  IME                        Native       Ibus(=>pkg)                                     Fcitx5(=>pkg)
    "Chinese" => phf::phf_ordered_map! {
        "Pinyin"            => im!(拼音         "pinyin",                                       "pinyin"=>"chinese-addons"),
        "Zhuyin"            => im!(注音         "chewing",                                      "chewing"),
        "Cangjie 3"         => im!(倉頡版本三   "table:cangjie3"=>"table-chinese-cangjie",      "cangjie3"=>"table-extra"),
        "Cangjie 5"         => im!(倉頡版本五   "table:cangjie5"=>"table-chinese-cangjie",      "cangjie5"=>"table-extra"),
        "Quick 3"           => im!(速成版本三   "table:quick3"=>"table-chinese-quick",          "quick3"=>"table-extra"),
        "Quick 5"           => im!(速成版本五   "table:quick5"=>"table-chinese-quick",          "quick5"=>"table-extra"),
        "Quick Classic"     => im!(速成舊版     "table:quick-classic"=>"table-chinese-quick",   "quick-classic"=>"table-extra"),
        "Rime"              => im!(Rime         "rime",                                         "rime"),
        "Shuangpin"         => im!(双拼         (),                                             "shuangpin"=>"chinese-addons"),
        "Smart Cangjie 6"   => im!(快倉第六代   "table:scj6"=>"table-chinese-scj",              "scj6"=>"table-extra"),
        "Array 30"          => im!(行列三十     "table:array"=>"table-chinese-array",           "array30"=>"table-extra"),
        "Boshiamy"          => im!(嘸蝦米       (),                                             "boshiamy"=>"table-extra"),
    },
    "Japanese" => phf::phf_ordered_map! {
        "Mozc"              => im!(Mozc         "mozc-on"=>"mozc",                              "mozc"),
    },
    "Korean" => phf::phf_ordered_map! {
        "libhangul"         => im!(한글         "hangul",                                       "hangul"),
    },
    "Vietnamese" => phf::phf_ordered_map! {
        "Unikey"            => im!(Unikey       "Unikey"=>"unikey",                             "unikey"),
        "ViQR"              => im!(ViQR         "table:viqr"=>"table-tv",                       "viqr"=>"m17n"),
    },
    "Bengali" => phf::phf_ordered_map! {
        "OpenBangla"        => im!(বাংলা           "openbangla",                                      "openbangla"),
    },
    "Sinhala" => phf::phf_ordered_map! {
        "Sayura Sinhala"    => im!(සිංහල          "sayura",                                        "sayura"),
    },
    "Thai" => phf::phf_ordered_map! {
        "Thai"              => im!(ภาษาไทย      "table:thai"=>"table-tv",                       "libthai"=>"libthai")
    },
};

// NOTE: I know you want to scream in my face that Chinese is not a language
// but please shut up
pub const STR_TO_LANG: phf::Map<&'static str, IMELanguages> = phf::phf_map! {
    "Chinese" => IMELanguages::Chinese,
    "Japanese" => IMELanguages::Japanese,
    "Korean" => IMELanguages::Korean,
    "Vietnamese" => IMELanguages::Vietnamese,
    "Bengali" => IMELanguages::Bengali,
    "Sinhala" => IMELanguages::Sinhala,
    "Thai" => IMELanguages::Thai,
};

/// List of languages that might require IMEs.
///
/// The list is obtained from <https://wiki.ultramarine-linux.org/en/usage/l10n/#list-of-imes-and-ims>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IMELanguages {
    Chinese,
    Japanese,
    Korean,
    Vietnamese,
    Bengali,
    Sinhala,
    Thai,
}
impl IMELanguages {
    #[must_use]
    pub const fn native(self) -> &'static str {
        // Wikipedia translation from the language selection list
        // there's basically no way they could get this wrong so
        match self {
            Self::Chinese => "中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Vietnamese => "tiếng Việt",
            Self::Bengali => "বাংলা",
            Self::Sinhala => "සිංහල භාෂාව",
            Self::Thai => "ไทย",
        }
    }
}
impl std::fmt::Display for IMELanguages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.native())
    }
}

/// Turns the IM name written in [`IMS`] to the [`InputMethod`] object.
///
/// # Panics
/// This panics if and only if the IM name is invalid.
#[must_use]
pub fn str_to_im(s: &str) -> InputMethod {
    *IMS.values().find_map(|ims| ims.get(s)).expect("invalid im")
}

#[derive(Debug, Clone, Copy)]
pub struct InputMethod {
    pub native_name: &'static str,
    /// how `IBus` refers to this IME
    pub ibus_ref: Option<&'static str>,
    /// the package name of the IME
    pub ibus_pkg: Option<&'static str>,
    /// how Fcitx5 refers to tthis IME
    pub fcitx5_ref: Option<&'static str>,
    /// the package name of the IME
    pub fcitx5_pkg: Option<&'static str>,
}

impl InputMethod {
    #[must_use]
    pub fn available(self, imf: I18nImf) -> bool {
        match imf {
            I18nImf::Fcitx5 => self.fcitx5_ref.is_some(),
            I18nImf::IBus => self.ibus_ref.is_some(),
        }
    }
    /// Gets the package for the current input method framework
    ///
    /// # Panics
    /// Panics if and only if this IM is not available for the current framework.
    #[must_use]
    pub fn get_pkg(self, imf: I18nImf) -> &'static str {
        match imf {
            I18nImf::Fcitx5 => self.fcitx5_pkg.unwrap(),
            I18nImf::IBus => self.ibus_pkg.unwrap(),
        }
    }
    /// Gets the internal reference of the IM for the current input method framework
    ///
    /// # Panics
    /// Panics if and only if this IM is not available for the current framework.
    #[must_use]
    pub fn get_ref(self, imf: I18nImf) -> &'static str {
        match imf {
            I18nImf::Fcitx5 => self.fcitx5_ref.unwrap(),
            I18nImf::IBus => self.ibus_ref.unwrap(),
        }
    }
}

async fn set_kde_keymap(user: &str, layout: &str, variant: Option<&str>) -> Res<()> {
    let args = ["--file", "kxkbrc", "--group", "Layout", "--key", "LayoutList", layout];
    pkexec(user, "kwriteconfig6", &args).await?;
    let variant = variant.unwrap_or("");
    let args = ["--file", "kxkbrc", "--group", "Layout", "--key", "VariantList", variant];
    pkexec(user, "kwriteconfig6", &args).await?;
    let args = [
        ["--session", "--type=signal"],
        ["--reply-timeout=100", "--reply-timeout=100"],
        ["/Layouts", "org.kde.keyboard.reloadConfig"],
    ];
    pkexec(user, "dbus-send", &args.concat()).await?;
    Ok(())
}
async fn set_gsettings_keymap(user: &str, layout: &str, variant: Option<&str>) -> Res<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let name = format!("{layout}{}", variant.map(|v| format!("+{v}")).unwrap_or_default());
    let args = [
        ["DISPLAY=:0", "gsettings"],
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('xkb', '{name}')]")],
    ];
    pkexec(user, "env", &args.concat()).await?;
    Ok(())
}

async fn set_localectl_keymap(layout: &str, variant: Option<&str>) -> Res<()> {
    let variant = variant.unwrap_or("");
    let args = ["set-x11-keymap", layout, "", variant, ""];
    root("localectl", &args).await?;
    Ok(())
}

#[allow(clippy::equatable_if_let)]
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn set_keymap(user: Option<&str>, layout: &str, variant: Option<&str>) -> Res<()> {
    let current_user = uzers::get_current_username().expect("can't get current username");
    if current_user.as_encoded_bytes() != b"taidan" && cfg!(debug_assertions) {
        // #74
        tracing::warn!("Skipping set_keymap in debug mode without being `taidan` user");
        return Ok(());
    }
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/localectl").await {
        set_localectl_keymap(layout, variant).await?;
    }

    crate::xhost_local().await?;
    let user = user.unwrap_or_else(|| current_user.to_str().unwrap());
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/kwriteconfig6").await {
        set_kde_keymap(user, layout, variant).await
    } else {
        set_gsettings_keymap(user, layout, variant).await
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct I18nCfg {
    pub imf: I18nImf,
}

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum I18nImf {
    Fcitx5,
    #[default]
    IBus,
}

impl I18nImf {
    /// Returns `true` if the i18n imf is [`Fcitx5`].
    ///
    /// [`Fcitx5`]: I18nImf::Fcitx5
    #[must_use]
    pub const fn is_fcitx5(&self) -> bool {
        matches!(self, Self::Fcitx5)
    }

    /// Returns `true` if the i18n imf is [`IBus`].
    ///
    /// [`IBus`]: I18nImf::IBus
    #[must_use]
    pub const fn is_ibus(&self) -> bool {
        matches!(self, Self::IBus)
    }
}
