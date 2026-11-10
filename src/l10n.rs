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

/// the embedded languages that [`PO_ASSETS`] also has. `available_languages` can't list a
/// [`FileSystemAssets`]. it gets file names without their language directory, so it only finds
/// the fallback
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

/// creates a loader for `langs`, leaving out ones [`PO_ASSETS`] doesn't have. if the translations
/// there can't be loaded, the embedded ones are used instead
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
    // a fresh loader, since the failed one may be half loaded
    let loader = fluent_language_loader!();
    let mut embedded =
        langs.into_iter().filter(|li| crate::AVAILABLE_LANGS.contains(li)).collect_vec();
    if embedded.is_empty() {
        embedded = vec![fallback];
    }
    if let Err(e) = loader.load_languages(&crate::Localizations, &embedded) {
        tracing::error!(?e, ?embedded, "cannot load the embedded translations either");
    }
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
    pub en_name: &'static str,
}

taidan_proc_macros::comptime_localedef_langrows!(LANGS);

impl LanguageRow {
    /// `localedef` also lists legacy glibc aliases (`bokmal`, `deutsch`, …) that aren't locales.
    /// real ones have a territory (`en-US`) or are a bare 2–3 letter language code (`eo`).
    fn is_alias(&self) -> bool {
        !self.locale.contains('-') && self.locale.len() > 3
    }

    /// [`Self::locale`] as a BCP-47 tag (`en-US`). the proc macro already normalizes `localedef`'s
    /// POSIX names (`en_US`), this catches any it missed
    fn bcp47(&self) -> slint::SharedString {
        self.locale.replace('_', "-").into()
    }
}

/// the system locale for a language tag from [`LANGS`], `pt-BR` → `pt_BR.UTF-8`. [`None`] for
/// empty and private-use tags
fn posix_locale(lang: &str) -> Option<String> {
    let tag = lang.split_once('.').map_or(lang, |(left, _)| left);
    // the OwO translation is only for the UI, the system gets US English
    if tag == "en-Xowo" {
        return Some("en_US.UTF-8".to_owned());
    }
    if tag.is_empty() || tag.split('-').any(|part| part.starts_with('X')) {
        return None;
    }
    Some(format!("{}.UTF-8", tag.replace('-', "_")))
}

/// translations in another language close enough to show before English, by base language
const EXTRA_FALLBACKS: &[(&str, &str)] = &[("yue", "zh-Hant")];

/// the translations to show for `locale`, best first, out of `available`. only ones in the same
/// language and those in [`EXTRA_FALLBACKS`], then `en-US`, never an unrelated language the
/// solver offers (`pt-BR` → `es`)
fn ui_langs(
    locale: LanguageIdentifier,
    available: &[LanguageIdentifier],
) -> Vec<LanguageIdentifier> {
    let extra = EXTRA_FALLBACKS
        .iter()
        .filter(|(lang, _)| locale.language.as_str() == *lang)
        .filter_map(|(_, fallback)| fallback.parse::<LanguageIdentifier>().ok())
        .collect_vec();
    let base = locale.language;
    // the solver doesn't return the tag itself, so an exact match (e.g. en-Xowo) goes first
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

/// switches the UI to `lang`, a BCP 47 tag, and keeps it for the installed system. returns the
/// tag of the translation it got, which may be a fallback
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
    let posix = posix_locale(lang).unwrap_or_default();
    if !posix.is_empty() {
        crate::a11y::language_changed(&posix);
    }
    // applied to the system by the installation's `SetLocale` step
    crate::SETTINGS.write().locale = posix;
    shown
}

/// whether `lang`, a BCP 47 tag, is written right to left, from the likely script of the language
/// (`ar` → Arabic, `uz-Arab`, …)
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

            // ICU's built-in data lacks names in many languages, so those get English ones
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

/// a keymap for the worker below, and whether it goes to the system too, not only the session
type KeymapReq = (String, Option<String>, bool);

