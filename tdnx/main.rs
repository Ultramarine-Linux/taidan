use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

    libtaidan::start_install(&mut settings, &cfg, &|_| {}).await.expect("taidan install failed");
}
