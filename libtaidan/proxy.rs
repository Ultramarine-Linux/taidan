//! a manual HTTP proxy, for networks that only reach the internet through one
//!
//! `pkexec` clears the environment, so anything run through it as root gets the proxy through
//! [`wrapper`] instead of inheriting `http_proxy` and friends.
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
    // without the file, `wrapper` would make every command run as root fail
    let proxy = match write_env_file(&env_file(), proxy.as_ref()) {
        Ok(()) => proxy,
        Err(e) => {
            tracing::error!(?e, "cannot save the proxy for commands run as root, going without");
            None
        }
    };
    *PROXY.write().unwrap_or_else(std::sync::PoisonError::into_inner) = proxy;
}

#[must_use]
pub fn get() -> Option<Proxy> {
    PROXY.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

impl Proxy {
    /// parses what the user typed, `host:port` gets an `http://` in front, `None` if the URL is
    /// invalid, has no host, isn't an HTTP(S) or SOCKS proxy, or either has control characters
    #[must_use]
    pub fn new(url: &str, no_proxy: &str) -> Option<Self> {
        let url = url.trim();
        // the URL parser drops tabs and newlines, but they'd stay in what's saved
        if has_control(url) {
            return None;
        }
        let url = if url.contains("://") { url.to_owned() } else { format!("http://{url}") };
        // the parser takes `http:///host` and `http:\\host` too, but the host must come right
        // after `://` where `without_credentials` looks for it
        if url.split_once("://").is_some_and(|(_, rest)| {
            rest.starts_with(['/', '\\'])
                || rest.split(['/', '?', '#']).next().is_some_and(|a| a.contains('\\'))
        }) {
            return None;
        }
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
        if has_control(&no_proxy) {
            return None;
        }
        // as typed, apart from the scheme's case, which some tools are picky about. Not as the
        // parser writes it out, which drops a default port, and a proxy without one is on 1080
        // for curl
        let url = match url.split_once("://") {
            Some((scheme, rest)) => format!("{}://{rest}", scheme.to_ascii_lowercase()),
            None => url,
        };
        Some(Self { url: url.trim_end_matches('/').to_owned(), no_proxy })
    }

    /// whether it's safe to write a line of, which settings that didn't come through [`Self::new`]
    /// may not be
    fn is_safe(&self) -> bool {
        !has_control(&self.url) && !has_control(&self.no_proxy)
    }

    /// the environment variables for the proxy, both cases since tools disagree on which one
    /// they read
    #[must_use]
    pub fn vars(&self) -> Vec<(&'static str, &str)> {
        let mut vars = vec![];
        for key in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"] {
            vars.push((key, self.url.as_str()));
        }
        if !self.no_proxy.is_empty() {
            for key in ["no_proxy", "NO_PROXY"] {
                vars.push((key, self.no_proxy.as_str()));
            }
        }
        vars
    }

    /// `KEY=value` lines of [`Self::vars`]
    #[must_use]
    pub fn env(&self) -> Vec<String> {
        self.vars().into_iter().map(|(k, v)| format!("{k}={v}")).collect()
    }

    /// the proxy without the `user:password@` in its URL, for places other users can read.
    /// [`None`] if the URL can't be parsed
    #[must_use]
    pub fn without_credentials(&self) -> Option<Self> {
        let url = reqwest::Url::parse(&self.url).ok()?;
        if url.username().is_empty() && url.password().is_none() {
            return Some(self.clone());
        }
        // cut out of the text rather than through the parser, which would drop a default port
        let (scheme, rest) = self.url.split_once("://")?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, path) = rest.split_at(end);
        let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        let public = format!("{scheme}://{host}{path}");
        // whatever the parser sees in it, it mustn't see credentials anymore
        let check = reqwest::Url::parse(&public).ok()?;
        (check.username().is_empty() && check.password().is_none())
            .then(|| Self { url: public, no_proxy: self.no_proxy.clone() })
    }

    /// the URL to show, without credentials, or with the password redacted if they can't be cut
    /// out
    #[must_use]
    pub fn public_url(&self) -> String {
        self.without_credentials().map_or_else(|| redact(&self.url), |p| p.url)
    }

    /// # Errors
    /// The URL isn't a valid proxy for reqwest.
    pub fn client(&self) -> reqwest::Result<reqwest::Client> {
        let proxy =
            reqwest::Proxy::all(&self.url)?.no_proxy(reqwest::NoProxy::from_string(&self.no_proxy));
        reqwest::Client::builder().proxy(proxy).build()
    }

    /// saves the proxy on the system, for login sessions (`/etc/environment`) and dnf, with
    /// [`Self::write_repo_overrides`] for the hosts it's not used for
    ///
    /// # Errors
    /// Cannot write the files.
    pub async fn persist(&self) -> Res<()> {
        // a newline would add lines of its own to the files
        if !self.is_safe() {
            return Err(Err::Msg("the proxy has control characters in it"));
        }
        // every user can read `/etc/environment` and gets it in their session, so the password
        // only goes into the dnf config. Apps ask for it themselves when the proxy wants one
        let public = self.without_credentials().ok_or(Err::Msg("the proxy URL is invalid"))?;
        // the dnf config first, `clear` goes by it, so it must be there if anything is. Only root
        // may read it. `/etc/environment` has to stay readable for login sessions to pick the
        // proxy up
        crate::root_with_stdin(
            "sh",
            [
                "-c",
                r#"
                    mkdir -p /etc/dnf/libdnf5.conf.d
                    umask 077
                    { printf '[main]\nproxy='; cat; } > "$1"
                    chmod 600 "$1"
                "#,
                "sh",
                DNF_DROPIN,
            ],
            format!("{}\n", self.url).as_bytes(),
        )
        .await
        .wrap_msg("cannot write the proxy to the dnf config")?;
        // dnf going through the proxy for a mirror it shouldn't is only slower, so this isn't
        // worth failing over
        if let Err(e) = self.write_repo_overrides().await {
            tracing::warn!(?e, "cannot keep dnf from using the proxy for the bypassed hosts");
        }
        // through stdin, since the arguments of commands run as root are logged
        let mut lines = public.env().join("\n");
        lines.push('\n');
        crate::root_with_stdin(
            "sh",
            [
                "-c",
                r#"
                    sed -i '/^\(https\?\|no\)_proxy=/Id' /etc/environment
                    cat >> /etc/environment
                "#,
            ],
            lines.as_bytes(),
        )
        .await
        .wrap_msg("cannot write the proxy to /etc/environment")
    }
}

