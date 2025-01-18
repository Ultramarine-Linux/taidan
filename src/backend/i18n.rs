use gettextrs::gettext;

use super::pkexec;

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
// NOTE: some characters may not be displayed properly on your device, please make sure you have
// the corresponding fonts installed.
pub const IMS: phf::Map<&'static str, phf::Map<&'static str, InputMethod>> = phf::phf_map! {
    "Chinese" => phf::phf_map! {
        // IME                     Native       Ibus(=>pkg)                         Fcitx5(=>pkg)
        "Pinyin"            => im!(拼音         "pinyin",                           "pinyin"=>"chinese-addons"),
        // there's also libzhuyin but it's extremely unpopular, sorry
        "Zhuyin"            => im!(注音         "chewing",                          "chewing"),
        "Cangjie 3"         => im!(倉頡版本三   "cangjie"=>"table-chinese-cangjie", "cangjie3"=>"table-extra"),
        "Cangjie 5"         => im!(倉頡版本五   "cangjie"=>"table-chinese-cangjie", "cangjie5"=>"table-extra"),
        "Quick 3"           => im!(速成版本三   "quick3"=>"table-chinese-quick",    "quick3"=>"table-extra"),
        "Quick 5"           => im!(速成版本五   "quick5"=>"table-chinese-quick",    "quick5"=>"table-extra"),
        "Quick Classic"     => im!(速成舊版     "quick5"=>"table-chinese-quick",    "quick-classic"=>"table-extra"),
        "Rime"              => im!(Rime         "rime",                             "rime"),
        "Shuangpin"         => im!(双拼         (),                                 "shuangpin"=>"chinese-addons"),
        "Smart Cangjie 6"   => im!(快倉第六代   "cangjie"=>"table-chinese-scj",     "scj6"=>"table-extra"),
        "Array 30"          => im!(行列三十     "array"=>"table-chinese-array",     "array30"=>"table-extra"),
        // 對唔住，超超超垃圾，都唔知係咪俾人用
        //"Jyutping"          => im!(粵拼         "jyutping"=>"table-chinese-cantonese","jyutping-table"=>"table-extra")
        // 叫下啲人用 rime 啦
        "Boshiamy"          => im!(嘸蝦米       (),                                 "boshiamy"=>"table-extra"),
    },
    "Japanese" => phf::phf_map! {
        "Mozc"              => im!(Mozc         "mozc",                             "mozc"),
        // "Anthy"             => im!(Anthy        "anthy",                            "anthy"),
    },
    "Korean" => phf::phf_map! {
        "libhangul"         => im!(한글         "hangul",                           "hangul"),
    },
    "Vietnamese" => phf::phf_map! {
        "Unikey"            => im!(Unikey       "unikey",                           "unikey"),
        "ViQR"              => im!(ViQR         "viqr"=>"table-tv",                 "viqr"=>"m17n"),
    },
    "Indic" => phf::phf_map! {
        "OpenBangla"        => im!(বাংলা        "openbangla",                       "openbangla"),
        "Sayura Sinhara"    => im!(සිංහල         "sayura",                           "sayura"),
        // "Others"            => im!(Others       "m17n",                             "m17n"),
    },
    "Thai" => phf::phf_map! {
        "Thai"              => im!(ภาษาไทย      "thai"=>"table-tv",                 "libthai"=>"libthai")
    },
};

pub const STR_TO_LANG: phf::Map<&'static str, IMELanguages> = phf::phf_map! {
    "Chinese" => IMELanguages::Chinese,
    "Japanese" => IMELanguages::Japanese,
    "Korean" => IMELanguages::Korean,
    "Vietnamese" => IMELanguages::Vietnamese,
    "Indic" => IMELanguages::Indic,
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
    Indic,
    Thai,
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
            // This is intentional and you are required to fill in this field.
            // %lang_name is the language name you probably have just translated (see
            // the nearby strings above this). %native_lang_name is the language name
            // in its corresponding language. For example, when displaying in English:
            //
            // Chinese (中文)
            // Japanese (日本語)
            // Korean (한국어)
            // ...
            //
            // In other languages, you might need to change the order around and maybe
            // use different brackets, etc.
            gettext("%lang_name (%native_lang_name)")
                .replace("%lang_name", &self.name())
                .replace("%native_lang_name", native)
        })
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
    pub fn available(self) -> bool {
        match &*super::CFG.edition {
            "plasma" | "kde" => self.fcitx5_ref.is_some(),
            _ => self.ibus_ref.is_some(),
        }
    }
    /// Gets the package for the current edition
    ///
    /// # Panics
    /// Panics if and only if this IM is not available for the current edition.
    #[must_use]
    pub fn get_pkg(self) -> &'static str {
        match &*super::CFG.edition {
            "plasma" | "kde" => self.fcitx5_pkg.unwrap(),
            _ => self.ibus_pkg.unwrap(),
        }
    }
    /// Gets the internal reference of the IM for the current edition
    ///
    /// # Panics
    /// Panics if and only if this IM is not available for the current edition.
    #[must_use]
    pub fn get_ref(self) -> &'static str {
        match &*super::CFG.edition {
            "plasma" | "kde" => self.fcitx5_ref.unwrap(),
            _ => self.ibus_ref.unwrap(),
        }
    }
    pub fn handle_switch_state(
        self,
        imname: &'static str,
    ) -> impl Fn(&relm4::gtk::Switch, bool) -> libhelium::glib::Propagation + 'static {
        use crate::prelude::*;
        move |_, state| {
            if state {
                SETTINGS.write().ims.push(imname);
            } else {
                let value = SETTINGS.read().ims.iter().position(|&s| s == imname);
                if let Some(i) = value {
                    SETTINGS.write().ims.swap_remove(i);
                } else {
                    tracing::warn!(?imname, "cannot find unselected IM");
                }
            }
            glib::Propagation::Proceed
        }
    }
    pub fn make_listboxrow(self, imname: &'static str) -> relm4::gtk::ListBoxRow {
        use crate::prelude::*;
        let ims = SETTINGS.read().ims.clone();
        gtk::ListBoxRow::builder()
            .child(&libhelium::MiniContentBlock::with_details(
                Some(self.native_name),
                Some(imname),
                None::<&libhelium::Button>,
                Some(&{
                    let switch = gtk::Switch::new();
                    switch.set_active(ims.contains(&imname));
                    switch.connect_state_set(self.handle_switch_state(imname));
                    switch
                }),
            ))
            .build()
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

#[allow(clippy::equatable_if_let)]
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
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
    if let Ok(true) = tokio::fs::try_exists("kwriteconfig6").await {
        set_kde_keymap(user, layout, variant).await
    } else {
        set_gsettings_keymap(user, layout, variant).await
    }
}
