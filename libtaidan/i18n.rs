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
        "ViQR"              => im!(ViQR         "table:viqr"=>"table-tv",                       "m17n_vi_viqr"=>"m17n"),
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
        // Cantonese is written in Traditional Chinese, and typed like in Hong Kong
        ("zh", "HK" | "MO") | ("yue", _) => &["Cangjie 5"],
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
}

/// layouts whose default variant can't type ASCII, which usernames, passwords that work
/// everywhere, and commands need
const NON_LATIN: &[&str] = &[
    "af", "am", "ara", "bd", "bg", "brai", "bt", "by", "eg", "et", "ge", "gn", "gr", "il", "in",
    "iq", "ir", "kg", "kh", "kz", "la", "lk", "ma", "mk", "mm", "mn", "mv", "my", "np", "pk", "rs",
    "ru", "sy", "th", "tj", "ua", "uz",
];

/// variants of [`NON_LATIN`] layouts that type Latin letters but don't say `latin` in their name
const LATIN_VARIANTS: &[(&str, &str)] = &[
    ("in", "eng"),
    ("iq", "ku"),
    ("iq", "ku_alt"),
    ("iq", "ku_f"),
    ("ir", "ku"),
    ("ir", "ku_alt"),
    ("ir", "ku_f"),
    ("lk", "us"),
    ("ma", "french"),
    ("ma", "rif"),
    ("ru", "ruchey_en"),
    ("sy", "ku"),
    ("sy", "ku_alt"),
    ("sy", "ku_f"),
    ("ua", "crh"),
    ("ua", "crh_alt"),
    ("ua", "crh_f"),
];

/// variants of other layouts that can't type ASCII, checked against xkeyboard-config's
/// `evdev.lst`. A variant ending in `*` stands for all that start with the rest, one starting with
/// `*` for all that end with it
const NON_LATIN_VARIANTS: &[(&str, &str)] = &[
    ("al", "veqilharxhi"),
    ("az", "cyrillic"),
    ("br", "rus"),
    ("ca", "ike"),
    ("cn", "mon_*"),
    ("cn", "tib"),
    ("cn", "tib_asciinum"),
    ("cn", "ug"),
    ("cz", "rus"),
    ("de", "ru"),
    ("de", "ru-recom"),
    ("de", "ru-translit"),
    ("dz", "ar"),
    ("dz", "ber"),
    ("fr", "geo"),
    ("id", "melayu-phonetic*"),
    ("id", "pegon-phonetic"),
    ("ie", "ogam*"),
    ("it", "geo"),
    ("jp", "kana"),
    ("jp", "kana86"),
    ("lv", "modern-cyr"),
    ("me", "cyrillic*"),
    ("ph", "*-bay"),
    ("pl", "ru_phonetic_dvorak"),
    ("se", "rus"),
    ("us", "chr"),
    ("us", "rus"),
];

/// whether `variant` matches `pattern`, which may start or end with a `*` wildcard
fn variant_matches(pattern: &str, variant: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        variant.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        variant.ends_with(suffix)
    } else {
        pattern == variant
    }
}

/// whether `layout` with `variant` (empty for the default one) can't type ASCII
fn is_non_latin(layout: &str, variant: &str) -> bool {
    if NON_LATIN.contains(&layout) {
        return !variant.contains("latin") && !LATIN_VARIANTS.contains(&(layout, variant));
    }
    NON_LATIN_VARIANTS.iter().any(|&(l, v)| l == layout && variant_matches(v, variant))
}

/// switches between the layouts of an [`XkbKeymap`] with a Latin one added, Super+Space as on
/// GNOME, which does it with its own shortcut instead
pub const LAYOUT_SWITCH: &str = "grp:win_space_toggle";

/// the xkb layouts, variants and options for a picked layout, each comma-separated
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XkbKeymap {
    pub layouts: String,
    pub variants: String,
    pub options: String,
}

impl XkbKeymap {
    /// `layout` and `variant`, with US English after it if it can't type ASCII, like Anaconda and
    /// GNOME Initial Setup do, else the user couldn't type a username until they switch back
    #[must_use]
    pub fn new(layout: &str, variant: Option<&str>) -> Self {
        let variant = variant.unwrap_or("");
        if is_non_latin(layout, variant) {
            Self {
                layouts: format!("{layout},us"),
                variants: format!("{variant},"),
                options: LAYOUT_SWITCH.to_owned(),
            }
        } else {
            Self {
                layouts: layout.to_owned(),
                variants: variant.to_owned(),
                options: String::new(),
            }
        }
    }

    /// whether US English was added after the picked layout
    #[must_use]
    pub fn has_latin_fallback(&self) -> bool {
        !self.options.is_empty()
    }

