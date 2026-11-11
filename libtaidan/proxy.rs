use std::path::Path;

use crate::prelude::*;

#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Proxy {
    pub url: String,
    pub no_proxy: String,
}

impl std::fmt::Debug for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proxy")
            .field("url", &redact(&self.url))
            .field("no_proxy", &self.no_proxy)
            .finish()
    }
}

fn redact(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) if parsed.password().is_some() => {
            _ = parsed.set_password(Some("redacted"));
            parsed.to_string()
        }
        Ok(_) => url.to_owned(),
        Err(_) => "<invalid>".to_owned(),
    }
}

static PROXY: std::sync::RwLock<Option<Proxy>> = std::sync::RwLock::new(None);

pub fn set(proxy: Option<Proxy>) {
    *PROXY.write().unwrap_or_else(std::sync::PoisonError::into_inner) = proxy;
}

#[must_use]
pub fn get() -> Option<Proxy> {
    PROXY.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

impl Proxy {
    #[must_use]
    pub fn new(url: &str, no_proxy: &str) -> Option<Self> {
        let url = url.trim();
        if has_control(url) {
            return None;
        }
        let (scheme, rest) = url.split_once("://").unwrap_or(("http", url));
        if !starts_with_authority(rest) {
            return None;
        }
        let url = format!("{}://{rest}", scheme.to_ascii_lowercase());
        let parsed = reqwest::Url::parse(&url).ok()?;
        if parsed.host_str().is_none_or(str::is_empty)
            || !matches!(parsed.scheme(), "http" | "https" | "socks5" | "socks5h")
        {
            return None;
        }
        let no_proxy =
            no_proxy.split([',', ' ', '\n']).map(str::trim).filter(|s| !s.is_empty()).join(",");
        if has_control(&no_proxy) {
            return None;
        }
        Some(Self { url: url.trim_end_matches('/').to_owned(), no_proxy })
    }

    fn is_safe(&self) -> bool {
        !has_control(&self.url) && !has_control(&self.no_proxy)
    }

    #[must_use]
    pub fn vars(&self) -> Vec<(&'static str, &str)> {
        let mut vars = Vec::from(
            ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"].map(|key| (key, &*self.url)),
        );
        if !self.no_proxy.is_empty() {
            vars.extend(["no_proxy", "NO_PROXY"].map(|key| (key, &*self.no_proxy)));
        }
        vars
    }

    fn systemd_dropin(&self) -> String {
        let mut dropin = "[Service]\n".to_owned();
        for (key, value) in self.vars() {
            let value = escape_unit_value(value);
            dropin.push_str(&format!("Environment=\"{key}={value}\"\n"));
        }
        dropin
    }

    fn env(&self) -> Vec<String> {
        self.vars().into_iter().map(|(k, v)| format!("{k}={v}")).collect()
    }

    #[must_use]
    pub fn without_credentials(&self) -> Option<Self> {
        let url = reqwest::Url::parse(&self.url).ok()?;
        if url.username().is_empty() && url.password().is_none() {
            return Some(self.clone());
        }
        let (scheme, rest) = self.url.split_once("://")?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, path) = rest.split_at(end);
        let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        let public = format!("{scheme}://{host}{path}");
        let reparsed = reqwest::Url::parse(&public).ok()?;
        (reparsed.username().is_empty() && reparsed.password().is_none())
            .then(|| Self { url: public, no_proxy: self.no_proxy.clone() })
    }

    #[must_use]
    pub fn public_url(&self) -> String {
        self.without_credentials().map_or_else(|| redact(&self.url), |p| p.url)
    }

    pub fn client(&self) -> reqwest::Result<reqwest::Client> {
        let proxy =
            reqwest::Proxy::all(&self.url)?.no_proxy(reqwest::NoProxy::from_string(&self.no_proxy));
        reqwest::Client::builder().proxy(proxy).build()
    }

    pub async fn persist(&self) -> Res<()> {
        if !self.is_safe() {
            return Err(Err::Msg("the proxy has control characters in it"));
        }
        if get().as_ref() != Some(self) {
            return Err(Err::Msg("only the installation's proxy can be saved"));
        }
        crate::root(crate::Verb::ProxyPersist).await.wrap_msg("cannot save the proxy")
    }

    pub(crate) fn persist_as_root(&self) -> Result<(), String> {
        if !self.is_safe() {
            return Err("the proxy has control characters in it".to_owned());
        }
        let world_readable = self.without_credentials().ok_or("the proxy URL is invalid")?;
        let dropin = format!("[main]\nproxy={}\n", self.url);
        crate::helper::write_atomic(Path::new(DNF_DROPIN), dropin.as_bytes(), 0o600)
            .map_err(|e| format!("cannot write the proxy to the dnf config: {e}"))?;
        if let Err(e) = self.write_repo_overrides_as_root() {
            eprintln!("cannot keep dnf from using the proxy for the bypassed hosts: {e}");
        }
        let mut env =
            without_proxy_vars(&std::fs::read_to_string(ETC_ENVIRONMENT).unwrap_or_default());
        for line in world_readable.env() {
            env.push_str(&line);
            env.push('\n');
        }
        crate::helper::write_atomic(Path::new(ETC_ENVIRONMENT), env.as_bytes(), 0o644)
            .map_err(|e| format!("cannot write the proxy to {ETC_ENVIRONMENT}: {e}"))?;
        if Path::new("/ostree").exists() {
            crate::helper::write_atomic(
                Path::new(RPM_OSTREED_ETC),
                self.systemd_dropin().as_bytes(),
                0o600,
            )
            .map_err(|e| format!("cannot write the proxy for rpm-ostree: {e}"))?;
        }
        Ok(())
    }
}

