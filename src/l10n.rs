//! Runtime l10n module.
//!
//! The compile-time loader is available as [`crate::LL`].
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

impl LanguageRow {
    /// `localedef` also lists legacy glibc aliases (`bokmal`, `deutsch`, …) that aren't locales.
    /// Real ones have a territory (`en-US`) or are a bare 2–3 letter language code (`eo`).
    fn is_alias(&self) -> bool {
        !self.locale.contains('-') && self.locale.len() > 3
    }

    /// [`Self::locale`] as a BCP-47 tag (`en-US`), the proc macro already normalizes `localedef`'s
    /// POSIX names (`en_US`) but this keeps any stragglers working.
    fn bcp47(&self) -> slint::SharedString {
        self.locale.replace('_', "-").into()
    }
}

/// the system locale for a language tag from [`LANGS`], `pt-BR` → `pt_BR.UTF-8`, `None` for ones
/// that are only for the UI
fn posix_locale(lang: &str) -> Option<String> {
    let tag = lang.split_once('.').map_or(lang, |(left, _)| left);
    // en-Xowo, the OwO translation
    if tag.is_empty() || tag.split('-').any(|part| part.starts_with('X')) {
        return None;
    }
    Some(format!("{}.UTF-8", tag.replace('-', "_")))
}

/// switches the UI to `lang`, a BCP 47 tag, and keeps it for the installed system
///
/// # Panics
/// If the bundled translations can't be loaded.
pub fn set_lang(lang: &str) {
    if let Ok(locale) = lang
        .split_once('.')
        .map_or(lang, |(left, _)| left)
        .to_owned()
        .parse::<LanguageIdentifier>()
        .inspect_err(|e| tracing::error!(?e, ?lang, "Cannot apply language"))
    {
        tracing::info!(?locale, lang, "Using selected locale");
        let mut locales = crate::LOCALE_SOLVER
            .solve_locale(locale)
            .into_iter()
            .filter(|li| crate::AVAILABLE_LANGS.contains(li))
            .collect_vec();
        let loader = fluent_language_loader!();
        if locales.is_empty() {
            locales.push("en-US".parse().unwrap());
        }
        loader.load_languages(&crate::Localizations, &locales).expect("fail to load languages");
        *crate::LL.write() = loader;
        *PO_LOADER.write() = new_loader(locales);
    }
    // applied to the system by the installation's `SetLocale` step
    crate::SETTINGS.write().locale = posix_locale(lang).unwrap_or_default();
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
                .expect("cannot get hdl");
            let res = f(&fmt);
            *hdl = Some((current_lang.locale.to_string(), fmt));
            res
        }
    })
}

