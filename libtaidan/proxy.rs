//! a manual HTTP proxy, for networks that only reach the internet through one
//!
//! `pkexec` clears the environment, so anything run through it gets the proxy from [`env`]
//! explicitly instead of inheriting `http_proxy` and friends.
use crate::prelude::*;

#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Proxy {
    /// e.g. `http://proxy.example.com:3128`, may carry `user:password@`
    pub url: String,
    /// comma-separated hosts and domains to reach directly, e.g. `localhost,.example.com`
    pub no_proxy: String,
}

/// without the password, since the settings with the proxy in them are logged
impl std::fmt::Debug for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proxy")
            .field("url", &redact(&self.url))
            .field("no_proxy", &self.no_proxy)
            .finish()
    }
}

/// `url` with its password, if any, replaced
fn redact(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) if parsed.password().is_some() => {
            // only fails for URLs that can't have a password, and this one has one
            _ = parsed.set_password(Some("redacted"));
            parsed.to_string()
        }
        Ok(_) => url.to_owned(),
        // can't tell where a password would be
        Err(_) => "<invalid>".to_owned(),
    }
}

/// the proxy for the installation, set by [`crate::start_install`] from the settings
static PROXY: std::sync::RwLock<Option<Proxy>> = std::sync::RwLock::new(None);

pub fn set(proxy: Option<Proxy>) {
    *PROXY.write().unwrap_or_else(std::sync::PoisonError::into_inner) = proxy;
}

#[must_use]
pub fn get() -> Option<Proxy> {
    PROXY.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

impl Proxy {
    /// parses what the user typed, `host:port` gets an `http://` in front, `None` if the URL is
    /// invalid, has no host, or isn't an HTTP(S) or SOCKS proxy
    #[must_use]
    pub fn new(url: &str, no_proxy: &str) -> Option<Self> {
        let url = url.trim();
        let url = if url.contains("://") { url.to_owned() } else { format!("http://{url}") };
        let parsed = reqwest::Url::parse(&url).ok()?;
        if parsed.host_str().is_none_or(str::is_empty)
            || !matches!(parsed.scheme(), "http" | "https" | "socks5" | "socks5h")
        {
            return None;
        }
        let no_proxy = no_proxy
            .split([',', ' ', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");
        Some(Self { url: url.trim_end_matches('/').to_owned(), no_proxy })
    }

    /// `KEY=value` pairs for `env`, both cases since tools disagree on which one they read
    #[must_use]
    pub fn env(&self) -> Vec<String> {
        let mut vars = vec![];
        for key in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"] {
            vars.push(format!("{key}={}", self.url));
        }
        if !self.no_proxy.is_empty() {
            for key in ["no_proxy", "NO_PROXY"] {
                vars.push(format!("{key}={}", self.no_proxy));
            }
        }
        vars
    }

    /// # Errors
    /// The URL isn't a valid proxy for reqwest.
    pub fn client(&self) -> reqwest::Result<reqwest::Client> {
        let proxy =
            reqwest::Proxy::all(&self.url)?.no_proxy(reqwest::NoProxy::from_string(&self.no_proxy));
        reqwest::Client::builder().proxy(proxy).build()
    }

    /// saves the proxy on the system, for login sessions (`/etc/environment`) and dnf
    ///
    /// # Errors
    /// Cannot write the files.
    pub async fn persist(&self) -> Res<()> {
        // values are passed as arguments, so nothing in them is interpreted by the shell
        crate::root(
            "sh",
            [
                "-c",
                r#"
                    sed -i '/^\(https\?\|no\)_proxy=/Id' /etc/environment
                    printf '%s\n' "$@" >> /etc/environment
                "#,
                "sh",
            ]
            .into_iter()
            .map(str::to_owned)
            .chain(self.env()),
        )
        .await
        .wrap_msg("cannot write the proxy to /etc/environment")?;
        // the URL may carry a password, so only root may read it. `/etc/environment` has to stay
        // readable for login sessions to pick the proxy up
        crate::root(
            "sh",
            [
                "-c",
                r#"
                    mkdir -p /etc/dnf/libdnf5.conf.d
                    umask 077
                    printf '[main]\nproxy=%s\n' "$1" > "$2"
                    chmod 600 "$2"
                "#,
                "sh",
                &self.url,
                DNF_DROPIN,
            ],
        )
        .await
        .wrap_msg("cannot write the proxy to the dnf config")
    }
}

/// where [`Proxy::persist`] saves the proxy for dnf
const DNF_DROPIN: &str = "/etc/dnf/libdnf5.conf.d/50-taidan-proxy.conf";

/// removes a proxy saved by [`Proxy::persist`] in an earlier attempt, nothing is touched if there
/// isn't one, so proxies set up by other means stay
///
/// # Errors
/// Cannot change the files.
pub async fn clear() -> Res<()> {
    if !tokio::fs::try_exists(DNF_DROPIN).await.unwrap_or(true) {
        return Ok(());
    }
    crate::root(
        "sh",
        [
            "-c",
            r#"
                sed -i '/^\(https\?\|no\)_proxy=/Id' /etc/environment
                rm -f "$1"
            "#,
            "sh",
            DNF_DROPIN,
        ],
    )
    .await
    .wrap_msg("cannot remove the saved proxy")
}

/// `KEY=value` pairs of the current proxy, for `env`, empty if there's none
#[must_use]
pub fn env() -> Vec<String> {
    get().map(|p| p.env()).unwrap_or_default()
}

/// a client that goes through the current proxy
pub(crate) fn client() -> reqwest::Client {
    get()
        .and_then(|p| p.client().inspect_err(|e| tracing::warn!(?e, "invalid proxy")).ok())
        .unwrap_or_else(|| crate::prelude::REQWEST_CLIENT.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_the_password() {
        let proxy = Proxy::new("http://bob:hunter2@proxy.example.com:3128", "").unwrap();
        let debug = format!("{proxy:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(debug.contains("bob:redacted@proxy.example.com:3128"), "{debug}");
        let proxy = Proxy::new("proxy.example.com:3128", "").unwrap();
        assert!(format!("{proxy:?}").contains("http://proxy.example.com:3128"));
    }

    #[test]
    fn parses() {
        let p =
            Proxy::new(" proxy.example.com:3128 ", "localhost, .example.com\n10.0.0.0/8").unwrap();
        assert_eq!(p.url, "http://proxy.example.com:3128");
        assert_eq!(p.no_proxy, "localhost,.example.com,10.0.0.0/8");
        assert_eq!(Proxy::new("socks5://10.0.0.1:1080", "").unwrap().url, "socks5://10.0.0.1:1080");
        assert!(Proxy::new("", "").is_none());
        assert!(Proxy::new("ftp://x:21", "").is_none());
    }

    #[test]
    fn env_skips_empty_no_proxy() {
        let p = Proxy::new("p:8080", "").unwrap();
        assert_eq!(p.env().len(), 4);
        assert!(p.env().iter().all(|v| v.ends_with("=http://p:8080")));
    }
}