fn starts_with_authority(after_scheme: &str) -> bool {
    !after_scheme.starts_with(['/', '\\'])
        && !after_scheme.split(['/', '?', '#']).next().is_some_and(|a| a.contains('\\'))
}

fn escape_unit_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('%', "%%")
}

const RPM_OSTREED_ETC: &str = "/etc/systemd/system/rpm-ostreed.service.d/50-taidan-proxy.conf";
const RPM_OSTREED_RUN: &str = "/run/systemd/system/rpm-ostreed.service.d/50-taidan-proxy.conf";

pub(crate) fn rpm_ostreed_as_root(proxy: Option<&Proxy>) -> Result<(), String> {
    let path = Path::new(RPM_OSTREED_RUN);
    let old = std::fs::read_to_string(path).ok();
    let new = match proxy {
        Some(proxy) if !proxy.is_safe() => {
            return Err("the proxy has control characters in it".to_owned());
        }
        Some(proxy) => Some(proxy.systemd_dropin()),
        None => None,
    };
    if old == new {
        return Ok(());
    }
    match &new {
        Some(dropin) => crate::helper::write_atomic(path, dropin.as_bytes(), 0o600),
        None => std::fs::remove_file(path),
    }
    .map_err(|e| format!("cannot write the proxy for rpm-ostree: {e}"))?;
    let systemctl = |args: &[&str]| {
        std::process::Command::new("systemctl")
            .args(args)
            .status()
            .map_err(|e| format!("cannot run systemctl: {e}"))
            .and_then(|s| s.success().then_some(()).ok_or(format!("systemctl {args:?}: {s}")))
    };
    systemctl(&["daemon-reload"])?;
    systemctl(&["try-restart", "rpm-ostreed.service"])
}

const ETC_ENVIRONMENT: &str = "/etc/environment";

fn without_proxy_vars(env: &str) -> String {
    env.lines()
        .filter(|line| {
            let key = line.split_once('=').map_or("", |(k, _)| k).to_ascii_lowercase();
            !matches!(key.as_str(), "http_proxy" | "https_proxy" | "no_proxy")
        })
        .map(|line| format!("{line}\n"))
        .collect()
}

impl Proxy {
    /// libdnf5 has no `no_proxy` of its own
    pub async fn write_repo_overrides(&self) -> Res<()> {
        if get().as_ref() != Some(self) {
            return Err(Err::Msg("only the installation's proxy has overrides written"));
        }
        crate::root(crate::Verb::ProxyRepoOverrides)
            .await
            .wrap_msg("cannot write the dnf proxy overrides")
    }

