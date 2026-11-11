//! Runtime l10n module.
use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{FileSystemAssets, LanguageLoader as _, unic_langid::LanguageIdentifier};
use icu::experimental::displaynames as dn;
use itertools::Itertools;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::rc::Rc;
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

// i18n_embed's `available_languages` can't list a `FileSystemAssets`
static PO_AVAILABLE_LANGS: LazyLock<Vec<LanguageIdentifier>> =
    LazyLock::new(|| po_langs(&***PO_ASSETS));

fn po_langs(assets: &dyn i18n_embed::I18nAssets) -> Vec<LanguageIdentifier> {
    let loader = fluent_language_loader!();
    let langs = crate::AVAILABLE_LANGS
        .iter()
        .filter(|li| !assets.get_files(&format!("{li}/{}.ftl", loader.domain())).is_empty())
        .cloned()
        .collect_vec();
    if langs.len() < crate::AVAILABLE_LANGS.len() {
        tracing::warn!(?langs, "some translations are missing in {PO_PATH}");
    }
    langs
}

#[must_use]
pub fn new_loader(langs: Vec<LanguageIdentifier>) -> FluentLanguageLoader {
    let loader = fluent_language_loader!();
    let fallback = loader.fallback_language().clone();
    let mut present =
        langs.iter().filter(|li| PO_AVAILABLE_LANGS.contains(li)).cloned().collect_vec();
    if present.is_empty() {
        present = vec![fallback.clone()];
    }
    let Err(e) = loader.load_languages(&***PO_ASSETS, &present) else { return loader };
    tracing::error!(
        ?e,
        ?present,
        "cannot load the translations in {PO_PATH}, using the embedded ones"
    );
    let fresh_loader = fluent_language_loader!();
    let mut embedded =
        langs.into_iter().filter(|li| crate::AVAILABLE_LANGS.contains(li)).collect_vec();
    if embedded.is_empty() {
        embedded = vec![fallback];
    }
    if let Err(e) = fresh_loader.load_languages(&crate::Localizations, &embedded) {
        tracing::error!(?e, ?embedded, "cannot load the embedded translations either");
    }
    fresh_loader
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
    pub en_name: &'static str,
}

taidan_proc_macros::comptime_localedef_langrows!(LANGS);

impl LanguageRow {
    fn is_legacy_glibc_alias(&self) -> bool {
        let has_territory = self.locale.contains('-');
        let is_bare_language_code = self.locale.len() <= 3;
        !has_territory && !is_bare_language_code
    }

    fn bcp47(&self) -> slint::SharedString {
        self.locale.replace('_', "-").into()
    }
}

const OWO_TAG: &str = "en-Xowo";

fn posix_locale(lang: &str) -> Option<String> {
    let tag = lang.split_once('.').map_or(lang, |(left, _)| left);
    if tag == OWO_TAG {
        return Some("en_US.UTF-8".to_owned());
    }
    if tag.is_empty() || tag.split('-').any(|part| part.starts_with('X')) {
        return None;
    }
    Some(format!("{}.UTF-8", tag.replace('-', "_")))
}

const CROSS_LANGUAGE_FALLBACKS: &[(&str, &str)] = &[("yue", "zh-Hant")];

fn ui_langs(
    locale: LanguageIdentifier,
    available: &[LanguageIdentifier],
) -> Vec<LanguageIdentifier> {
    let base = locale.language;
    let extra = CROSS_LANGUAGE_FALLBACKS
        .iter()
        .filter(move |(lang, _)| base.as_str() == *lang)
        .filter_map(|(_, fallback)| fallback.parse::<LanguageIdentifier>().ok());
    std::iter::once(locale.clone())
        .chain(
            crate::LOCALE_SOLVER.solve_locale(locale).into_iter().filter(|li| li.language == base),
        )
        .chain(extra)
        .chain("en-US".parse::<LanguageIdentifier>().ok())
        .filter(|li| available.contains(li))
        .unique()
        .collect_vec()
}

pub fn set_lang(lang: &str) -> String {
    let mut shown = "en-US".to_owned();
    if let Ok(locale) = lang
        .split_once('.')
        .map_or(lang, |(left, _)| left)
        .to_owned()
        .parse::<LanguageIdentifier>()
        .inspect_err(|e| tracing::error!(?e, ?lang, "Cannot apply language"))
    {
        tracing::info!(?locale, lang, "Using selected locale");
        let locales = ui_langs(locale, &crate::AVAILABLE_LANGS);
        if let Some(first) = locales.first() {
            shown = first.to_string();
        }
        *PO_LOADER.write() = new_loader(locales);
    }
    let posix = posix_locale(lang);
    if let Some(posix) = &posix {
        crate::a11y::language_changed(posix);
    }
    crate::SETTINGS.write().locale = posix.unwrap_or_default();
    shown
}

