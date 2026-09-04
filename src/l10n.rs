//! Runtime l10n module.
//!
//! The compile-time loader is available as [`crate::LL`].
use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{FileSystemAssets, LanguageLoader as _, unic_langid::LanguageIdentifier};
use icu::experimental::displaynames as dn;
use itertools::Itertools;
use parking_lot::RwLock;
use std::sync::{Arc, LazyLock};

#[cfg(not(debug_assertions))]
const PO_PATH: &str = "/usr/share/taidan/po/";

#[cfg(debug_assertions)]
const PO_PATH: &str = "po/";

type B = Box<dyn i18n_embed::I18nAssets + Send + Sync>;

static PO_ASSETS: LazyLock<Arc<B>> = LazyLock::new(|| {
    Arc::new(
        FileSystemAssets::try_new(PO_PATH)
            .inspect_err(|e| tracing::error!(?e, "Cannot load assets in {PO_PATH}"))
            .inspect_err(|_| tracing::warn!("Falling back to global compile-time assets"))
            .map_or_else(|_| Box::new(crate::Localizations) as B, |a| Box::new(a) as B),
    )
});

static PO_AVAILABLE_LANGS: LazyLock<Vec<LanguageIdentifier>> =
    LazyLock::new(|| fluent_language_loader!().available_languages(&***PO_ASSETS).unwrap());

/// Create a new loader according to the list of languages.
///
/// # Panics
/// If languages that cannot be loaded are encounted, panics immediately.
#[must_use]
pub fn new_loader(mut langs: Vec<LanguageIdentifier>) -> FluentLanguageLoader {
    let loader = fluent_language_loader!();
    if langs.is_empty() {
        langs = vec![loader.fallback_language().clone()];
    }
    loader.load_languages(&***PO_ASSETS, &langs).unwrap();
    loader
}

pub static PO_LOADER: LazyLock<RwLock<FluentLanguageLoader>> = LazyLock::new(|| {
    RwLock::new(new_loader(
        poly_l10n::system_want_langids()
            .flat_map(|li| crate::LOCALE_SOLVER.solve_locale(li))
            .filter(|li| PO_AVAILABLE_LANGS.contains(li))
            .collect_vec(),
    ))
});

#[derive(Debug)]
pub struct LanguageRow {
    pub locale: &'static str,
    pub native_name: &'static str,
}

taidan_proc_macros::comptime_localedef_langrows!(LANGS);

#[macro_export]
macro_rules! t {
    (@$msgid:literal) => {
        if $crate::backend::l10n::PO_LOADER.read().has($msgid) {
            $crate::backend::l10n::PO_LOADER.read().get($msgid)
        } else {
            i18n_embed_fl::fl!($crate::LL.read(), $msgid)
        }
    };
    ($msgid:literal $(, $k:ident = $v:expr)*$(,)?) => {
        if $crate::backend::l10n::PO_LOADER.read().has($msgid) {
            $crate::backend::l10n::PO_LOADER.read()
                .get_args_concrete($msgid, [$((stringify!($k), $v.into())),*].into())
        } else {
            i18n_embed_fl::fl!($crate::LL.read(), $msgid $(, $k = $v)*)
        }
    };
    (@$msgid:expr) => {
        if $crate::backend::l10n::PO_LOADER.read().has($msgid) {
            $crate::backend::l10n::PO_LOADER.read().get($msgid)
        } else {
            $crate::LL.read().get($msgid)
        }
    };
    ($msgid:expr $(, $k:ident = $v:expr)*$(,)?) => {
        if $crate::backend::l10n::PO_LOADER.read().has($msgid) {
            $crate::backend::l10n::PO_LOADER.read()
                .get_args_concrete($msgid, [$((stringify!($k), $v.into())),*].into())
        } else {
            $crate::LL.read()
                .get_args_concrete($msgid, [$((stringify!($k), $v.into())),*].into())
        }
    };
}

pub fn set_lang(lang: &str) {
    if let Ok(locale) = (lang)
        .split_once('.')
        .map_or(lang, |(left, _)| left)
        .to_owned()
        .parse::<i18n_embed::unic_langid::LanguageIdentifier>()
        .inspect_err(|e| tracing::error!(?e, ?lang, "Cannot apply language"))
    {
        tracing::info!(?locale, lang, "Using selected locale");
        let mut locales = crate::LOCALE_SOLVER
            .solve_locale(locale)
            .into_iter()
            .filter(|li| crate::AVAILABLE_LANGS.contains(li))
            .collect_vec();
        let loader = i18n_embed::fluent::fluent_language_loader!();
        if locales.is_empty() {
            locales.push("en-US".parse().unwrap());
        }
        loader.load_languages(&crate::Localizations, &locales).expect("fail to load languages");
        *crate::LL.write() = loader;
        *PO_LOADER.write() = new_loader(locales);
        // TODO: localectl set locale
    }
}

