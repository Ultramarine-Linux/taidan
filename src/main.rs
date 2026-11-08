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
    cfg::populate(&mut cfg);
    tracing::debug!("Populated cfg::Config (turn on `trace` to see body)");
    cfg
});
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

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
fn main() {
    let _guard = setup_logs_and_install_panic_hook();
    // returning instead of exiting drops `_guard`, which flushes the log and Sentry
    if CFG.should_exit {
        return;
    }
    std::sync::LazyLock::force(&l10n::PO_LOADER);

    ui::run();
}

/// log fields that can say who the user is, left out of what goes to Sentry
#[cfg_attr(debug_assertions, allow(dead_code))]
const PRIVATE_FIELDS: &[&str] = &[
    "user", "username", "prev", "ssid", "hostname", "name", "fullname", "path", "home", "dest",
    // command lines and errors, which often carry one of the above
    "args", "e",
];

/// what the user typed that says who they are, each with its placeholder, replaced wherever it
/// shows up in what goes to Sentry. Filled in by `install::start`
static PRIVATE_VALUES: RwLock<Vec<(regex::Regex, &'static str)>> = RwLock::new(Vec::new());

/// makes [`scrub_event`] replace these wherever they show up
pub fn set_private_values(settings: &libtaidan::settings::Settings) {
    let values = [
        (settings.fullname.as_str(), "<fullname>"),
        (settings.device_name.as_str(), "<hostname>"),
        (settings.hostname.as_str(), "<hostname>"),
        // also takes the home with it, whatever directory it's in
        (settings.username.as_str(), "<user>"),
    ];
    let values = values
        .into_iter()
        .filter_map(|(value, placeholder)| Some((value_regex(value.trim())?, placeholder)))
        .collect();
    *PRIVATE_VALUES.write() = values;
}

/// matches `value` wherever it's a word of its own, so a short one doesn't take parts of longer
/// words with it. [`None`] if it's empty
fn value_regex(value: &str) -> Option<regex::Regex> {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let first = value.chars().next()?;
    let last = value.chars().next_back()?;
    // only where the value itself starts or ends a word, `/home/alice` needs nothing before it
    let before = if word(first) { r"(^|[^\w-])" } else { "()" };
    let after = if word(last) { r"($|[^\w-])" } else { "()" };
    regex::RegexBuilder::new(&format!("{before}{}{after}", regex::escape(value)))
        .case_insensitive(true)
        .build()
        .inspect_err(|e| tracing::debug!(?e, "cannot make the regex to scrub a value"))
        .ok()
}

#[cfg_attr(debug_assertions, allow(dead_code))]
/// `text` with the [`PRIVATE_VALUES`] replaced
fn scrub_str(text: &mut String) {
    replace_values(text, &PRIVATE_VALUES.read());
}

#[cfg_attr(debug_assertions, allow(dead_code))]
/// `text` with each of `values` replaced by its placeholder
fn replace_values(text: &mut String, values: &[(regex::Regex, &str)]) {
    for (re, placeholder) in values {
        // twice, a match takes the character after it, which may be where the next one starts
        for _ in 0..2 {
            if let std::borrow::Cow::Owned(new) =
                re.replace_all(text, format!("${{1}}{placeholder}${{2}}"))
            {
                *text = new;
            }
        }
    }
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn scrub_value(value: &mut sentry::protocol::Value) {
    use sentry::protocol::Value;
    match value {
        Value::String(s) => scrub_str(s),
        Value::Array(values) => values.iter_mut().for_each(scrub_value),
        Value::Object(fields) => fields.values_mut().for_each(scrub_value),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg_attr(debug_assertions, allow(dead_code))]
/// drops the [`PRIVATE_FIELDS`] and replaces the [`PRIVATE_VALUES`] in the rest
fn scrub(fields: &mut sentry::protocol::Map<String, sentry::protocol::Value>) {
    fields.retain(|key, _| !PRIVATE_FIELDS.contains(&key.as_str()));
    fields.values_mut().for_each(scrub_value);
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn scrub_breadcrumb(crumb: &mut sentry::protocol::Breadcrumb) {
    if let Some(message) = &mut crumb.message {
        scrub_str(message);
    }
    scrub(&mut crumb.data);
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn scrub_event(event: &mut sentry::protocol::Event<'_>) {
    if let Some(message) = &mut event.message {
        scrub_str(message);
    }
    if let Some(entry) = &mut event.logentry {
        scrub_str(&mut entry.message);
        entry.params.iter_mut().for_each(scrub_value);
    }
    for exception in &mut event.exception.values {
        if let Some(value) = &mut exception.value {
            scrub_str(value);
        }
    }
    // the machine's hostname, which is the one the user picked on a retry after it was set
    if let Some(server) = &mut event.server_name {
        let mut name = server.to_string();
        scrub_str(&mut name);
        *server = name.into();
    }
    for context in event.contexts.values_mut() {
        if let sentry::protocol::Context::Other(fields) = context {
            scrub(fields);
        }
    }
    event.tags.retain(|key, _| !PRIVATE_FIELDS.contains(&key.as_str()));
    event.tags.values_mut().for_each(scrub_str);
    scrub(&mut event.extra);
    for crumb in &mut event.breadcrumbs.values {
        scrub_breadcrumb(crumb);
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
            scrub_breadcrumb(&mut crumb);
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

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::{Breadcrumb, Event, Exception};

    #[test]
    fn scrub_event_removes_the_user() {
        set_private_values(&libtaidan::settings::Settings {
            username: "alice".to_owned(),
            fullname: "Alice Smith".to_owned(),
            device_name: "Alice's Laptop".to_owned(),
            hostname: "alices-laptop".to_owned(),
            ..Default::default()
        });
        let mut event = Event {
            message: Some("user `alice` already exists".to_owned()),
            server_name: Some("alices-laptop".into()),
            exception: vec![Exception {
                ty: "Error".to_owned(),
                value: Some("program `runuser` failed: /home/alice/.config: denied".to_owned()),
                ..Default::default()
            }]
            .into(),
            breadcrumbs: vec![Breadcrumb {
                message: Some("hello Alice Smith on Alice's Laptop".to_owned()),
                data: [
                    ("args".to_owned(), serde_json::json!(["-u", "alice", "--", "true"])),
                    ("detail".to_owned(), serde_json::json!({ "out": ["ALICE was here"] })),
                ]
                .into(),
                ..Default::default()
            }]
            .into(),
            ..Default::default()
        };
        event.extra.insert("args".to_owned(), serde_json::json!(["alice"]));
        event.tags.insert("host".to_owned(), "alices-laptop".to_owned());
        scrub_event(&mut event);

        let json = serde_json::to_string(&event).expect("serialize event");
        assert!(!json.to_lowercase().contains("alice"), "{json}");
        assert_eq!(event.message.as_deref(), Some("user `<user>` already exists"));
        assert_eq!(
            event.exception.values[0].value.as_deref(),
            Some("program `runuser` failed: /home/<user>/.config: denied")
        );
        let crumb = &event.breadcrumbs.values[0];
        assert_eq!(crumb.message.as_deref(), Some("hello <fullname> on <hostname>"));
        assert!(!crumb.data.contains_key("args"));
        assert_eq!(crumb.data["detail"], serde_json::json!({ "out": ["<user> was here"] }));
        assert!(!event.extra.contains_key("args"));
        assert_eq!(event.server_name.as_deref(), Some("<hostname>"));
        assert!(matches!(event.tags.get("host").map(String::as_str), Some("<hostname>")));
    }

    #[test]
    fn value_regex_leaves_longer_words_alone() {
        let re = value_regex("al").expect("regex");
        assert!(!re.is_match("also"));
        assert!(re.is_match("user al exists"));
        assert!(re.is_match("/home/al"));
        assert!(value_regex("").is_none());
    }

    #[test]
    fn replace_values_takes_every_match() {
        // not through `set_private_values`, the other test sets those at the same time
        let values = [(value_regex("al").expect("regex"), "<user>")];
        let mut msg = "user `al` already exists: al al, /home/al".to_owned();
        replace_values(&mut msg, &values);
        assert_eq!(msg, "user `<user>` already exists: <user> <user>, /home/<user>");
        let mut msg = "cannot install alsa-lib".to_owned();
        replace_values(&mut msg, &values);
        assert_eq!(msg, "cannot install alsa-lib");
    }
}