fn is_rtl(lang: &str) -> bool {
    let tag = lang.split_once('.').map_or(lang, |(left, _)| left);
    icu::locale::LanguageIdentifier::try_from_str(tag)
        .inspect_err(|e| tracing::error!(?e, lang, "cannot parse locale for its direction"))
        .is_ok_and(|li| icu::locale::LocaleDirectionality::new_extended().is_right_to_left(&li))
}

thread_local! {
    static LANG_DISPLAY_HDL: RefCell<
        Option<(String, dn::multi::LocaleDisplayNamesFormatter)>,
    > = const { RefCell::new(None) };
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
            assert_ne!(locale, icu::locale::Locale::UNKNOWN, "locale {current_lang:?} is unk");

            let fmt = dn::multi::LocaleDisplayNamesFormatter::try_new((&locale).into(), opt)
                .or_else(|e| {
                    tracing::debug!(?e, ?locale, "no display names, using English ones");
                    dn::multi::LocaleDisplayNamesFormatter::try_new(
                        icu::locale::locale!("en-US").into(),
                        opt,
                    )
                })
                .expect("cannot get hdl");
            let res = f(&fmt);
            *hdl = Some((current_lang.locale.to_string(), fmt));
            res
        }
    })
}

struct KeymapReq {
    layout: String,
    variant: Option<String>,
    also_system: bool,
}

fn coalesce_to_latest(mut req: KeymapReq, rx: &std::sync::mpsc::Receiver<KeymapReq>) -> KeymapReq {
    while let Ok(newer) = rx.try_recv() {
        let also_system = req.also_system || newer.also_system;
        req = KeymapReq { also_system, ..newer };
    }
    req
}

static KEYMAP_UI: std::sync::OnceLock<slint::Weak<crate::ui::AppWindow>> =
    std::sync::OnceLock::new();

static KEYMAP_TX: LazyLock<std::sync::mpsc::Sender<KeymapReq>> = LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(req) = rx.recv() {
            let KeymapReq { layout, variant, also_system } = coalesce_to_latest(req, &rx);
            let saved = crate::RT
                .block_on(libtaidan::i18n::set_keymap(
                    None,
                    &layout,
                    variant.as_deref(),
                    also_system,
                ))
                .inspect_err(|e| tracing::error!(?e, layout, ?variant, "cannot set keymap"))
                .unwrap_or(true);
            if also_system && let Some(ui) = KEYMAP_UI.get() {
                let res = ui.upgrade_in_event_loop(move |ui| {
                    use slint::ComponentHandle as _;
                    ui.global::<crate::ui::Lang<'_>>().set_keymap_not_saved(!saved);
                });
                if let Err(e) = res {
                    tracing::error!(?e, "cannot tell the UI about the keymap");
                }
            }
        }
    });
    tx
});

fn save_keymap_without_applying(layout: &str, variant: Option<&str>) {
    let mut settings = crate::SETTINGS.write();
    layout.clone_into(&mut settings.kb_layout);
    settings.kb_variant = variant.map(ToOwned::to_owned);
}

fn has_latin_fallback(layout: &str, variant: Option<&str>) -> bool {
    libtaidan::i18n::XkbKeymap::new(layout, variant).has_latin_fallback()
}

fn set_keymap(layout: &str, variant: Option<&str>, also_system: bool) {
    save_keymap_without_applying(layout, variant);
    if also_system {
        crate::SETTINGS.write().kb_picked = true;
    }
    let req = KeymapReq {
        layout: layout.to_owned(),
        variant: variant.map(ToOwned::to_owned),
        also_system,
    };
    if let Err(e) = KEYMAP_TX.send(req) {
        tracing::error!(?e, "keymap worker is gone");
    }
}

fn first_of_comma_list(list: &str) -> Option<String> {
    let first = list.split(',').next().unwrap_or_default().trim();
    (!first.is_empty()).then(|| first.to_owned())
}

fn system_keymap() -> Option<(String, Option<String>)> {
    let km = crate::RT.block_on(libtaidan::i18n::XkbKeymap::system())?;
    let keymap = first_of_comma_list(&km.layouts).map(|l| (l, first_of_comma_list(&km.variants)));
    tracing::debug!(?keymap, "System keymap");
    keymap
}

type Query = Rc<RefCell<String>>;

