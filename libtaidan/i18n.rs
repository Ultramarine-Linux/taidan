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
    find_im(s).expect("invalid im")
}

/// the [`InputMethod`] called `s` in [`IMS`], if there's one
#[must_use]
pub fn find_im(s: &str) -> Option<InputMethod> {
    IMS.values().find_map(|ims| ims.get(s)).copied()
}

/// names in [`IMS`] of the input methods that go with a POSIX locale like `zh_TW.UTF-8`, for when
/// the user didn't pick any, empty if the language is typed with the keyboard layout alone
#[must_use]
pub fn default_ims(locale: &str, imf: I18nImf) -> Vec<String> {
    let name = locale.split(['.', '@']).next().unwrap_or_default();
    let (lang, country) = name.split_once('_').unwrap_or((name, ""));
    let ims: &[&str] = match (lang, country) {
        // Traditional Chinese: Zhuyin is what Taiwan types with, Cangjie what Hong Kong and Macau do
        ("zh", "TW") => &["Zhuyin"],
        ("zh", "HK" | "MO") => &["Cangjie 5"],
        ("zh", _) => &["Pinyin"],
        ("ja", _) => &["Mozc"],
        ("ko", _) => &["libhangul"],
        ("vi", _) => &["Unikey"],
        ("bn", _) => &["OpenBangla"],
        ("si", _) => &["Sayura Sinhala"],
        ("th", _) => &["Thai"],
        _ => &[],
    };
    ims.iter()
        .filter(|im| find_im(im).is_some_and(|im| im.available(imf)))
        .map(|&im| im.to_owned())
        .collect()
}