/// the window the keymap worker notifies when the system keymap couldn't be saved
static KEYMAP_UI: std::sync::OnceLock<slint::Weak<crate::ui::AppWindow>> =
    std::sync::OnceLock::new();

/// applies keymap changes on a worker thread. only the newest request is applied, so scrolling
/// through the layout list doesn't queue up a `localectl` call per row
static KEYMAP_TX: LazyLock<std::sync::mpsc::Sender<KeymapReq>> = LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::channel::<KeymapReq>();
    std::thread::spawn(move || {
        while let Ok(mut req) = rx.recv() {
            while let Ok(newer) = rx.try_recv() {
                // a picked one stays picked for the system, even if the newest is only the session
                let system = req.2 || newer.2;
                req = newer;
                req.2 = system;
            }
            let (layout, variant, system) = req;
            let saved = match crate::RT.block_on(libtaidan::i18n::set_keymap(
                None,
                &layout,
                variant.as_deref(),
                system,
            )) {
                Ok(saved) => saved,
                Err(e) => {
                    tracing::error!(?e, layout, ?variant, "cannot set keymap");
                    // the system keymap may have been saved, there's no telling
                    true
                }
            };
            // only a system request says whether the system keymap was saved
            if system && let Some(ui) = KEYMAP_UI.get() {
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

/// keeps the keymap for the installed system, without applying it
fn save_keymap(layout: &str, variant: Option<&str>) {
    let mut settings = crate::SETTINGS.write();
    layout.clone_into(&mut settings.kb_layout);
    settings.kb_variant = variant.map(ToOwned::to_owned);
}

/// whether US English goes along with `layout`, see [`libtaidan::i18n::XkbKeymap::new`]
fn has_latin_fallback(layout: &str, variant: Option<&str>) -> bool {
    libtaidan::i18n::XkbKeymap::new(layout, variant).has_latin_fallback()
}

/// keeps the keymap and applies it to the session, and to the system too if `system`, which is
/// what picking one on the page does
fn set_keymap(layout: &str, variant: Option<&str>, system: bool) {
    save_keymap(layout, variant);
    if system {
        crate::SETTINGS.write().kb_picked = true;
    }
    if let Err(e) = KEYMAP_TX.send((layout.to_owned(), variant.map(ToOwned::to_owned), system)) {
        tracing::error!(?e, "keymap worker is gone");
    }
}

/// the first of a comma-separated list of layouts or variants, `None` if it's empty
fn first_of(list: &str) -> Option<String> {
    let first = list.split(',').next().unwrap_or_default().trim();
    (!first.is_empty()).then(|| first.to_owned())
}

/// the first layout and variant of the X11 keymap in `/etc/vconsole.conf`, see
/// [`libtaidan::i18n::XkbKeymap::parse_vconsole`]
fn parse_vconsole_keymap(conf: &str) -> Option<(Option<String>, Option<String>)> {
    libtaidan::i18n::XkbKeymap::parse_vconsole(conf)
        .map(|km| (first_of(&km.layouts), first_of(&km.variants)))
}

/// the first layout and variant of the keymap in `/etc/X11/xorg.conf.d/00-keyboard.conf`, see
/// [`libtaidan::i18n::XkbKeymap::parse_xorg`]
fn parse_xorg_keymap(conf: &str) -> (Option<String>, Option<String>) {
    let km = libtaidan::i18n::XkbKeymap::parse_xorg(conf);
    (first_of(&km.layouts), first_of(&km.variants))
}

/// the keymap the system already has, so the keyboard page starts there instead of changing it.
/// read from the files `systemd-localed` keeps it in, `vconsole.conf` first as localed does, so
/// there's no blocking `localectl` call before the UI is up
fn system_keymap() -> Option<(String, Option<String>)> {
    let read = |path: &str| {
        std::fs::read_to_string(path)
            .inspect_err(|e| tracing::debug!(?e, path, "cannot read the keymap"))
            .ok()
    };
    let (layout, variant) =
        read("/etc/vconsole.conf").as_deref().and_then(parse_vconsole_keymap).or_else(|| {
            read("/etc/X11/xorg.conf.d/00-keyboard.conf").as_deref().map(parse_xorg_keymap)
        })?;
    let keymap = layout.map(|layout| (layout, variant));
    tracing::debug!(?keymap, "System keymap");
    keymap
}

/// a search query shared between a Slint callback and the [`slint::FilterModel`] it drives
type Query = Rc<RefCell<String>>;

/// `text` lowercased and without accents, so a search for `sao paulo` finds `São Paulo`
///
/// only the combining diacritics of Latin, Greek and Cyrillic are dropped, and the composed form
/// comes back, so e.g. kana with dakuten and Hangul are left as they are
#[must_use]
pub fn fold_search(text: &str) -> String {
    use icu::normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};
    let lower = text.to_lowercase();
    let stripped = DecomposingNormalizerBorrowed::new_nfd()
        .normalize(&lower)
        .chars()
        .filter(|c| !matches!(c, '\u{300}'..='\u{36f}'))
        .collect::<String>();
    ComposingNormalizerBorrowed::new_nfc().normalize(&stripped).into_owned()
}

/// whether any of `haystacks` has `query`, which is already [`fold_search`]ed
fn matches(query: &str, haystacks: &[&str]) -> bool {
    query.is_empty() || haystacks.iter().any(|h| fold_search(h).contains(query))
}

/// # Panics
/// if `en-US` is missing from [`LANGS`]
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
        // the source string dropped the GTK-era `<tt>` tags, but translations keep them until
        // they're redone in Weblate, and Slint text has no markup
        s.replace("<tt>", "").replace("</tt>", "").into()
    });
    // the system language if it's listed, otherwise English
    let initial = poly_l10n::system_want_langids()
        .find_map(|li| {
            let tag = li.to_string();
            LANGS.iter().find(|row| row.bcp47() == tag)
        })
        .or_else(|| LANGS.iter().find(|lang| lang.locale == "en-US"))
        .expect("no english");
    // by the translation shown, since a language without one would be English laid out right to
    // left
    lang.set_rtl(is_rtl(&set_lang(&initial.bcp47())));
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
                if lang.locale == "en-owo" { set_lang("en-Xowo") } else { set_lang(&lang.locale) };
            // before `current_language` changes, like the translations, see `Lang.select`
            if let Some(ui) = ui.upgrade() {
                ui.global::<crate::ui::Lang<'_>>().set_rtl(is_rtl(&loaded));
                crate::ims::follow_locale(&ui);
            }
            ui.upgrade_in_event_loop(|ui| {
                // runs after `current_language` changed. the time zone names are localized in
                // Rust, so redo them even when the time zone page is skipped. ones never shown
                // yet are filled when their page or the summary is entered.
                let tz = ui.global::<crate::ui::Tz<'_>>();
                if !tz.get_current().id.is_empty() {
                    tz.invoke_refresh();
                }
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

/// fills [`crate::ui::Lang::set_languages`]
fn init_language_list(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::Language;
    use slint::{ComponentHandle, FilterModel, ModelRc, ToSharedString, VecModel};
    let all_langs = LANGS
        .iter()
        .filter(|row| row.locale == "en-owo" || !row.is_alias())
        .map(|row| Language {
            locale: row.bcp47(),
            native_name: row.native_name.to_shared_string(),
        })
        .collect::<VecModel<_>>();
    let query = Query::default();
    // the UI language when the search was typed. the arrow keys switch the language while the
    // search stays, and the matches mustn't change under them
    let query_lang = Rc::new(RefCell::new(None::<Language>));
    let model = Rc::new(FilterModel::new(all_langs, {
        let (query, query_lang) = (Rc::clone(&query), Rc::clone(&query_lang));
        move |row: &Language| {
            let query = query.borrow();
            if query.is_empty() {
                return true;
            }
            // also match the name as displayed when the search was typed
            let shown = query_lang.borrow().clone().unwrap_or_else(|| row.clone());
            matches(&query, &[&row.locale, &row.native_name, &display_name(row, &shown)])
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

fn init_keymaps(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::{Keymap, KeymapVariant};
    use slint::{ComponentHandle, FilterModel, Model, ModelRc, ToSharedString, VecModel};
    let keymaps = libtaidan::i18n::LAYOUTS
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
        // `us` first, as it's the most likely pick, then the rest alphabetically
        .sorted_by_key(|km| (km.id != "us", km.name.clone()))
        .collect_vec();
    // start at the system's keymap, `us` (sorted first) if it's unknown
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

    // applied to the session, since KWin doesn't follow the system's layout and would type as US
    // English, e.g. a password typed here wouldn't work at the login screen. not to the system,
    // which already has it more completely than what's parsed here (all its layouts and options).
    // not in a dev session, where it would change the developer's keyboard
    let initial_variant_id =
        (!initial_variant.id.is_empty()).then_some(initial_variant.id.as_str());
    if crate::install::is_dev_session() {
        save_keymap(&initial.id, initial_variant_id);
    } else {
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
            // the variant search is cleared in Slint before this is called
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
        // CJK and Hangul are left alone
        assert!(found("日本", "日本語"));
        assert!(found("が", "がっこう"));
        assert!(!found("か", "が"));
        assert!(found("한국", "한국어"));
        assert!(!found("하", "한"));
        assert_eq!(fold_search("東京"), "東京");
    }

    #[test]
    fn languages_without_icu_names_are_listed() {
        // only where the build machine has the locale, since `localedef --list-archive` is the
        // source
        let has = |tag: &str| LANGS.iter().any(|row| row.bcp47() == tag);
        if std::process::Command::new("localedef")
            .arg("--list-archive")
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l == "ka_GE.utf8"))
        {
            assert!(has("ka-GE"));
        }
        assert!(LANGS.len() > 200, "{}", LANGS.len());
    }

    #[test]
    fn vconsole_keymap() {
        let conf = "# Written by systemd-localed(8)\nKEYMAP=de\nFONT=eurlatgr\nXKBLAYOUT=\"de,us\"\nXKBMODEL=pc105\nXKBVARIANT=nodeadkeys,\n";
        assert_eq!(
            parse_vconsole_keymap(conf),
            Some((Some("de".to_owned()), Some("nodeadkeys".to_owned())))
        );
        assert_eq!(parse_vconsole_keymap("XKBLAYOUT=us\n"), Some((Some("us".to_owned()), None)));
        // localed then goes on to the xorg config
        assert_eq!(parse_vconsole_keymap("KEYMAP=us\n#XKBLAYOUT=de\n"), None);
        assert_eq!(parse_vconsole_keymap("XKBLAYOUT=\n"), None);
        assert_eq!(parse_vconsole_keymap("XKBOPTIONS=ctrl:nocaps\n"), Some((None, None)));
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
        // there's no Portuguese translation, and Spanish doesn't stand in for one
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
    fn xorg_keymap() {
        let conf = "# Written by systemd-localed(8)\nSection \"InputClass\"\n        Identifier \"system-keyboard\"\n        MatchIsKeyboard \"on\"\n        Option \"XkbLayout\" \"de,us\"\n        Option \"XkbModel\" \"pc105\"\n        Option \"XkbVariant\" \"nodeadkeys,\"\nEndSection\n";
        assert_eq!(parse_xorg_keymap(conf), (Some("de".to_owned()), Some("nodeadkeys".to_owned())));
        assert_eq!(parse_xorg_keymap("Option \"XkbLayout\" \"de\"\n"), (None, None));
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