    pub(crate) fn write_repo_overrides_as_root(&self) -> std::io::Result<()> {
        let repos = REPO_DIRS
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "repo"))
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .flat_map(|text| repo_hosts(&text))
            .collect::<Vec<_>>();
        let overrides = repo_overrides(&repos, &self.no_proxy);
        if overrides.is_empty() {
            return remove(REPO_OVERRIDE);
        }
        crate::helper::write_atomic(Path::new(REPO_OVERRIDE), overrides.as_bytes(), 0o644)
    }
}

const REPO_DIRS: [&str; 3] = ["/etc/yum.repos.d", "/etc/distro.repos.d", "/usr/share/dnf5/repos.d"];

const REPO_OVERRIDE: &str = "/etc/dnf/repos.override.d/50-taidan-proxy-bypass.repo";

fn repo_hosts(text: &str) -> Vec<(String, Vec<String>)> {
    let mut repos: Vec<(String, Vec<String>)> = vec![];
    let mut in_urls = false;
    for line in text.lines() {
        let continued = line.starts_with([' ', '\t']);
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        let values = if continued {
            if !in_urls {
                continue;
            }
            line
        } else if let Some(id) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            repos.push((id.trim().to_owned(), vec![]));
            in_urls = false;
            continue;
        } else {
            let Some((key, value)) = line.split_once('=') else { continue };
            in_urls = matches!(key.trim(), "baseurl" | "metalink" | "mirrorlist");
            if !in_urls {
                continue;
            }
            value
        };
        let Some((_, hosts)) = repos.last_mut() else { continue };
        for url in values.split([',', ' ', '\t']).filter(|u| !u.is_empty()) {
            if let Ok(url) = reqwest::Url::parse(url)
                && let Some(host) = url.host_str()
                && !host.is_empty()
                && !host.contains('$')
            {
                hosts.push(host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase());
            }
        }
    }
    repos
}

fn repo_overrides(repos: &[(String, Vec<String>)], no_proxy: &str) -> String {
    let mut out = String::new();
    for (id, hosts) in repos {
        if is_literal_repo_id(id) && hosts.iter().any(|host| bypassed(host, no_proxy)) {
            out.push_str(&format!("[{id}]\nproxy=\n\n"));
        }
    }
    out
}

fn is_literal_repo_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

fn bypassed(host: &str, no_proxy: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
    let host_ip = host.parse::<std::net::IpAddr>().ok();
    no_proxy.split(',').map(str::trim).filter(|e| !e.is_empty()).any(|entry| {
        if entry == "*" {
            return true;
        }
        let entry = entry.to_ascii_lowercase();
        if let Some(host_ip) = host_ip {
            let (net, bits) = entry.split_once('/').unwrap_or((&entry, ""));
            let net = net.trim_start_matches('[').trim_end_matches(']');
            let Ok(net) = net.parse::<std::net::IpAddr>() else { return false };
            return in_cidr(host_ip, net, bits.parse().ok());
        }
        let name = entry.split(':').next().unwrap_or_default();
        let name = name.trim_start_matches('*').trim_start_matches('.');
        !name.is_empty()
            && (host == name || host.strip_suffix(name).is_some_and(|rest| rest.ends_with('.')))
    })
}

fn in_cidr(ip: std::net::IpAddr, net: std::net::IpAddr, bits: Option<u32>) -> bool {
    use std::net::IpAddr;
    let (ip, net, len) = match (ip, net) {
        (IpAddr::V4(ip), IpAddr::V4(net)) => {
            (u128::from(ip.to_bits()), u128::from(net.to_bits()), 32)
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) => (ip.to_bits(), net.to_bits(), 128),
        _ => return false,
    };
    let bits = bits.unwrap_or(len).min(len);
    if bits == 0 {
        return true;
    }
    let shift = len - bits;
    ip >> shift == net >> shift
}

