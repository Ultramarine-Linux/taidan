//! Runtime l10n module.
//!
//! The compile-time loader is available as [`crate::LL`].
use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::{unic_langid::LanguageIdentifier, FileSystemAssets, LanguageLoader as _};
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
            .map_or_else(
                |_| Box::new(crate::Localizations) as B,
                |a| Box::new(a) as B,
            ),
    )
});

static PO_AVAILABLE_LANGS: LazyLock<Vec<LanguageIdentifier>> = LazyLock::new(|| {
    fluent_language_loader!()
        .available_languages(&***PO_ASSETS)
        .unwrap()
});

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
