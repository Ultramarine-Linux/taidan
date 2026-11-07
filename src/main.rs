#![warn(rust_2018_idioms)]
pub mod a11y;
pub mod account;
pub mod avatar;
pub mod cfg;
pub mod ims;
pub mod install;
pub mod l10n;
pub mod macros;
pub mod network;
pub mod prelude;
pub mod screen;
pub mod theme;
pub mod timezone;
pub mod tweaks;
pub mod ui;

use std::sync::LazyLock;

use crate::prelude::*;
use i18n_embed::LanguageLoader;
use parking_lot::RwLock;
use sentry::integrations::tracing::{BreadcrumbLayer, ErrorLayer, SpanLayer};
use tracing::Level;
use tracing_subscriber::filter::{LevelFilter, filter_fn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const APPID: &str = "com.fyralabs.Taidan";
#[cfg(not(debug_assertions))]
const SENTRY_LINK: &str =
    "https://26f566a221c052a686581b9170c2c9e6@o271654.ingest.us.sentry.io/4510401485799427";
static LOCALE_SOLVER: LazyLock<poly_l10n::LocaleFallbackSolver> = LazyLock::new(Default::default);
static AVAILABLE_LANGS: LazyLock<Vec<i18n_embed::unic_langid::LanguageIdentifier>> =
    LazyLock::new(|| {
        i18n_embed::fluent::fluent_language_loader!().available_languages(&Localizations).unwrap()
    });

/// Configuration of the distro OOBE.
pub static CFG: LazyLock<cfg::Config> = LazyLock::new(|| {
    tracing::debug!("Initializing cfg::Config");
    // a broken config mustn't keep the OOBE from coming up at all
    let mut cfg = cfg::Config::new().unwrap_or_else(|e| {
        tracing::error!(%e, "cannot parse /etc/taidan.toml, using the defaults");
        sentry::capture_message(
            &format!("cannot parse /etc/taidan.toml: {e}"),
            sentry::Level::Error,
        );
        cfg::Config::default()
    });
    cfg.populate();
    tracing::debug!("Populated cfg::Config (turn on `trace` to see body)");
    cfg
});
/// Language Loader
pub static LL: LazyLock<RwLock<i18n_embed::fluent::FluentLanguageLoader>> =
    LazyLock::new(|| RwLock::new(handle_l10n()));

pub static SETTINGS: LazyLock<RwLock<libtaidan::settings::Settings>> =
    LazyLock::new(Default::default);

/// runtime for `libtaidan`'s async system calls, so they never block the UI thread
pub static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("cannot create tokio runtime")
});

#[derive(rust_embed::RustEmbed)]
#[folder = "po/"]
#[exclude = "en-owo/*.ftl"]
struct Localizations;

fn handle_l10n() -> i18n_embed::fluent::FluentLanguageLoader {
    use i18n_embed::{LanguageLoader, fluent::fluent_language_loader};
    let loader = fluent_language_loader!();
    let locale_solver = poly_l10n::LocaleFallbackSolver::<poly_l10n::Rulebook>::default();
    let available_langs = loader.available_languages(&Localizations).unwrap();
    let mut langs = poly_l10n::system_want_langids()
        .flat_map(|li| locale_solver.solve_locale(li))
        .filter(|li| available_langs.contains(li))
        .collect_vec();
    if langs.is_empty() {
        langs = vec![loader.fallback_language().clone()];
    }
    loader.load_languages(&Localizations, &langs).unwrap();
    loader
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
fn main() {
    let _guard = setup_logs_and_install_panic_hook();
    std::sync::LazyLock::force(&LL);

    ui::run();
}

/// log fields that can say who the user is, left out of what goes to Sentry
#[cfg_attr(debug_assertions, allow(dead_code))]
const PRIVATE_FIELDS: &[&str] =
    &["user", "username", "prev", "ssid", "hostname", "name", "fullname", "path", "home", "dest"];

#[cfg_attr(debug_assertions, allow(dead_code))]
fn scrub<V>(fields: &mut sentry::protocol::Map<String, V>) {
    fields.retain(|key, _| !PRIVATE_FIELDS.contains(&key.as_str()));
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn scrub_event(event: &mut sentry::protocol::Event<'_>) {
    for context in event.contexts.values_mut() {
        if let sentry::protocol::Context::Other(fields) = context {
            scrub(fields);
        }
    }
    event.tags.retain(|key, _| !PRIVATE_FIELDS.contains(&key.as_str()));
    scrub(&mut event.extra);
    for crumb in &mut event.breadcrumbs.values {
        scrub(&mut crumb.data);
    }
}

pub static TEMP_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    let dir = tempfile::Builder::new()
        .prefix("taidan-logs")
        .tempdir()
        .expect("create logs tempdir")
        .keep();
    std::fs::create_dir_all(&dir).expect("create logs tempdir");
    dir
});

/// Returns a logging guard.
///
/// # Panics
/// - cannot install `color_eyre`
/// - cannot create taidan tempdir
#[allow(clippy::cognitive_complexity)]
fn setup_logs_and_install_panic_hook() -> impl std::any::Any {
    // first, it replaces the panic hook rather than chaining it, and Sentry's chains to it
    color_eyre::install().expect("install color_eyre");
    #[cfg(not(debug_assertions))]
    let sentry_guard = sentry::init((SENTRY_LINK, {
        let mut options = sentry::ClientOptions::new().maybe_release(sentry::release_name!());
        options.before_send = Some(std::sync::Arc::new(|mut event| {
            scrub_event(&mut event);
            Some(event)
        }));
        options.before_breadcrumb = Some(std::sync::Arc::new(|mut crumb| {
            scrub(&mut crumb.data);
            Some(crumb)
        }));
        options
    }));
    #[cfg(debug_assertions)]
    let sentry_guard = ();
    let journald = tracing_journald::layer();
    let journald_error = journald.as_ref().err().map(ToString::to_string);
    let journald = journald.ok();
    let file_appender = tracing_appender::rolling::never(&*TEMP_DIR, "taidan.log");
    let (non_blocking, tracing_guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(SpanLayer::new().with_filter(LevelFilter::INFO))
        .with(BreadcrumbLayer::new().with_filter(filter_fn(|metadata| {
            matches!(*metadata.level(), Level::WARN | Level::INFO)
        })))
        .with(ErrorLayer::new().with_filter(LevelFilter::ERROR))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .with_env_var("TAIDAN_LOG")
                .from_env_lossy(),
        )
        // not there in a container, or while journald restarts, which mustn't keep the OOBE down
        .with(journald.map(|l| l.with_syslog_identifier("taidan".to_owned())))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false).compact())
        .init();

    if cfg!(debug_assertions) {
        tracing::info!("Running in debug mode");
    }
    tracing::info!("Taidan {version}", version = env!("CARGO_PKG_VERSION"));
    match journald_error {
        None => tracing::info!("Logging to journald"),
        Some(e) => tracing::warn!(e, "cannot log to journald"),
    }
    tracing::info!("Logging to {}/taidan.log", TEMP_DIR.display());
    tracing::info!("Installation logs go to {}", libtaidan::TEMP_DIR.display());
    (sentry_guard, tracing_guard)
}