/// sets the input method framework from the config, and the input methods from the locale if the
/// user didn't pick any, dropping any the framework doesn't have
pub fn fill_settings(settings: &mut crate::settings::Settings, cfg: &crate::cfg::Config) {
    settings.imf = cfg.i18n.imf(&cfg.edition);
    if settings.ims.is_empty() && !settings.ims_picked {
        settings.ims = default_ims(&settings.locale, settings.imf);
    }
    let imf = settings.imf;
    settings.ims.retain(|name| {
        let ok = find_im(name).is_some_and(|im| im.available(imf));
        if !ok {
            tracing::warn!(name, ?imf, "unknown input method, or not available for the framework");
        }
        ok
    });
    tracing::info!(?settings.imf, ?settings.ims, "Input methods");
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
    // the layout is written already, a session without the tool or a KWin to hear it just picks
    // it up on its next login
    if let Err(e) = pkexec(user, "dbus-send", &args.concat()).await {
        tracing::warn!(?e, "cannot tell KWin to reload the keyboard layout");
    }
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

/// `kwriteconfig6` arguments, after `--file`, that put the layout picked in `settings` into
/// `kxkbrc`, [`None`] if there's no layout
fn kxkbrc_entries(settings: &crate::settings::Settings) -> Option<[[&str; 6]; 3]> {
    if settings.kb_layout.is_empty() {
        return None;
    }
    let variant = settings.kb_variant.as_deref().unwrap_or("");
    Some([
        ["--group", "Layout", "--key", "Use", "--type=bool", "true"],
        ["--group", "Layout", "--key", "LayoutList", "--", &settings.kb_layout],
        ["--group", "Layout", "--key", "VariantList", "--", variant],
    ])
}

/// writes the keyboard layout into the new user's `kxkbrc`, since KWin on Wayland doesn't fall
/// back to the system's (locale1) layout, only to `XKB_DEFAULT_LAYOUT` or US. Plasma only, and
/// not on systemd-homed, where it went into the skeleton, see [`write_keymap_to_skel`]
///
/// # Errors
/// - cannot find the user's home directory
pub async fn apply_keymap_to_user(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Res<()> {
    if cfg.taidan0.systemd_homed || !crate::a11y::is_plasma() {
        return Ok(());
    }
    let Some(entries) = kxkbrc_entries(settings) else { return Ok(()) };
    let home = crate::a11y::home_of(&settings.username).await?;
    let file = home.join(".config/kxkbrc");
    let file = file.to_string_lossy();
    for entry in entries {
        // as the user, so the file they get is theirs
        let mut args = vec!["-u", &settings.username, "--", "kwriteconfig6", "--file", &file];
        args.extend(entry);
        root("runuser", &args).await.wrap_msg("cannot write the keyboard layout")?;
    }
    Ok(())
}

/// [`apply_keymap_to_user`] for `skel`, a copy of `/etc/skel` the new user's home is made from,
/// returns whether there was anything to write
///
/// # Errors
/// - cannot create `skel/.config` or write the file
pub async fn write_keymap_to_skel(
    settings: &crate::settings::Settings,
    skel: &std::path::Path,
) -> Res<bool> {
    let Some(entries) = kxkbrc_entries(settings).filter(|_| crate::a11y::is_plasma()) else {
        return Ok(false);
    };
    let config = skel.join(".config");
    tokio::fs::create_dir_all(&config).await.map_err(Err::Io)?;
    let file = config.join("kxkbrc");
    let file = file.to_string_lossy();
    for entry in entries {
        let mut args = vec!["--file", &*file];
        args.extend(entry);
        crate::steps::acmd("kwriteconfig6", &args).await?;
    }
    Ok(true)
}

#[allow(clippy::equatable_if_let)]
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub async fn set_keymap(user: Option<&str>, layout: &str, variant: Option<&str>) -> Res<()> {
    let current_user =
        uzers::get_current_username().ok_or(Err::Msg("can't get current username"))?;
    if current_user.as_encoded_bytes() != b"taidan" && cfg!(debug_assertions) {
        // #74
        tracing::warn!("Skipping set_keymap in debug mode without being `taidan` user");
        return Ok(());
    }
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/localectl").await {
        set_localectl_keymap(layout, variant).await?;
    }

    let user = match user {
        Some(user) => user,
        None => current_user.to_str().ok_or(Err::Msg("current username is not valid UTF-8"))?,
    };
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/kwriteconfig6").await {
        set_kde_keymap(user, layout, variant).await
    } else {
        set_gsettings_keymap(user, layout, variant).await
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct I18nCfg {
    /// the input method framework, see [`Self::imf()`] for when it's not set
    #[serde(default)]
    pub imf: Option<I18nImf>,
}

impl I18nCfg {
    /// the configured input method framework, otherwise IBus on GNOME, which has it built in, and
    /// Fcitx5 everywhere else
    #[must_use]
    pub fn imf(&self, edition: &str) -> I18nImf {
        self.imf.unwrap_or(if edition.eq_ignore_ascii_case("gnome") {
            I18nImf::IBus
        } else {
            I18nImf::Fcitx5
        })
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum I18nImf {
    #[default]
    Fcitx5,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ims_from_locale() {
        let f = I18nImf::Fcitx5;
        assert_eq!(default_ims("zh_CN.UTF-8", f), ["Pinyin"]);
        assert_eq!(default_ims("zh_SG.UTF-8", f), ["Pinyin"]);
        assert_eq!(default_ims("zh_TW.UTF-8", f), ["Zhuyin"]);
        assert_eq!(default_ims("zh_HK.UTF-8", f), ["Cangjie 5"]);
        assert_eq!(default_ims("ja_JP.UTF-8", f), ["Mozc"]);
        assert_eq!(default_ims("ko_KR.UTF-8", I18nImf::IBus), ["libhangul"]);
        assert_eq!(default_ims("th_TH.UTF-8", I18nImf::IBus), ["Thai"]);
        assert!(default_ims("en_US.UTF-8", f).is_empty());
        assert!(default_ims("", f).is_empty());
        assert!(default_ims("C.UTF-8", f).is_empty());
    }

    #[test]
    fn every_default_im_exists() {
        for locale in
            ["zh_CN", "zh_TW", "zh_HK", "ja_JP", "ko_KR", "vi_VN", "bn_IN", "si_LK", "th_TH"]
        {
            for imf in [I18nImf::Fcitx5, I18nImf::IBus] {
                let ims = default_ims(locale, imf);
                assert_eq!(ims.len(), 1, "{locale} {imf:?}");
                assert!(find_im(&ims[0]).is_some_and(|im| im.available(imf)));
            }
        }
    }

    #[test]
    fn imf_default() {
        let cfg = I18nCfg::default();
        assert_eq!(cfg.imf("gnome"), I18nImf::IBus);
        assert_eq!(cfg.imf("plasma"), I18nImf::Fcitx5);
        assert_eq!(cfg.imf(""), I18nImf::Fcitx5);
        let cfg = I18nCfg { imf: Some(I18nImf::IBus) };
        assert_eq!(cfg.imf("plasma"), I18nImf::IBus);
    }

    #[test]
    fn fills_settings() {
        let mut settings =
            crate::settings::Settings { locale: "ja_JP.UTF-8".into(), ..Default::default() };
        fill_settings(&mut settings, &crate::cfg::Config::default());
        assert_eq!(settings.imf, I18nImf::Fcitx5);
        assert_eq!(settings.ims, ["Mozc"]);

        // picked ones stay, ones the framework doesn't have go
        let mut settings = crate::settings::Settings {
            locale: "ja_JP.UTF-8".into(),
            ims: vec!["Shuangpin".into(), "Pinyin".into(), "bogus".into()],
            ..Default::default()
        };
        let cfg =
            crate::cfg::Config { i18n: I18nCfg { imf: Some(I18nImf::IBus) }, ..Default::default() };
        fill_settings(&mut settings, &cfg);
        assert_eq!(settings.ims, ["Pinyin"]);

        // turning them all off means none, not the locale's
        let mut settings = crate::settings::Settings {
            locale: "ja_JP.UTF-8".into(),
            ims_picked: true,
            ..Default::default()
        };
        fill_settings(&mut settings, &crate::cfg::Config::default());
        assert!(settings.ims.is_empty());
    }
}
