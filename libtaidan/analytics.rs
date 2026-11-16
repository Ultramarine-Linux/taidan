//! The anonymous Plausible pageview sent when the user leaves analytics on.
//!
//! This is the same event `detect-internet` from taidan-ultramarine-configs used to send, so the
//! existing Plausible dashboards keep working.

const ENDPOINT: &str = "https://plausible.fyralabs.com/api/event";
const DOMAIN: &str = "taidan";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Debug, Default, PartialEq, Eq)]
struct Hardware {
    chromebook: bool,
    surface: bool,
    rpi: bool,
}

impl Hardware {
    fn from_release_pkgs(pkgs: &str) -> Self {
        let has = |name: &str| pkgs.lines().any(|pkg| pkg.trim() == name);
        Self {
            chromebook: has("ultramarine-release-chromebook"),
            surface: has("ultramarine-release-surface"),
            rpi: has("ultramarine-release-raspberry_pi"),
        }
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

fn event(os_release: &str, hw: &Hardware) -> serde_json::Value {
    let field = |key| crate::cfg::os_release_field(os_release, key).unwrap_or_default();
    let flag = |on: bool| if on { "1" } else { "0" };
    serde_json::json!({
        "name": "pageview",
        "url": format!("app://internet/v0.2/{}", field("VERSION_ID")),
        "domain": DOMAIN,
        "props": {
            "arch": std::env::consts::ARCH,
            "edition": field("VARIANT_ID"),
            "chromebook": flag(hw.chromebook),
            "surface": flag(hw.surface),
            "rpi": flag(hw.rpi),
        },
    })
}

async fn current_event() -> serde_json::Value {
    event(&crate::cfg::read_os_release(), &Hardware::detect().await)
}

/// The event [`send`] would send, pretty-printed for the user to look at.
pub async fn sample() -> String {
    serde_json::to_string_pretty(&current_event().await).unwrap_or_default()
}

/// Send the pageview. Failures are only logged, analytics never stops the install.
pub async fn send(settings: &crate::settings::Settings) {
    let body = current_event().await.to_string();
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
        let hw = Hardware::from_release_pkgs(
            "ultramarine-release-common\nultramarine-release-surface\nultramarine-release-kde\n",
        );
        assert_eq!(hw, Hardware { surface: true, ..Hardware::default() });
        assert_eq!(Hardware::from_release_pkgs(""), Hardware::default());
    }

    #[test]
    fn event_matches_detect_internet() {
        let hw = Hardware { rpi: true, ..Hardware::default() };
        let ev = event("NAME=\"Ultramarine Linux\"\nVERSION_ID=43\nVARIANT_ID=kde\n", &hw);
        assert_eq!(ev["url"], "app://internet/v0.2/43");
        assert_eq!(ev["domain"], "taidan");
        assert_eq!(ev["props"]["edition"], "kde");
        assert_eq!(ev["props"]["rpi"], "1");
        assert_eq!(ev["props"]["surface"], "0");
        assert_eq!(ev["props"]["arch"], std::env::consts::ARCH);
    }
}
