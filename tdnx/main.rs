// each backend defines its own `main`
#[cfg(all(feature = "desktop-install", feature = "server-oobe"))]
compile_error!("`desktop-install` and `server-oobe` can't be enabled together, pick one");

#[cfg(feature = "server-oobe")]
use std::io::{self, Read};

#[cfg(any(feature = "desktop-install", feature = "server-oobe"))]
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(feature = "desktop-install")]
use std::fs;

#[cfg(feature = "desktop-install")]
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        // stdout is for the result
        .with(fmt::layer().compact().with_writer(std::io::stderr))
        .with(EnvFilter::from_env("TDNX_LOG"))
        .init();

    let settings_file = std::env::args().nth(1).expect("usage: tdnx <settings.json>");
    let settings_content = fs::read_to_string(settings_file).expect("cannot read settings file");
    let mut settings: libtaidan::settings::Settings =
        serde_json::from_str(&settings_content).expect("cannot parse settings json");
    let mut cfg = libtaidan::cfg::Config::new().unwrap_or_else(|e| {
        eprintln!("cannot parse /etc/taidan.toml, using the defaults: {e}");
        libtaidan::cfg::Config::default()
    });
    // the edition picks the input method framework, among others. Not `populate`, which exits
    // when a user exists already
    cfg.read_os_release();
    libtaidan::start_install(&mut settings, &cfg, &|_| {}).await.expect("taidan install failed");
}

#[cfg(feature = "server-oobe")]
fn main() {
    tracing_subscriber::registry()
        // stdout is for the result
        .with(fmt::layer().compact().with_writer(std::io::stderr))
        .with(EnvFilter::from_env("TDNX_LOG"))
        .init();

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        fail(&format!("cannot read the setup operation: {e}"));
    }
    let operation: libtaidan::server_setup::SetupOperation = match serde_json::from_str(&input) {
        Ok(operation) => operation,
        Err(e) => fail(&format!("invalid setup operation JSON: {e}")),
    };

    match libtaidan::server_setup::validate_operation(&operation) {
        Ok(()) => println!("{{\"ok\":true}}"),
        Err(error) => fail(&error.to_string()),
    }
}

/// answers with `error` and exits with 2, the same as for any operation that isn't allowed
#[cfg(feature = "server-oobe")]
fn fail(error: &str) -> ! {
    println!(
        "{{\"ok\":false,\"error\":{}}}",
        serde_json::to_string(error).unwrap_or_else(|_| "\"\"".to_owned())
    );
    std::process::exit(2);
}

#[cfg(not(any(feature = "desktop-install", feature = "server-oobe")))]
fn main() {
    eprintln!(
        "tdnx was built without a backend feature; enable `server-oobe` or `desktop-install`"
    );
    std::process::exit(2);
}