impl Proxy {
    /// turns the proxy off in dnf for the repos on hosts in [`Self::no_proxy`], since libdnf5 has
    /// no `no_proxy` of its own. Removes the overrides if no repo needs one. Run again once the
    /// install has added its repos
    ///
    /// A best effort: only the hosts in a repo's `baseurl`, `metalink` and `mirrorlist` are
    /// known, not the mirrors a metalink or mirrorlist hands out at runtime, which keep going
    /// through the proxy
    ///
    /// # Errors
    /// Cannot write or remove the file.
    pub async fn write_repo_overrides(&self) -> Res<()> {
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
            if !tokio::fs::try_exists(REPO_OVERRIDE).await.unwrap_or(true) {
                return Ok(());
            }
            return crate::root("rm", ["-f", REPO_OVERRIDE])
                .await
                .wrap_msg("cannot remove the dnf proxy overrides");
        }
        // nothing secret in it, readable like the repo files themselves
        crate::root_with_stdin(
            "sh",
            [
                "-c",
                r#"
                    mkdir -p /etc/dnf/repos.override.d
                    umask 022
                    cat > "$1"
                "#,
                "sh",
                REPO_OVERRIDE,
            ],
            overrides.as_bytes(),
        )
        .await
        .wrap_msg("cannot write the dnf proxy overrides")
    }
}

/// where dnf reads repos from, its default `reposdir`, see dnf5.conf(5)
const REPO_DIRS: [&str; 3] = ["/etc/yum.repos.d", "/etc/distro.repos.d", "/usr/share/dnf5/repos.d"];

/// where [`Proxy::write_repo_overrides`] turns the proxy off for repos on bypassed hosts
const REPO_OVERRIDE: &str = "/etc/dnf/repos.override.d/50-taidan-proxy-bypass.repo";

/// the repos in a `.repo` file, each with the hosts in its `baseurl`, `metalink` and
/// `mirrorlist`. One with `$` in a host is left out, which is a variable only dnf knows
fn repo_hosts(text: &str) -> Vec<(String, Vec<String>)> {
    let mut repos: Vec<(String, Vec<String>)> = vec![];
    // whether the lines are for a URL key, which go on in indented lines
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
                // the parser takes it as part of the name
                && !host.contains('$')
            {
                hosts.push(host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase());
            }
        }
    }
    repos
}

/// a repos.override.d file that turns the proxy off for the `repos` on a host in `no_proxy`,
/// empty if there are none
///
/// An empty `proxy=` in a repo section overrides the one in `[main]`, and libdnf5 then doesn't
/// give librepo a proxy at all, see dnf5.conf(5). curl still goes by `http_proxy` and
/// `no_proxy` from the environment then, which `/etc/environment` sets together
fn repo_overrides(repos: &[(String, Vec<String>)], no_proxy: &str) -> String {
    let mut out = String::new();
    for (id, hosts) in repos {
        // a glob or `]` would match other repos or break the section
        let id_ok = !id.is_empty()
            && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
        if id_ok && hosts.iter().any(|host| bypassed(host, no_proxy)) {
            out.push_str(&format!("[{id}]\nproxy=\n\n"));
        }
    }
    out
}

