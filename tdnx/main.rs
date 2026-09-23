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
        .with(fmt::layer().compact())
        .with(EnvFilter::from_env("TDNX_LOG"))
        .init();

    let settings_file = std::env::args().nth(1).expect("usage: tdnx <settings.json>");
    let settings_content = fs::read_to_string(settings_file).expect("cannot read settings file");
    let mut settings: libtaidan::settings::Settings =
        serde_json::from_str(&settings_content).expect("cannot parse settings json");
    let cfg = libtaidan::cfg::Config::new().expect("cannot read system taidan config");
    libtaidan::start_install(&mut settings, &cfg, &|_| {})
        .await
        .expect("taidan install failed");
}

#[cfg(feature = "server-oobe")]
fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().compact())
        .with(EnvFilter::from_env("TDNX_LOG"))
        .init();

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read setup operation");
    let operation: libtaidan::server_setup::SetupOperation =
        serde_json::from_str(&input).expect("invalid setup operation JSON");

    match libtaidan::server_setup::validate_operation(&operation) {
        Ok(()) => println!("{{\"ok\":true}}"),
        Err(error) => {
            println!(
                "{{\"ok\":false,\"error\":{}}}",
                serde_json::to_string(&error.to_string()).expect("serialize error")
            );
            std::process::exit(2);
        }
    }
}

#[cfg(not(any(feature = "desktop-install", feature = "server-oobe")))]
fn main() {
    eprintln!("tdnx was built without a backend feature; enable `server-oobe` or `desktop-install`");
    std::process::exit(2);
}
