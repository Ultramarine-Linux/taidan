#![warn(rust_2018_idioms)]
pub mod a11y;
pub mod account;
pub mod avatar;
pub mod cfg;
pub mod install;
pub mod l10n;
pub mod macros;
pub mod network;
pub mod prelude;
pub mod screen;
pub mod timezone;
pub mod tweaks;
pub mod ui;

use std::sync::LazyLock;

use crate::prelude::*;
use i18n_embed::LanguageLoader;
use parking_lot::RwLock;
use sentry::integrations::tracing::{BreadcrumbLayer, ErrorLayer, LogLayer, SpanLayer};
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
    let mut cfg = cfg::Config::new().expect("cannot init config");
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
    #[cfg(not(debug_assertions))]
    let sentry_guard = sentry::init((
        SENTRY_LINK,
        sentry::ClientOptions::new()
            .maybe_release(sentry::release_name!())
            // Capture user IPs and potentially sensitive headers when using HTTP server integrations
            // see https://docs.sentry.io/platforms/rust/data-management/data-collected for more info
            .send_default_pii(true),
    ));
    #[cfg(debug_assertions)]
    let sentry_guard = ();
    color_eyre::install().expect("install color_eyre");
    let file_appender = tracing_appender::rolling::never(&*TEMP_DIR, "taidan.log");
    let (non_blocking, tracing_guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(SpanLayer::new().with_filter(LevelFilter::INFO))
        .with(BreadcrumbLayer::new().with_filter(filter_fn(|metadata| {
            matches!(*metadata.level(), Level::WARN | Level::INFO)
        })))
        .with(ErrorLayer::new().with_filter(LevelFilter::ERROR))
        .with(LogLayer::new().with_filter(LevelFilter::INFO))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .with_env_var("TAIDAN_LOG")
                .from_env_lossy(),
        )
        .with(tracing_journald::layer().unwrap().with_syslog_identifier("taidan".to_owned()))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false).compact())
        .init();

    if cfg!(debug_assertions) {
        tracing::info!("Running in debug mode");
    }
    tracing::info!("Taidan {version}", version = env!("CARGO_PKG_VERSION"));
    tracing::info!("Logging to journald");
    tracing::info!("Logging to {}/taidan.log", TEMP_DIR.display());
    (sentry_guard, tracing_guard)
}