    /// `kwriteconfig6` arguments, after `--file`, for `kxkbrc`
    fn kxkbrc_entries(&self) -> Vec<Vec<String>> {
        let entry = |key: &str, rest: &[&str]| {
            ["--group", "Layout", "--key", key].iter().chain(rest).map(|&s| s.to_owned()).collect()
        };
        // `Options` even when empty, so a switch toggle left from an earlier pick goes away. KWin
        // only uses it with `ResetOldOptions`, else it keeps `XKB_DEFAULT_OPTIONS`
        let mut entries = vec![
            entry("Use", &["--type=bool", "true"]),
            entry("LayoutList", &["--", &self.layouts]),
            entry("VariantList", &["--", &self.variants]),
            entry("Options", &["--", &self.options]),
        ];
        if self.has_latin_fallback() {
            entries.push(entry("ResetOldOptions", &["--type=bool", "true"]));
        }
        entries
    }

    /// GNOME's `input-sources` entries, `('xkb', 'layout+variant')` for each layout
    #[must_use]
    pub fn gnome_sources(&self) -> Vec<String> {
        self.layouts
            .split(',')
            .zip(self.variants.split(',').chain(std::iter::repeat("")))
            .map(|(layout, variant)| {
                let name = if variant.is_empty() {
                    layout.to_owned()
                } else {
                    format!("{layout}+{variant}")
                };
                // a GVariant string, xkb names never need escaping but the settings may be odd
                let name = name.replace('\\', "\\\\").replace('\'', "\\'");
                format!("('xkb', '{name}')")
            })
            .collect()
    }
}

async fn set_kde_keymap(user: &str, layout: &str, variant: Option<&str>) -> Res<()> {
    for entry in XkbKeymap::new(layout, variant).kxkbrc_entries() {
        let mut args = vec!["--file", "kxkbrc"];
        args.extend(entry.iter().map(String::as_str));
        pkexec(user, "kwriteconfig6", &args).await?;
    }
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
    let sources = XkbKeymap::new(layout, variant).gnome_sources().join(", ");
    let args = [
        ["DISPLAY=:0", "gsettings"],
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[{sources}]")],
    ];
    pkexec(user, "env", &args.concat()).await?;
    Ok(())
}

/// sets the system's keymap (locale1), which the login screen and console use
///
/// # Errors
/// - `localectl` fails
pub async fn set_localectl_keymap(layout: &str, variant: Option<&str>) -> Res<()> {
    let xkb = XkbKeymap::new(layout, variant);
    let args = ["set-x11-keymap", &xkb.layouts, "", &xkb.variants, &xkb.options];
    root("localectl", &args).await?;
    Ok(())
}

/// `kwriteconfig6` arguments, after `--file`, that put the layout picked in `settings` into
/// `kxkbrc`, [`None`] if there's no layout
fn kxkbrc_entries(settings: &crate::settings::Settings) -> Option<Vec<Vec<String>>> {
    if settings.kb_layout.is_empty() {
        return None;
    }
    Some(XkbKeymap::new(&settings.kb_layout, settings.kb_variant.as_deref()).kxkbrc_entries())
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
        args.extend(entry.iter().map(String::as_str));
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
        args.extend(entry.iter().map(String::as_str));
        crate::steps::acmd("kwriteconfig6", &args).await?;
    }
    Ok(true)
}