/// whether `no_proxy` (comma-separated, like `localhost,.example.com,10.0.0.0/8`) says to reach
/// `host` directly, matching it like curl does: an entry matches itself and its subdomains, a
/// leading `.` or `*.` doesn't matter, `*` matches everything, and IPs match by CIDR
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
        // a port doesn't narrow it down, `::1`-like entries never match a name anyway
        let name = entry.split(':').next().unwrap_or_default();
        let name = name.trim_start_matches('*').trim_start_matches('.');
        !name.is_empty()
            && (host == name || host.strip_suffix(name).is_some_and(|rest| rest.ends_with('.')))
    })
}

/// whether `ip` is in `net`/`bits`, or is `net` without `bits`
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

/// whether `s` has a control character, like a newline, in it
fn has_control(s: &str) -> bool {
    s.chars().any(char::is_control)
}

/// where [`Proxy::persist`] saves the proxy for dnf
const DNF_DROPIN: &str = "/etc/dnf/libdnf5.conf.d/50-taidan-proxy.conf";

/// removes a proxy saved by [`Proxy::persist`] in an earlier attempt, nothing is touched if there
/// isn't one, so proxies set up by other means stay
///
/// # Errors
/// Cannot change the files.
pub async fn clear() -> Res<()> {
    let exists = |path| async move { tokio::fs::try_exists(path).await.unwrap_or(true) };
    if !exists(DNF_DROPIN).await && !exists(REPO_OVERRIDE).await {
        return Ok(());
    }
    crate::root(
        "sh",
        [
            "-c",
            r#"
                rm -f "$2"
                if [ -e "$1" ]; then
                    sed -i '/^\(https\?\|no\)_proxy=/Id' /etc/environment
                    rm -f "$1"
                fi
            "#,
            "sh",
            DNF_DROPIN,
            REPO_OVERRIDE,
        ],
    )
    .await
    .wrap_msg("cannot remove the saved proxy")
}

/// where [`set`] saves the current proxy for [`wrapper`], in Taidan's own temporary directory,
/// which only its user (and root) can read
fn env_file() -> std::path::PathBuf {
    crate::TEMP_DIR.join("proxy.env")
}

/// writes `proxy` to `path` as `export` lines for `sh`, only readable by its owner, or removes
/// the file if there's no proxy
fn write_env_file(path: &std::path::Path, proxy: Option<&Proxy>) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    let Some(proxy) = proxy else {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            res => res,
        };
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // the mode only applies to a new file
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    let mut lines = String::new();
    for (key, value) in proxy.vars() {
        lines.push_str(&format!("export {key}={}\n", sh_quote(value)));
    }
    file.write_all(lines.as_bytes())
}

/// `s` in single quotes for `sh`, where nothing in it is special
pub(crate) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// what to put between `pkexec --user root env` and the command so it runs with the current
/// proxy, empty if there's none. The proxy is read from [`env_file`] instead of being passed as
/// arguments, since the URL may carry a password, and `pkexec` logs the arguments of what it runs
/// and anyone can read them in `/proc`
#[must_use]
pub fn wrapper() -> Vec<std::ffi::OsString> {
    if get().is_none() {
        return vec![];
    }
    vec!["/bin/sh".into(), "-c".into(), r#". "$0" && exec "$@""#.into(), env_file().into()]
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
        // without reqwest's `socks` feature this would be an HTTP request instead
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
        // the parser would drop it, curl would then go to 1080
        let p = Proxy::new("HTTP://bob:pw@proxy.example.com:80", "").unwrap();
        assert_eq!(p.url, "http://bob:pw@proxy.example.com:80");
        assert_eq!(p.without_credentials().unwrap().url, "http://proxy.example.com:80");
        // the parser reads these as credentials too, which mustn't end up in /etc/environment
        assert!(Proxy::new("http:///bob:hunter2@proxy.example:3128", "").is_none());
        assert!(Proxy::new("http:\\\\bob:hunter2@proxy.example:3128", "").is_none());
        assert!(Proxy::new("http://bob:hunter2\\@proxy.example:3128", "").is_none());
    }

    #[test]
    fn env_file_round_trips_through_sh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("proxy.env");
        let p = Proxy {
            url: "http://bob:it's $HOME `x`@proxy.example.com:3128".into(),
            no_proxy: "localhost".into(),
        };
        write_env_file(&path, Some(&p)).unwrap();
        let mode = std::os::unix::fs::PermissionsExt::mode(&path.metadata().unwrap().permissions());
        assert_eq!(mode & 0o777, 0o600);
        let out = std::process::Command::new("/bin/sh")
            .args(["-c", r#". "$0" && exec "$@""#])
            .arg(&path)
            .args(["/usr/bin/printenv", "HTTPS_PROXY", "no_proxy"])
            .env_clear()
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8(out.stdout).unwrap(), format!("{}\nlocalhost\n", p.url));
        write_env_file(&path, None).unwrap();
        assert!(!path.exists());
        write_env_file(&path, None).unwrap();
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
