const ENDPOINT: &str = "https://plausible.fyralabs.com/api/event";
const DOMAIN: &str = "taidan";
const DETECT_INTERNET_URL_PREFIX: &str = "app://internet/v1.0";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Device {
    #[default]
    Other,
    Chromebook,
    Surface,
    RaspberryPi,
}

impl Device {
    fn from_release_pkgs(pkgs: &str) -> Self {
        let has = |name: &str| pkgs.lines().any(|pkg| pkg.trim() == name);
        [
            ("ultramarine-release-chromebook", Self::Chromebook),
            ("ultramarine-release-surface", Self::Surface),
            ("ultramarine-release-raspberry_pi", Self::RaspberryPi),
        ]
        .into_iter()
        .find_map(|(pkg, device)| has(pkg).then_some(device))
        .unwrap_or_default()
    }

    async fn detect() -> Self {
        let out = tokio::process::Command::new("rpm")
            .args(["-qa", "--qf", "%{NAME}\\n", "ultramarine-release-*"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .await;
        match out {
            Ok(out) => Self::from_release_pkgs(&String::from_utf8_lossy(&out.stdout)),
            Err(e) => {
                tracing::debug!(?e, "cannot list the release packages");
                Self::default()
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub version: String,
    pub edition: String,
    pub arch: &'static str,
    pub device: Device,
}

impl Sample {
    fn new(os_release: &str, device: Device) -> Self {
        let field = |key| crate::cfg::os_release_field(os_release, key).unwrap_or_default();
        Self {
            version: field("VERSION_ID"),
            edition: field("VARIANT_ID"),
            arch: std::env::consts::ARCH,
            device,
        }
    }

    pub async fn detect() -> Self {
        Self::new(&crate::cfg::read_os_release(), Device::detect().await)
    }

    fn event(&self) -> serde_json::Value {
        let flag = |device| if self.device == device { "1" } else { "0" };
        serde_json::json!({
            "name": "pageview",
            "url": format!("{DETECT_INTERNET_URL_PREFIX}/{}", self.version),
            "domain": DOMAIN,
            "props": {
                "arch": self.arch,
                "edition": self.edition,
                "chromebook": flag(Device::Chromebook),
                "surface": flag(Device::Surface),
                "rpi": flag(Device::RaspberryPi),
            },
        })
    }
}

pub async fn send(settings: &crate::settings::Settings) {
    let body = Sample::detect().await.event().to_string();
    let client = match settings.proxy.as_ref().map(crate::proxy::Proxy::client) {
        Some(Ok(client)) => client,
        Some(Err(e)) => {
            tracing::warn!(?e, "cannot use the proxy for analytics, not sending");
            return;
        }
        None => reqwest::Client::new(),
    };
    let res = client
        .post(ENDPOINT)
        .header(reqwest::header::USER_AGENT, "Taidan")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .timeout(TIMEOUT)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);
    match res {
        Ok(_) => tracing::info!("Sent the analytics event"),
        Err(e) => tracing::warn!(?e, "cannot send the analytics event"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_pkgs() {
        let pkgs =
            "ultramarine-release-common\nultramarine-release-surface\nultramarine-release-kde\n";
        assert_eq!(Device::from_release_pkgs(pkgs), Device::Surface);
        assert_eq!(Device::from_release_pkgs(""), Device::Other);
    }

    #[test]
    fn event_matches_detect_internet() {
        let sample = Sample::new(
            "NAME=\"Ultramarine Linux\"\nVERSION_ID=43\nVARIANT_ID=kde\n",
            Device::RaspberryPi,
        );
        let ev = sample.event();
        assert_eq!(ev["url"], "app://internet/v0.2/43");
        assert_eq!(ev["domain"], "taidan");
        assert_eq!(ev["props"]["edition"], "kde");
        assert_eq!(ev["props"]["rpi"], "1");
        assert_eq!(ev["props"]["surface"], "0");
        assert_eq!(ev["props"]["arch"], std::env::consts::ARCH);
    }
}
