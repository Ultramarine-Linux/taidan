#[cfg(all(feature = "desktop-install", feature = "server-oobe"))]
compile_error!("`desktop-install` and `server-oobe` can't be enabled together, pick one");

#[cfg(feature = "server-oobe")]
use std::io::{self, Read};

#[cfg(any(feature = "desktop-install", feature = "server-oobe"))]
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(feature = "desktop-install")]
use std::fs;

#[cfg(any(feature = "desktop-install", feature = "server-oobe"))]
fn init_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer().compact().with_writer(std::io::stderr))
        .with(EnvFilter::from_env("TDNX_LOG"))
        .init();
}

#[cfg(feature = "desktop-install")]
#[tokio::main]
async fn main() {
    init_logging();

    let settings_file = std::env::args().nth(1).expect("usage: tdnx <settings.json>");
    let settings_content = fs::read_to_string(settings_file).expect("cannot read settings file");
    let mut settings: libtaidan::settings::Settings =
        serde_json::from_str(&settings_content).expect("cannot parse settings json");
    let mut cfg = libtaidan::cfg::Config::new().unwrap_or_else(|e| {
        eprintln!("cannot parse /etc/taidan.toml, using the defaults: {e}");
        libtaidan::cfg::Config::default()
    });
    cfg.read_os_release();
    libtaidan::start_install(&mut settings, &cfg, &|_| {}).await.expect("taidan install failed");
}

#[cfg(feature = "server-oobe")]
fn main() {
    init_logging();

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        reject(&format!("cannot read the setup operation: {e}"));
    }
    let operation: libtaidan::server_setup::SetupOperation = match serde_json::from_str(&input) {
        Ok(operation) => operation,
        Err(e) => reject(&format!("invalid setup operation JSON: {e}")),
    };

    match libtaidan::server_setup::validate_operation(&operation) {
        Ok(()) => println!("{{\"ok\":true}}"),
        Err(error) => reject(&error.to_string()),
    }
}

#[cfg(feature = "server-oobe")]
fn reject(error: &str) -> ! {
    println!("{{\"ok\":false,\"error\":{}}}", serde_json::Value::from(error));
    std::process::exit(2);
}

#[cfg(not(any(feature = "desktop-install", feature = "server-oobe")))]
fn main() {
    eprintln!(
        "tdnx was built without a backend feature; enable `server-oobe` or `desktop-install`"
    );
    std::process::exit(2);
}