/// applies keymap changes on a worker thread, and only the newest request is applied, so scrolling
/// through the layout list doesn't queue up a `localectl` call per row
static KEYMAP_TX: LazyLock<std::sync::mpsc::Sender<(String, Option<String>)>> =
    LazyLock::new(|| {
        let (tx, rx) = std::sync::mpsc::channel::<(String, Option<String>)>();
        std::thread::spawn(move || {
            while let Ok(mut req) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    req = newer;
                }
                let (layout, variant) = req;
                if let Err(e) = crate::RT.block_on(libtaidan::i18n::set_keymap(
                    None,
                    &layout,
                    variant.as_deref(),
                )) {
                    tracing::error!(?e, layout, ?variant, "cannot set keymap");
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

fn set_keymap(layout: &str, variant: Option<&str>) {
    save_keymap(layout, variant);
    if let Err(e) = KEYMAP_TX.send((layout.to_owned(), variant.map(ToOwned::to_owned))) {
        tracing::error!(?e, "keymap worker is gone");
    }
}

/// the first X11 layout and its variant in `localectl status`'s output
fn parse_localectl_keymap(status: &str) -> Option<(String, Option<String>)> {
    let field = |name: &str| {
        status.lines().find_map(|line| {
            let (key, val) = line.split_once(':')?;
            (key.trim() == name).then(|| val.split(',').next().unwrap_or_default().trim())
        })
    };
    let layout = field("X11 Layout").filter(|l| !l.is_empty() && *l != "n/a")?;
    let variant = field("X11 Variant").filter(|v| !v.is_empty() && *v != "n/a");
    Some((layout.to_owned(), variant.map(ToOwned::to_owned)))
}

/// the keymap the system already has, so the keyboard page starts there instead of changing it
fn system_keymap() -> Option<(String, Option<String>)> {
    let out = std::process::Command::new("localectl")
        .arg("status")
        .env("LC_ALL", "C")
        .output()
        .inspect_err(|e| tracing::warn!(?e, "cannot run localectl"))
        .ok()?;
    if !out.status.success() {
        tracing::warn!(status = ?out.status, "localectl status failed");
        return None;
    }
    let keymap = parse_localectl_keymap(&String::from_utf8_lossy(&out.stdout));
    tracing::debug!(?keymap, "System keymap");
    keymap
}

/// a search query shared between a Slint callback and the [`slint::FilterModel`] it drives
type Query = Rc<RefCell<String>>;

fn matches(query: &str, haystacks: &[&str]) -> bool {
    query.is_empty() || haystacks.iter().any(|h| h.to_lowercase().contains(query))
}

/// # Panics
/// If `en-US` is missing from [`LANGS`].
pub fn initialize_ui(ui: &slint::Weak<crate::ui::AppWindow>, lang: &crate::ui::Lang<'_>) {
    use crate::ui::{FluentArg, Language};
    use slint::{ComponentHandle, Model, ToSharedString};
    lang.on__t(|id, args, _current_language| {
        tracing::trace!(?id);
        let mut fargs = fluent::FluentArgs::new();
        args.iter().for_each(|FluentArg { key, value }| {
            // for some reason k, v are owned values
            // weird design of `Model::iter()`
            fargs.set(key.to_string(), value.to_string());
        });
        let s = if PO_LOADER.read().has(&id) {
            PO_LOADER.read().get_args_fluent(&id, Some(&fargs))
        } else {
            crate::LL.read().get_args_fluent(&id, Some(&fargs))
        };
        // translations still carry the GTK-era `<tt>` tags, but Slint text has no markup
        s.replace("<tt>", "").replace("</tt>", "").into()
    });
    // Start in the system language if we have it, otherwise English.
    let initial = poly_l10n::system_want_langids()
        .find_map(|li| {
            let tag = li.to_string();
            LANGS.iter().find(|row| row.bcp47() == tag)
        })
        .or_else(|| LANGS.iter().find(|lang| lang.locale == "en-US"))
        .expect("no english");
    set_lang(&initial.bcp47());
    lang.set_current_language(Language {
        locale: initial.bcp47(),
        native_name: initial.native_name.to_shared_string(),
    });
    lang.on__name(|lang, current_language| display_name(&lang, &current_language));
    lang.on_set_lang({
        let ui = ui.clone();
        move |lang| {
            if lang.locale == "en-owo" {
                set_lang("en-Xowo");
            } else {
                set_lang(&lang.locale);
            }
            ui.upgrade_in_event_loop(|ui| ui.window().request_redraw()).expect("event loop err");
        }
    });
    init_language_list(ui, lang);
    init_keymaps(ui, lang);
}

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
    let model = Rc::new(FilterModel::new(all_langs, {
        let (query, ui) = (Rc::clone(&query), ui.clone());
        move |row: &Language| {
            let query = query.borrow();
            if query.is_empty() {
                return true;
            }
            // also match the name as displayed, i.e. in the current UI language
            let current = ui.upgrade().map_or_else(
                || row.clone(),
                |ui| ui.global::<crate::ui::Lang<'_>>().get_current_language(),
            );
            matches(&query, &[&row.locale, &row.native_name, &display_name(row, &current)])
        }
    }));
    lang.set_languages(ModelRc::from(Rc::clone(&model)));
    lang.on_filter_languages(move |q| {
        *query.borrow_mut() = q.to_lowercase();
        model.reset();
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
        *keymap_query.borrow_mut() = q.to_lowercase();
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
            *variant_query.borrow_mut() = q.to_lowercase();
            if let Some(ui) = ui.upgrade() {
                let lang = ui.global::<crate::ui::Lang<'_>>();
                lang.set_variants(ModelRc::new(FilterModel::new(
                    lang.get_current_keymap().variants,
                    variant_filter.clone(),
                )));
            }
        }
    });

    // nothing is applied until a layout is picked, the system already has this one
    save_keymap(
        &initial.id,
        (!initial_variant.id.is_empty()).then_some(initial_variant.id.as_str()),
    );
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
            set_keymap(&km.id, None);
            lang.set_current_keymap(km);
        }
    });
    lang.on_select_variant({
        let ui = ui.clone();
        move |variant| {
            let Some(ui) = ui.upgrade() else { return };
            let lang = ui.global::<crate::ui::Lang<'_>>();
            let layout = lang.get_current_keymap().id;
            set_keymap(&layout, (!variant.id.is_empty()).then_some(variant.id.as_str()));
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
    fn localectl_keymap() {
        let status = "   System Locale: LANG=en_US.UTF-8\n       VC Keymap: de\n      X11 Layout: de,us\n       X11 Model: pc105\n     X11 Variant: nodeadkeys,\n";
        assert_eq!(
            parse_localectl_keymap(status),
            Some(("de".to_owned(), Some("nodeadkeys".to_owned())))
        );
        assert_eq!(parse_localectl_keymap("      X11 Layout: us\n"), Some(("us".to_owned(), None)));
        assert_eq!(parse_localectl_keymap("       VC Keymap: n/a\n      X11 Layout: n/a\n"), None);
    }

    #[test]
    fn posix_locales() {
        assert_eq!(posix_locale("pt-BR").as_deref(), Some("pt_BR.UTF-8"));
        assert_eq!(posix_locale("eo").as_deref(), Some("eo.UTF-8"));
        assert_eq!(posix_locale("en-Xowo"), None);
    }
}