thread_local! {
    static LANG_DISPLAY_HDL: std::cell::RefCell<
        Option<(String, dn::multi::LocaleDisplayNamesFormatter)>,
    > = const { std::cell::RefCell::new(None) };
}

fn display_hdl<T>(
    current_lang: &crate::ui::Language,
    f: impl FnOnce(&dn::multi::LocaleDisplayNamesFormatter) -> T,
) -> T {
    LANG_DISPLAY_HDL.with_borrow_mut(|hdl| {
        if let Some((lang, hdl)) = hdl.as_ref()
            && lang == current_lang.locale.as_str()
        {
            f(hdl)
        } else {
            let mut opt = dn::DisplayNamesOptions::default();
            opt.style = Some(dn::Style::Short);
            let locale =
                if current_lang.locale == "en-owo" { "en-US" } else { &current_lang.locale };
            let locale = icu::locale::Locale::try_from_str(locale)
                .unwrap_or_else(|e| panic!("cannot parse locale {current_lang:?}: {e}"));
            assert!(locale != icu::locale::Locale::UNKNOWN, "locale {current_lang:?} is unk");

            let fmt = dn::multi::LocaleDisplayNamesFormatter::try_new((&locale).into(), opt)
                .expect("cannot get hdl");
            let res = f(&fmt);
            *hdl = Some((current_lang.locale.to_string(), fmt));
            res
        }
    })
}

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, lang: crate::ui::Lang<'_>) {
    use crate::ui::{FluentArg, Keymap, KeymapVariant, Language};
    use slint::{Model, ToSharedString};
    lang.on__t(|id, args, _current_language| {
        tracing::trace!(?id);
        let mut fargs = fluent::FluentArgs::new();
        args.iter().for_each(|FluentArg { key, value }| {
            // for some reason k, v are owned values
            // weird design of `Model::iter()`
            fargs.set(key.to_string(), value.to_string());
        });
        if crate::l10n::PO_LOADER.read().has(&id) {
            crate::l10n::PO_LOADER.read().get_args_fluent(&id, Some(&fargs)).into()
        } else {
            crate::LL.read().get(&id).into()
        }
    });
    let eng = crate::l10n::LANGS.iter().find(|lang| lang.locale == "en-US").expect("no english");
    lang.set_current_language(Language {
        locale: eng.locale.to_shared_string(),
        native_name: eng.native_name.to_shared_string(),
    });
    lang.on__name(|lang, current_language| {
        if lang.locale == "en-owo" {
            return "OWO".into();
        }
        display_hdl(&current_language, |hdl| {
            let locale = icu::locale::Locale::try_from_str(&lang.locale)
                .unwrap_or_else(|e| panic!("cannot parse locale {lang:?}: {e}"));
            assert!(locale != icu::locale::Locale::UNKNOWN, "locale {lang:?} is unk");
            hdl.of(&locale).to_shared_string()
        })
    });
    lang.set_languages(slint::ModelRc::new(
        crate::l10n::LANGS
            .iter()
            .map(|crate::l10n::LanguageRow { locale, native_name }| Language {
                locale: locale.to_shared_string(),
                native_name: native_name.to_shared_string(),
            })
            .collect::<slint::VecModel<_>>(),
    ));
    lang.on_set_lang(move |lang| {
        use slint::ComponentHandle;
        if lang.locale == "en-owo" {
            crate::l10n::set_lang("en-Xowo");
        } else {
            crate::l10n::set_lang(&lang.locale);
        }
        ui.upgrade_in_event_loop(|ui| ui.window().request_redraw()).expect("event loop err");
    });
    lang.set_keymaps(slint::ModelRc::new(
        libtaidan::i18n::LAYOUTS
            .entries()
            .map(|(id, layout)| Keymap {
                id: id.to_shared_string(),
                name: layout.name.to_shared_string(),
                variants: slint::ModelRc::new(
                    layout
                        .variants
                        .entries()
                        .map(|(id, name)| KeymapVariant {
                            id: id.to_shared_string(),
                            name: name.to_shared_string(),
                        })
                        .collect::<slint::VecModel<_>>(),
                ),
            })
            .collect::<slint::VecModel<_>>(),
    ));
}