fn has_control(s: &str) -> bool {
    s.chars().any(char::is_control)
}

const DNF_DROPIN: &str = "/etc/dnf/libdnf5.conf.d/50-taidan-proxy.conf";

pub async fn clear() -> Res<()> {
    let exists = |path| async move { tokio::fs::try_exists(path).await.unwrap_or(true) };
    if !exists(DNF_DROPIN).await && !exists(REPO_OVERRIDE).await {
        return Ok(());
    }
    crate::root(crate::Verb::ProxyClear).await.wrap_msg("cannot remove the saved proxy")
}

fn remove(path: &str) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        res => res,
    }
}

pub(crate) fn clear_as_root() -> std::io::Result<()> {
    remove(REPO_OVERRIDE)?;
    remove(RPM_OSTREED_ETC)?;
    if Path::new(DNF_DROPIN).exists() {
        let env = std::fs::read_to_string(ETC_ENVIRONMENT).unwrap_or_default();
        let cleaned = without_proxy_vars(&env);
        if cleaned != env {
            crate::helper::write_atomic(Path::new(ETC_ENVIRONMENT), cleaned.as_bytes(), 0o644)?;
        }
        remove(DNF_DROPIN)?;
    }
    Ok(())
}

pub(crate) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systemd_dropin_escapes() {
        let proxy = Proxy::new("http://bob:p%40ss@proxy:3128", "localhost").unwrap();
        let dropin = proxy.systemd_dropin();
        assert!(dropin.starts_with("[Service]\n"), "{dropin}");
        assert!(dropin.contains("Environment=\"https_proxy=http://bob:p%%40ss@proxy:3128\"\n"));
        assert!(dropin.contains("Environment=\"no_proxy=localhost\"\n"), "{dropin}");
    }

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

    #[tokio::test]
    async fn socks_proxy_speaks_socks() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = Proxy::new(&format!("socks5h://{addr}"), "").unwrap().client().unwrap();
        let req = tokio::spawn(async move { client.get("http://example.com").send().await });
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut greeting = [0; 1];
        sock.read_exact(&mut greeting).await.unwrap();
        assert_eq!(greeting[0], 5, "not a SOCKS5 greeting");
        drop(sock);
        assert!(req.await.unwrap().is_err());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(Proxy::new("http://proxy.example.com:3128\nevil=1", "").is_none());
        assert!(Proxy::new("http://proxy.exa\tmple.com:3128", "").is_none());
        assert!(Proxy::new("http://proxy.example.com:3128\r", "localhost").is_some());
        assert!(Proxy::new("p:8080", "localhost\r,example.com").is_some());
        assert!(Proxy::new("p:8080", "local\rhost").is_none());
        assert!(Proxy::new("p:8080", "local\u{1b}host").is_none());
        let p = Proxy { url: "http://p:8080\nevil=1".into(), no_proxy: String::new() };
        assert!(!p.is_safe());
        let p = Proxy { url: "http://p:8080".into(), no_proxy: "a\nb".into() };
        assert!(!p.is_safe());
        assert!(Proxy::new("p:8080", "a\nb").unwrap().is_safe());
    }

    #[test]
    fn strips_credentials() {
        let p = Proxy::new("http://bob:hunter2@proxy.example.com:3128", "localhost").unwrap();
        let public = p.without_credentials().unwrap();
        assert_eq!(public.url, "http://proxy.example.com:3128");
        assert_eq!(public.no_proxy, "localhost");
        let p = Proxy::new("http://bob@proxy.example.com:3128", "").unwrap();
        assert_eq!(p.without_credentials().unwrap().url, "http://proxy.example.com:3128");
        let p = Proxy::new("proxy.example.com:3128", "").unwrap();
        assert_eq!(p.without_credentials().unwrap(), p);
        // the URL parser drops a default port, and curl then assumes 1080 for a proxy
        let p = Proxy::new("HTTP://bob:pw@proxy.example.com:80", "").unwrap();
        assert_eq!(p.url, "http://bob:pw@proxy.example.com:80");
        assert_eq!(p.without_credentials().unwrap().url, "http://proxy.example.com:80");
        assert!(Proxy::new("http:///bob:hunter2@proxy.example:3128", "").is_none());
        assert!(Proxy::new("http:\\\\bob:hunter2@proxy.example:3128", "").is_none());
        assert!(Proxy::new("http://bob:hunter2\\@proxy.example:3128", "").is_none());
    }

    #[test]
    fn drops_proxy_vars_from_etc_environment() {
        let env = "LANG=C\nhttp_proxy=x\nHTTPS_PROXY=y\nNo_Proxy=z\nhttp_proxy_user=keep\n# c\n";
        assert_eq!(without_proxy_vars(env), "LANG=C\nhttp_proxy_user=keep\n# c\n");
        assert_eq!(without_proxy_vars("A=1"), "A=1\n");
        assert_eq!(without_proxy_vars(""), "");
    }

    #[test]
    fn finds_repo_hosts() {
        let text = "\
[fedora]
name=Fedora $releasever
#baseurl=http://commented.example/
metalink=https://mirrors.fedoraproject.org/metalink?repo=fedora-$releasever

[corp]
baseurl=https://Mirror.Corp/fedora/$releasever/
  http://backup.corp/fedora/, file:///srv/repo
gpgcheck=1
  https://not-a-url-key.example/

[vars]
baseurl=https://$mirrorhost/fedora/
";
        let repos = repo_hosts(text);
        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0], ("fedora".to_owned(), vec!["mirrors.fedoraproject.org".to_owned()]));
        assert_eq!(repos[1].0, "corp");
        assert_eq!(repos[1].1, ["mirror.corp", "backup.corp"]);
        assert!(repos[2].1.is_empty());
    }

    #[test]
    fn overrides_bypassed_repos() {
        let repos = repo_hosts(
            "[corp]\nbaseurl=https://mirror.corp/fedora/\n\
             [fedora]\nmetalink=https://mirrors.fedoraproject.org/metalink\n\
             [bad*id]\nbaseurl=https://mirror.corp/\n",
        );
        let overrides = repo_overrides(&repos, "localhost,mirror.corp");
        assert_eq!(overrides, "[corp]\nproxy=\n\n");
        assert_eq!(repo_overrides(&repos, "localhost,127.0.0.1"), "");
        assert_eq!(repo_overrides(&repos, ""), "");
    }

    #[test]
    fn matches_no_proxy_like_curl() {
        assert!(bypassed("mirror.corp", "mirror.corp"));
        assert!(bypassed("eu.mirror.corp", "mirror.corp"));
        assert!(bypassed("eu.mirror.corp", ".mirror.corp"));
        assert!(bypassed("mirror.corp", "*.mirror.corp"));
        assert!(bypassed("MIRROR.corp", "localhost, Mirror.Corp:443"));
        assert!(!bypassed("evilmirror.corp", "mirror.corp"));
        assert!(!bypassed("mirror.corp.evil", "mirror.corp"));
        assert!(bypassed("anything.example", "*"));
        assert!(bypassed("10.1.2.3", "10.0.0.0/8"));
        assert!(!bypassed("11.1.2.3", "10.0.0.0/8"));
        assert!(bypassed("127.0.0.1", "127.0.0.1"));
        assert!(bypassed("[::1]", "::1"));
        assert!(bypassed("fd00::5", "fd00::/8"));
        assert!(!bypassed("10.1.2.3", "fd00::/8"));
        assert!(!bypassed("mirror.corp", "::1,10.0.0.0/8"));
    }

    #[test]
    fn public_url_hides_credentials() {
        let p = Proxy::new("http://bob:hunter2@proxy.example.com:3128", "").unwrap();
        assert_eq!(p.public_url(), "http://proxy.example.com:3128");
    }

    #[test]
    fn env_skips_empty_no_proxy() {
        let p = Proxy::new("p:8080", "").unwrap();
        assert_eq!(p.env().len(), 4);
        assert!(p.env().iter().all(|v| v.ends_with("=http://p:8080")));
    }
}