#[allow(clippy::equatable_if_let)]
#[allow(clippy::missing_panics_doc)]
/// applies the keymap to `user`'s session (the current user's for [`None`]), and to the system
/// (locale1, which the login screen and console use) if `system`. Returns whether the system's
/// was saved, `true` if it wasn't asked for. A failure there is logged and the session still gets
/// the keymap, the installation's `SetLocale` step tries the system again
///
/// # Errors
/// - cannot get the current username
/// - cannot apply the keymap to the session
pub async fn set_keymap(
    user: Option<&str>,
    layout: &str,
    variant: Option<&str>,
    system: bool,
) -> Res<bool> {
    let current_user =
        uzers::get_current_username().ok_or(Err::Msg("can't get current username"))?;
    if current_user.as_encoded_bytes() != b"taidan" && cfg!(debug_assertions) {
        // #74
        tracing::warn!("Skipping set_keymap in debug mode without being `taidan` user");
        return Ok(true);
    }
    let mut saved = true;
    if system
        && let Ok(true) = tokio::fs::try_exists("/usr/bin/localectl").await
        && let Err(e) = set_localectl_keymap(layout, variant).await
    {
        tracing::error!(?e, layout, ?variant, "cannot set the system keymap");
        saved = false;
    }

    let user = match user {
        Some(user) => user,
        None => current_user.to_str().ok_or(Err::Msg("current username is not valid UTF-8"))?,
    };
    if let Ok(true) = tokio::fs::try_exists("/usr/bin/kwriteconfig6").await {
        set_kde_keymap(user, layout, variant).await?;
    } else {
        set_gsettings_keymap(user, layout, variant).await?;
    }
    Ok(saved)
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
        self.imf.unwrap_or(if crate::cfg::desktop(edition) == "gnome" {
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
    fn latin_fallback() {
        let ru = XkbKeymap::new("ru", None);
        assert_eq!((ru.layouts.as_str(), ru.variants.as_str()), ("ru,us", ","));
        assert_eq!(ru.options, LAYOUT_SWITCH);
        assert_eq!(ru.gnome_sources(), ["('xkb', 'ru')", "('xkb', 'us')"]);
        let rs = XkbKeymap::new("rs", Some("latin"));
        assert_eq!((rs.layouts.as_str(), rs.options.as_str()), ("rs", ""));
        let de = XkbKeymap::new("de", Some("nodeadkeys"));
        assert!(!de.has_latin_fallback());
        assert_eq!(de.gnome_sources(), ["('xkb', 'de+nodeadkeys')"]);
        assert!(XkbKeymap::new("jp", Some("kana")).has_latin_fallback());
        assert!(XkbKeymap::new("cn", Some("mon_trad")).has_latin_fallback());
        assert!(!XkbKeymap::new("jp", None).has_latin_fallback());
        assert!(!XkbKeymap::new("iq", Some("ku_f")).has_latin_fallback());
        assert!(XkbKeymap::new("eg", None).has_latin_fallback());
        let gr = XkbKeymap::new("gr", Some("polytonic"));
        assert_eq!(gr.gnome_sources(), ["('xkb', 'gr+polytonic')", "('xkb', 'us')"]);
    }

    #[test]
    fn non_latin_variants() {
        for (layout, variant) in [
            ("brai", None),
            ("brai", Some("left_hand")),
            ("dz", Some("ar")),
            ("dz", Some("ber")),
            ("ph", Some("qwerty-bay")),
            ("ph", Some("capewell-qwerf2k6-bay")),
            ("fr", Some("geo")),
            ("it", Some("geo")),
            ("id", Some("melayu-phonetic")),
            ("id", Some("melayu-phoneticx")),
            ("id", Some("pegon-phonetic")),
            ("ie", Some("ogam")),
            ("ie", Some("ogam_is434")),
            ("lv", Some("modern-cyr")),
            ("me", Some("cyrillic")),
            ("me", Some("cyrillicalternatequotes")),
            ("al", Some("veqilharxhi")),
            ("az", Some("cyrillic")),
        ] {
            assert!(XkbKeymap::new(layout, variant).has_latin_fallback(), "{layout}({variant:?})");
        }
        for (layout, variant) in [
            ("dz", None),
            ("dz", Some("azerty-deadkeys")),
            ("ph", None),
            ("ph", Some("capewell-dvorak")),
            ("id", Some("javanese")),
            ("ie", None),
            ("lv", Some("modern")),
            ("me", Some("latinyz")),
            ("al", Some("plisi")),
        ] {
            assert!(!XkbKeymap::new(layout, variant).has_latin_fallback(), "{layout}({variant:?})");
        }
    }

    #[test]
    fn every_non_latin_variant_exists() {
        // the patterns follow xkeyboard-config, each should still match one of its variants. These
        // aren't in `evdev.lst`, the `ru` ones are only in the extras and `kana86` in older ones
        let extras = [("de", "ru-recom"), ("de", "ru-translit"), ("jp", "kana86")];
        for &(layout, pattern) in NON_LATIN_VARIANTS {
            if extras.contains(&(layout, pattern)) {
                continue;
            }
            let variants = &LAYOUTS.get(layout).unwrap_or_else(|| panic!("no {layout}")).variants;
            assert!(
                variants.keys().any(|v| variant_matches(pattern, v)),
                "{layout}({pattern}) matches no variant"
            );
        }
        for layout in NON_LATIN {
            assert!(LAYOUTS.contains_key(layout), "no {layout}");
        }
        assert!(!LAYOUTS.contains_key("custom"));
    }

    #[test]
    fn kxkbrc_clears_options() {
        let has = |km: &XkbKeymap, key: &str| {
            km.kxkbrc_entries().iter().find(|e| e[3] == key).map(|e| e.last().cloned())
        };
        let de = XkbKeymap::new("de", None);
        assert_eq!(has(&de, "Options"), Some(Some(String::new())));
        assert_eq!(has(&de, "ResetOldOptions"), None);
        let ru = XkbKeymap::new("ru", None);
        assert_eq!(has(&ru, "Options"), Some(Some(LAYOUT_SWITCH.to_owned())));
        assert_eq!(has(&ru, "ResetOldOptions"), Some(Some("true".to_owned())));
    }

    #[test]
    fn ims_from_locale() {
        let f = I18nImf::Fcitx5;
        assert_eq!(default_ims("zh_CN.UTF-8", f), ["Pinyin"]);
        assert_eq!(default_ims("zh_SG.UTF-8", f), ["Pinyin"]);
        assert_eq!(default_ims("zh_TW.UTF-8", f), ["Zhuyin"]);
        assert_eq!(default_ims("zh_HK.UTF-8", f), ["Cangjie 5"]);
        assert_eq!(default_ims("yue_HK.UTF-8", f), ["Cangjie 5"]);
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
        assert_eq!(cfg.imf("atomic-gnome"), I18nImf::IBus);
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