const LATIN_GREEK_CYRILLIC_COMBINING_DIACRITICS: std::ops::RangeInclusive<char> =
    '\u{300}'..='\u{36f}';

#[must_use]
pub fn fold_search(text: &str) -> String {
    use icu::normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};
    let lower = text.to_lowercase();
    let stripped = DecomposingNormalizerBorrowed::new_nfd()
        .normalize(&lower)
        .chars()
        .filter(|c| !LATIN_GREEK_CYRILLIC_COMBINING_DIACRITICS.contains(c))
        .collect::<String>();
    ComposingNormalizerBorrowed::new_nfc().normalize(&stripped).into_owned()
}

fn matches(folded_query: &str, haystacks: &[&str]) -> bool {
    folded_query.is_empty() || haystacks.iter().any(|h| fold_search(h).contains(folded_query))
}

fn strip_legacy_tt_tags(translated: &str) -> String {
    translated.replace("<tt>", "").replace("</tt>", "")
}

fn refresh_localized_time_zone_names(ui: &crate::ui::AppWindow) {
    use slint::ComponentHandle as _;
    let tz = ui.global::<crate::ui::Tz<'_>>();
    if !tz.get_current().id.is_empty() {
        tz.invoke_refresh();
    }
}

pub fn initialize_ui(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::{FluentArg, Language};
    use slint::{ComponentHandle, Model, ToSharedString};
    lang.on__t(|id, args, _current_language| {
        tracing::trace!(?id);
        let mut fargs = fluent::FluentArgs::new();
        args.iter().for_each(|FluentArg { key, value }| {
            fargs.set(key.to_string(), value.to_string());
        });
        let s = PO_LOADER.read().get_args_fluent(&id, Some(&fargs));
        strip_legacy_tt_tags(&s).into()
    });
    let initial = poly_l10n::system_want_langids()
        .find_map(|li| {
            let tag = li.to_string();
            LANGS.iter().find(|row| row.bcp47() == tag)
        })
        .or_else(|| LANGS.iter().find(|lang| lang.locale == "en-US"))
        .expect("no english");
    let shown_translation = set_lang(&initial.bcp47());
    lang.set_rtl(is_rtl(&shown_translation));
    lang.set_current_language(Language {
        locale: initial.bcp47(),
        native_name: initial.native_name.to_shared_string(),
    });
    lang.on__name(|lang, current_language| display_name(&lang, &current_language));
    init_language_list(ui, lang);
    lang.on_set_lang({
        let ui = ui.clone();
        move |lang| {
            let loaded =
                if lang.locale == "en-owo" { set_lang(OWO_TAG) } else { set_lang(&lang.locale) };
            if let Some(ui) = ui.upgrade() {
                ui.global::<crate::ui::Lang<'_>>().set_rtl(is_rtl(&loaded));
                crate::ims::follow_locale_unless_picked(&ui);
            }
            ui.upgrade_in_event_loop(|ui| {
                refresh_localized_time_zone_names(&ui);
                ui.window().request_redraw();
            })
            .expect("event loop err");
        }
    });
    if KEYMAP_UI.set(ui.clone()).is_err() {
        tracing::warn!("keymaps initialized twice");
    }
    init_keymaps(ui, lang);
}

fn init_language_list(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::Language;
    use slint::{ComponentHandle, FilterModel, ModelRc, ToSharedString, VecModel};
    let all_langs = LANGS
        .iter()
        .filter(|row| row.locale == "en-owo" || !row.is_legacy_glibc_alias())
        .map(|row| Language {
            locale: row.bcp47(),
            native_name: row.native_name.to_shared_string(),
        })
        .collect::<VecModel<_>>();
    let query = Query::default();
    let query_lang = Rc::new(RefCell::new(None::<Language>));
    let model = Rc::new(FilterModel::new(all_langs, {
        let (query, query_lang) = (Rc::clone(&query), Rc::clone(&query_lang));
        move |row: &Language| {
            let query = query.borrow();
            if query.is_empty() {
                return true;
            }
            let shown_in = query_lang.borrow().clone().unwrap_or_else(|| row.clone());
            let name_as_shown = display_name(row, &shown_in);
            matches(&query, &[&row.locale, &row.native_name, &name_as_shown])
        }
    }));
    lang.set_languages(ModelRc::from(Rc::clone(&model)));
    lang.on_filter_languages({
        let ui = ui.clone();
        move |q| {
            *query.borrow_mut() = fold_search(&q);
            *query_lang.borrow_mut() =
                ui.upgrade().map(|ui| ui.global::<crate::ui::Lang<'_>>().get_current_language());
            model.reset();
        }
    });
}

fn sorted_keymaps() -> Vec<crate::ui::Keymap> {
    use crate::ui::{Keymap, KeymapVariant};
    use slint::{ModelRc, ToSharedString, VecModel};
    libtaidan::i18n::LAYOUTS
        .entries()
        .map(|(id, layout)| Keymap {
            id: id.to_shared_string(),
            name: layout.name.to_shared_string(),
            variants: ModelRc::new(
                layout
                    .variants
                    .entries()
                    .map(|(id, name)| KeymapVariant {
                        id: id.to_shared_string(),
                        name: name.to_shared_string(),
                    })
                    .sorted_by(|a, b| a.name.cmp(&b.name))
                    .collect::<VecModel<_>>(),
            ),
        })
        .sorted_by_key(|km| (km.id != "us", km.name.clone()))
        .collect_vec()
}

fn init_keymaps(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::{Keymap, KeymapVariant};
    use slint::{ComponentHandle, FilterModel, Model, ModelRc, VecModel};
    let keymaps = sorted_keymaps();
    let system = system_keymap();
    let initial = system
        .as_ref()
        .and_then(|(layout, _)| keymaps.iter().find(|km| km.id == layout.as_str()))
        .or_else(|| keymaps.first())
        .expect("no keymaps")
        .clone();
    let initial_variant = system
        .and_then(|(layout, variant)| (layout == initial.id.as_str()).then_some(variant)?)
        .and_then(|variant| initial.variants.iter().find(|v| v.id == variant.as_str()))
        .unwrap_or_default();

    let keymap_query = Query::default();
    let keymap_model = Rc::new(FilterModel::new(VecModel::from(keymaps), {
        let keymap_query = Rc::clone(&keymap_query);
        move |km: &Keymap| matches(&keymap_query.borrow(), &[&km.id, &km.name])
    }));
    lang.set_keymaps(ModelRc::from(Rc::clone(&keymap_model)));
    lang.on_filter_keymaps(move |q| {
        *keymap_query.borrow_mut() = fold_search(&q);
        keymap_model.reset();
    });

    let variant_query = Query::default();
    let variant_filter = {
        let variant_query = Rc::clone(&variant_query);
        move |v: &KeymapVariant| matches(&variant_query.borrow(), &[&v.id, &v.name])
    };
    lang.set_variants(initial.variants.clone());
    lang.on_filter_variants({
        let variant_query = Rc::clone(&variant_query);
        let ui = ui.clone();
        move |q| {
            *variant_query.borrow_mut() = fold_search(&q);
            if let Some(ui) = ui.upgrade() {
                let lang = ui.global::<crate::ui::Lang<'_>>();
                lang.set_variants(ModelRc::new(FilterModel::new(
                    lang.get_current_keymap().variants,
                    variant_filter.clone(),
                )));
            }
        }
    });

    let initial_variant_id =
        (!initial_variant.id.is_empty()).then_some(initial_variant.id.as_str());
    if crate::install::is_dev_session() {
        save_keymap_without_applying(&initial.id, initial_variant_id);
    } else {
        // KWin doesn't follow the system's layout
        set_keymap(&initial.id, initial_variant_id, false);
    }
    lang.set_latin_fallback(has_latin_fallback(&initial.id, initial_variant_id));
    lang.set_current_keymap(initial);
    lang.set_current_variant(initial_variant);
    lang.on_select_keymap({
        let ui = ui.clone();
        move |km| {
            let Some(ui) = ui.upgrade() else { return };
            let lang = ui.global::<crate::ui::Lang<'_>>();
            if km.id == lang.get_current_keymap().id {
                return;
            }
            variant_query.borrow_mut().clear();
            lang.set_variants(km.variants.clone());
            lang.set_current_variant(KeymapVariant::default());
            set_keymap(&km.id, None, true);
            lang.set_latin_fallback(has_latin_fallback(&km.id, None));
            lang.set_current_keymap(km);
        }
    });
    lang.on_select_variant({
        let ui = ui.clone();
        move |variant| {
            let Some(ui) = ui.upgrade() else { return };
            let lang = ui.global::<crate::ui::Lang<'_>>();
            let layout = lang.get_current_keymap().id;
            let id = (!variant.id.is_empty()).then_some(variant.id.as_str());
            set_keymap(&layout, id, true);
            lang.set_latin_fallback(has_latin_fallback(&layout, id));
            lang.set_current_variant(variant);
        }
    });
}

fn display_name(
    lang: &crate::ui::Language,
    current_language: &crate::ui::Language,
) -> slint::SharedString {
    use slint::ToSharedString;
    if lang.locale == "en-owo" {
        return "OWO".into();
    }
    let Ok(locale) = icu::locale::Locale::try_from_str(&lang.locale)
        .inspect_err(|e| tracing::error!(?lang, ?e, "cannot parse locale"))
    else {
        return lang.native_name.clone();
    };
    display_hdl(current_language, |hdl| hdl.of(&locale).to_shared_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ignores_accents() {
        let found = |query: &str, text: &str| matches(&fold_search(query), &[text]);
        assert!(found("sao paulo", "São Paulo"));
        assert!(found("SÃO", "sao paulo"));
        assert!(found("francais", "Français"));
        assert!(found("mexico", "México"));
        assert!(found("cestina", "Čeština"));
        assert!(found("ελληνικα", "Ελληνικά"));
        assert!(found("istanbul", "İstanbul"));
    }

    #[test]
    fn search_leaves_cjk_and_hangul_alone() {
        let found = |query: &str, text: &str| matches(&fold_search(query), &[text]);
        assert!(found("日本", "日本語"));
        assert!(found("が", "がっこう"));
        assert!(!found("か", "が"));
        assert!(found("한국", "한국어"));
        assert!(!found("하", "한"));
        assert_eq!(fold_search("東京"), "東京");
    }

    #[test]
    fn languages_without_icu_names_are_listed() {
        let has = |tag: &str| LANGS.iter().any(|row| row.bcp47() == tag);
        let build_machine_has_ka_ge = std::process::Command::new("localedef")
            .arg("--list-archive")
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l == "ka_GE.utf8"));
        if build_machine_has_ka_ge {
            assert!(has("ka-GE"));
        }
        assert!(LANGS.len() > 200, "{}", LANGS.len());
    }

    #[test]
    fn translations_load_from_disk() {
        let assets = FileSystemAssets::try_new("po/").unwrap();
        let langs = po_langs(&assets);
        let fr: LanguageIdentifier = "fr".parse().unwrap();
        assert!(langs.contains(&fr), "{langs:?}");
        assert_eq!(langs.len(), crate::AVAILABLE_LANGS.len(), "{langs:?}");
        let loader = fluent_language_loader!();
        loader.load_languages(&assets, std::slice::from_ref(&fr)).unwrap();
        assert_eq!(loader.current_language(), fr);
        let en = fluent_language_loader!();
        en.load_languages(&assets, &[en.fallback_language().clone()]).unwrap();
        assert_ne!(loader.get("page-welcome-go"), en.get("page-welcome-go"));
    }

    #[test]
    fn ui_fallbacks() {
        let langs = |tag: &str| {
            ui_langs(tag.parse().unwrap(), &crate::AVAILABLE_LANGS)
                .iter()
                .map(ToString::to_string)
                .collect_vec()
        };
        assert_eq!(langs("pt-BR"), ["en-US"]);
        assert_eq!(langs("es-MX").first().map(String::as_str), Some("es"));
        assert_eq!(langs("es-MX").last().map(String::as_str), Some("en-US"));
        assert_eq!(langs("en-Xowo"), ["en-Xowo", "en-US"]);
        assert_eq!(langs("zh-TW").first().map(String::as_str), Some("zh-Hant"));
        let yue = langs("yue-HK");
        assert_eq!(yue.last().map(String::as_str), Some("en-US"));
        assert!(yue.contains(&"zh-Hant".to_owned()), "{yue:?}");
        for tag in ["pt-BR", "es-MX", "zh-TW", "yue-HK", "de-DE"] {
            assert!(langs(tag).iter().all(|l| {
                l == "en-US" || l.split('-').next() == tag.split('-').next() || l == "zh-Hant"
            }));
        }
    }

    #[test]
    fn rtl_languages() {
        for lang in ["ar", "ar-EG", "fa-IR", "he-IL", "ur-PK", "ps", "yi", "ckb-IQ", "ug-CN", "dv"]
        {
            assert!(is_rtl(lang), "{lang} is right to left");
        }
        for lang in ["en-US", "de-DE", "ja-JP", "zh-TW", "en-owo", "uz-UZ"] {
            assert!(!is_rtl(lang), "{lang} is left to right");
        }
    }

    #[test]
    fn posix_locales() {
        assert_eq!(posix_locale("pt-BR").as_deref(), Some("pt_BR.UTF-8"));
        assert_eq!(posix_locale("eo").as_deref(), Some("eo.UTF-8"));
        assert_eq!(posix_locale("en-Xowo").as_deref(), Some("en_US.UTF-8"));
    }
}
