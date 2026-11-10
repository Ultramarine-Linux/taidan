//! `taidan-helper`, the one program the OOBE may run as root
//!
//! While the OOBE runs, its polkit rule lets the taidan user `pkexec` [`PATH`] and nothing else.
//! The helper takes one [`Verb`] per run, checks its arguments, and does that one thing, so
//! something else running as taidan (a browser Orca's help opened, say) can't just `pkexec sh`.
//! It refuses to run at all once the setup is done (see [`Verb::after_setup`]), as a second line
//! behind the rule being gone by then.
//!
//! `pkexec` clears the environment and logs the arguments, so the request is passed on stdin
//! instead: a line of JSON ([`Request`], with the proxy, which may carry a password), then
//! whatever the verb reads, like a password or a file. The verb's name is also the only
//! argument, for the logs. With nothing on stdin, a verb without fields can be run by its name
//! alone, which is how `start-taidan` uses it.
//!
//! Some verbs are still powerful, since the OOBE's job is to make an admin and install software.
//! They're narrowed down as far as that allows, see each one's docs. A few (custom repos and
//! scripts, packages outside the ones Taidan installs itself) are only taken from a caller that's
//! root already, like `tdnx` run as root, for whom they give nothing away.
use std::{
    ffi::OsString,
    io::{BufRead as _, Read as _, Write as _},
    os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// where the helper is installed, the polkit rule allows exactly this path
pub const PATH: &str = "/usr/libexec/taidan-helper";

/// `PATH` for what the helper runs, `pkexec`'s for root without root's home
const SAFE_PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// the helper's own state, on a tmpfs, so it's gone after a reboot
const RUN_DIR: &str = "/run/taidan-helper";

/// the user [`Verb::UserAdd`] or [`Verb::HomedCreate`] made during this boot, which
/// [`Verb::AsUser`] may run things as even after [`crate::passwd::CREATED_RECORD`] is cleared
const CREATED_USER: &str = "/run/taidan-helper/created-user";

/// where 802.1X certificates are copied for `NetworkManager`, one folder per network, then one per
/// login attempt
pub const CERT_DIR: &str = "/etc/pki/taidan/8021x";

/// where tweaks are. Always the installed path, never the relative one debug builds list tweaks
/// from, which would resolve against whatever directory root is in
const TWEAKS_DIR: &str = "/usr/share/taidan/tweaks";

/// the polkit rule `start-taidan` drops when it gives up
const POLKIT_RULE: &str = "/run/polkit-1/rules.d/100-taidan.rules";

/// the captive portal browser's unit and user, see `src/network.rs`
const PORTAL_UNIT: &str = "taidan-portal.service";
const PORTAL_USER: &str = "taidan-portal";
const PORTAL_DISPLAY: &str = "wayland-0";
const PORTAL_BROWSER: &str = "/usr/bin/cog";

/// the most the request line may be
const MAX_REQUEST: u64 = 1 << 20;
/// the most a verb's input may be, enough for a large avatar
const MAX_INPUT: u64 = 32 << 20;

/// what the helper does, one per run
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verb", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Verb {
    /// removes `/var/.unconfigured` and `/.unconfigured` and disables the reconfiguration unit
    MarkConfigured,
    Reboot,
    PowerOff,
    /// a root terminal on the session's display, only with `taidan.debug_shell` on the kernel
    /// command line, which the helper checks itself
    DebugShell {
        wayland_display: String,
    },
    /// the captive portal browser as `taidan-portal` in a unit of its own, given the session's
    /// display socket (which must be in the caller's runtime dir), the language and the page
    StartPortal {
        socket: String,
        lang: Option<String>,
        uri: String,
    },
    StopPortal,
    /// asks the root processes in the caller's cgroup, what an earlier taidan left running, to
    /// stop
    KillLeftovers,
    /// [`Self::KillLeftovers`], with `SIGKILL`
    KillLeftoversNow,
    /// removes the polkit rule, so nothing running as taidan can use it anymore
    DropPolkitRule,
    /// writes [`crate::passwd::CREATED_RECORD`], only for a user that doesn't exist yet, so it
    /// can't name someone else's account for the verbs below
    RecordCreated {
        user: String,
        keep_home: bool,
    },
    ClearRecord,
    /// removes the recorded user, if it's removable and nobody has logged in as it
    RemoveUser {
        user: String,
        keep_home: bool,
    },
    /// `useradd` for the recorded user, in `wheel`, with the crypt hash on stdin, locked. `uid`
    /// only if it owns the home that's already there
    UserAdd {
        user: String,
        fullname: String,
        uid: Option<u32>,
    },
    /// `homectl create` for the recorded user, with the identity (password, hash, `locked`) on
    /// stdin. `skel` must be a directory the caller owns
    HomedCreate {
        user: String,
        fullname: String,
        skel: Option<PathBuf>,
    },
    /// the image on stdin as the created user's icon in `AccountsService`
    SetAvatar {
        user: String,
    },
    UnlockUser {
        user: String,
    },
    /// `homectl update --locked=no`, with the password on stdin
    HomedUnlock {
        user: String,
    },
    AddToVideo {
        user: String,
    },
    /// `argv` as the created user, with the rest of stdin as its stdin. Only for the user Taidan
    /// made, never root, taidan or someone who was already there
    AsUser {
        user: String,
        argv: Vec<String>,
    },
    /// [`Self::AsUser`] inside `homectl with`, with the user's password as the first line of stdin
    AsUserHomed {
        user: String,
        argv: Vec<String>,
    },
    /// a tweak's `up`, found by its id in `/usr/share/taidan/tweaks`, with the settings on stdin
    RunTweak {
        id: String,
        on: bool,
    },
    /// writes [`crate::tweaks::ON_RECORD`]
    RecordTweaks {
        ids: Vec<String>,
    },
    ClearTweakRecord,
    /// saves the request's proxy for dnf and login sessions
    ProxyPersist,
    /// turns the proxy off in dnf for the repos on the request's proxy's bypassed hosts
    ProxyRepoOverrides,
    /// removes a proxy saved by [`Self::ProxyPersist`]
    ProxyClear,
    /// unmasks GeoClue if `on`, masks it otherwise
    Geoclue {
        on: bool,
    },
    EnableNtp,
    SetTimezone {
        timezone: String,
    },
    /// the static hostname, or the pretty one
    SetHostname {
        hostname: String,
        pretty: bool,
    },
    SetLocale {
        locale: String,
    },
    SetX11Keymap {
        layouts: String,
        variants: String,
        options: String,
    },
    /// which of `pkgs` the repos have, printed one per line
    RepoQuery {
        pkgs: Vec<String>,
    },
    /// a `.repo` URL or the id of a disabled repo, root callers only
    EnableRepo {
        repo: String,
    },
    /// root callers only
    CoprEnable {
        copr: String,
    },
    /// installs (or only downloads) the extras: codecs, input methods, language packs. Others only
    /// for root callers
    DnfExtras {
        pkgs: Vec<String>,
        download_only: bool,
    },
    /// swaps `ffmpeg-free` for RPM Fusion's `ffmpeg`
    DnfSwapFfmpeg,
    /// installs driver packages with dnf, only the ones Taidan knows
    DnfDrivers {
        pkgs: Vec<String>,
    },
    /// layers driver packages with rpm-ostree, only the ones Taidan knows
    RpmOstreeInstall {
        pkgs: Vec<String>,
    },
    /// installs (or only downloads) flatpaks from the configured remotes
    FlatpakInstall {
        refs: Vec<String>,
        download_only: bool,
    },
    /// a custom script from the settings, root callers only
    RunScript {
        script: String,
    },
    /// downloads the updates as an offline transaction
    DnfUpgradeOffline,
    DnfOfflineClean,
    /// arms the offline transaction for the next restart, without restarting
    DnfOfflineArm,
    /// the kernel arguments the NVIDIA driver needs on rpm-ostree
    NvidiaKargs,
    /// the Xorg snippet that makes NVIDIA the primary GPU
    NvidiaPrimaryGpu,
    /// prints the kernel that boots next, from grubby
    DefaultKernel,
    Akmods {
        kernel: String,
    },
    /// the file on stdin as `CERT_DIR/network/attempt/file`, only readable by root
    InstallCert {
        network: String,
        attempt: String,
        file: String,
    },
    /// removes a network's certificates but the attempt in `keep`
    PruneCerts {
        network: String,
        keep: Option<String>,
    },
    RemoveCertAttempt {
        network: String,
        attempt: String,
    },
}

/// what the helper reads from the first line of stdin
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    verb: Verb,
    /// for what the verb downloads, and [`Verb::ProxyPersist`]
    #[serde(default)]
    proxy: Option<crate::proxy::Proxy>,
}

impl Verb {
    /// the verb's name, as it's written in the request and the helper's argument
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::MarkConfigured => "mark-configured",
            Self::Reboot => "reboot",
            Self::PowerOff => "power-off",
            Self::DebugShell { .. } => "debug-shell",
            Self::StartPortal { .. } => "start-portal",
            Self::StopPortal => "stop-portal",
            Self::KillLeftovers => "kill-leftovers",
            Self::KillLeftoversNow => "kill-leftovers-now",
            Self::DropPolkitRule => "drop-polkit-rule",
            Self::RecordCreated { .. } => "record-created",
            Self::ClearRecord => "clear-record",
            Self::RemoveUser { .. } => "remove-user",
            Self::UserAdd { .. } => "user-add",
            Self::HomedCreate { .. } => "homed-create",
            Self::SetAvatar { .. } => "set-avatar",
            Self::UnlockUser { .. } => "unlock-user",
            Self::HomedUnlock { .. } => "homed-unlock",
            Self::AddToVideo { .. } => "add-to-video",
            Self::AsUser { .. } => "as-user",
            Self::AsUserHomed { .. } => "as-user-homed",
            Self::RunTweak { .. } => "run-tweak",
            Self::RecordTweaks { .. } => "record-tweaks",
            Self::ClearTweakRecord => "clear-tweak-record",
            Self::ProxyPersist => "proxy-persist",
            Self::ProxyRepoOverrides => "proxy-repo-overrides",
            Self::ProxyClear => "proxy-clear",
            Self::Geoclue { .. } => "geoclue",
            Self::EnableNtp => "enable-ntp",
            Self::SetTimezone { .. } => "set-timezone",
            Self::SetHostname { .. } => "set-hostname",
            Self::SetLocale { .. } => "set-locale",
            Self::SetX11Keymap { .. } => "set-x11-keymap",
            Self::RepoQuery { .. } => "repo-query",
            Self::EnableRepo { .. } => "enable-repo",
            Self::CoprEnable { .. } => "copr-enable",
            Self::DnfExtras { .. } => "dnf-extras",
            Self::DnfSwapFfmpeg => "dnf-swap-ffmpeg",
            Self::DnfDrivers { .. } => "dnf-drivers",
            Self::RpmOstreeInstall { .. } => "rpm-ostree-install",
            Self::FlatpakInstall { .. } => "flatpak-install",
            Self::RunScript { .. } => "run-script",
            Self::DnfUpgradeOffline => "dnf-upgrade-offline",
            Self::DnfOfflineClean => "dnf-offline-clean",
            Self::DnfOfflineArm => "dnf-offline-arm",
            Self::NvidiaKargs => "nvidia-kargs",
            Self::NvidiaPrimaryGpu => "nvidia-primary-gpu",
            Self::DefaultKernel => "default-kernel",
            Self::Akmods { .. } => "akmods",
            Self::InstallCert { .. } => "install-cert",
            Self::PruneCerts { .. } => "prune-certs",
            Self::RemoveCertAttempt { .. } => "remove-cert-attempt",
        }
    }

    /// whether the verb still runs once the setup is done (the markers are gone), for what
    /// `install::finish` and `start-taidan` do after `mark_configured`. Everything else is refused
    /// then
    #[must_use]
    pub const fn after_setup(&self) -> bool {
        matches!(
            self,
            Self::MarkConfigured
                | Self::Reboot
                | Self::PowerOff
                | Self::StopPortal
                | Self::KillLeftovers
                | Self::KillLeftoversNow
                | Self::DropPolkitRule
                | Self::ClearRecord
                | Self::ClearTweakRecord
                // the accessibility options changed on the done page, only for the user this boot
                // made, see `created_user`
                | Self::AsUser { .. }
                | Self::AsUserHomed { .. }
        )
    }

    /// whether what the verb runs gets the proxy
    const fn uses_network(&self) -> bool {
        matches!(
            self,
            Self::RunTweak { .. }
                | Self::RepoQuery { .. }
                | Self::EnableRepo { .. }
                | Self::CoprEnable { .. }
                | Self::DnfExtras { .. }
                | Self::DnfSwapFfmpeg
                | Self::DnfDrivers { .. }
                | Self::RpmOstreeInstall { .. }
                | Self::FlatpakInstall { .. }
                | Self::RunScript { .. }
                | Self::DnfUpgradeOffline
                | Self::DnfOfflineArm
        )
    }

    /// checks the arguments, without looking at the system. `root_caller` if whoever asked is
    /// root already, who may use the verbs that are root only and install any package
    ///
    /// # Errors
    /// Why the arguments aren't taken.
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self, root_caller: bool) -> Result<(), String> {
        let root_only = |what: &str| {
            if root_caller { Ok(()) } else { Err(format!("{what} is only taken from root")) }
        };
        match self {
            Self::MarkConfigured
            | Self::Reboot
            | Self::PowerOff
            | Self::StopPortal
            | Self::KillLeftovers
            | Self::KillLeftoversNow
            | Self::DropPolkitRule
            | Self::ClearRecord
            | Self::ClearTweakRecord
            | Self::ProxyPersist
            | Self::ProxyRepoOverrides
            | Self::ProxyClear
            | Self::Geoclue { .. }
            | Self::EnableNtp
            | Self::DnfSwapFfmpeg
            | Self::DnfUpgradeOffline
            | Self::DnfOfflineClean
            | Self::DnfOfflineArm
            | Self::NvidiaKargs
            | Self::NvidiaPrimaryGpu
            | Self::DefaultKernel => Ok(()),
            Self::DebugShell { wayland_display } => {
                check(word(wayland_display, "._-"), "display", wayland_display)
            }
            Self::StartPortal { socket, lang, uri } => {
                check(
                    socket.starts_with("/run/user/")
                        && !socket.contains("/../")
                        && !socket
                            .contains(|c: char| c == ':' || c.is_whitespace() || c.is_control()),
                    "socket",
                    socket,
                )?;
                if let Some(lang) = lang {
                    check(word(lang, "_.@-"), "language", lang)?;
                }
                let url = reqwest::Url::parse(uri).map_err(|e| format!("invalid uri: {e}"))?;
                check(matches!(url.scheme(), "http" | "https"), "uri", uri)
            }
            Self::RecordCreated { user, .. }
            | Self::RemoveUser { user, .. }
            | Self::SetAvatar { user }
            | Self::UnlockUser { user }
            | Self::HomedUnlock { user }
            | Self::AddToVideo { user } => check_user(user),
            Self::UserAdd { user, fullname, uid } => {
                check_user(user)?;
                check_fullname(fullname)?;
                match uid {
                    Some(uid) if !crate::passwd::REGULAR_UIDS.contains(uid) => {
                        Err(format!("uid {uid} isn't a regular one"))
                    }
                    _ => Ok(()),
                }
            }
            Self::HomedCreate { user, fullname, skel } => {
                check_user(user)?;
                check_fullname(fullname)?;
                match skel {
                    Some(skel) if !skel.is_absolute() || skel.to_str().is_none() => {
                        Err(format!("invalid skel `{}`", skel.display()))
                    }
                    _ => Ok(()),
                }
            }
            Self::AsUser { user, argv } | Self::AsUserHomed { user, argv } => {
                check_user(user)?;
                match argv.first() {
                    None => Err("nothing to run".to_owned()),
                    Some(cmd) if cmd.is_empty() || cmd.starts_with('-') => {
                        Err(format!("invalid command `{cmd}`"))
                    }
                    Some(_) if argv.iter().any(|a| a.contains('\0')) => {
                        Err("NUL in an argument".to_owned())
                    }
                    Some(_) => Ok(()),
                }
            }
            Self::RunTweak { id, .. } => check_tweak_id(id),
            Self::RecordTweaks { ids } => ids.iter().try_for_each(|id| {
                check_tweak_id(id.strip_prefix(crate::tweaks::STARTED).unwrap_or(id))
            }),
            Self::SetTimezone { timezone } => check(
                word(timezone, "/_+-")
                    && !timezone.starts_with('/')
                    && !timezone.split('/').any(|c| c.is_empty() || c.starts_with('.')),
                "timezone",
                timezone,
            ),
            Self::SetHostname { hostname, pretty: true } => check(
                !hostname.trim().is_empty()
                    && hostname.len() <= 255
                    && !hostname.contains(char::is_control),
                "pretty hostname",
                hostname,
            ),
            Self::SetHostname { hostname, pretty: false } => check(
                hostname.len() <= 64
                    && word(hostname, "-.")
                    && !hostname.starts_with(['-', '.'])
                    && !hostname.ends_with(['-', '.']),
                "hostname",
                hostname,
            ),
            Self::SetLocale { locale } => {
                check(word(locale, "_.@-") && !locale.starts_with('-'), "locale", locale)
            }
            Self::SetX11Keymap { layouts, variants, options } => {
                for value in [layouts, variants, options] {
                    check(value.is_empty() || word(value, "_,:()+-"), "keymap", value)?;
                }
                Ok(())
            }
            Self::RepoQuery { pkgs } => check_pkgs(pkgs, |_| true),
            Self::DnfExtras { pkgs, .. } => check_pkgs(pkgs, |p| root_caller || allowed_extra(p)),
            Self::DnfDrivers { pkgs } | Self::RpmOstreeInstall { pkgs } => {
                check_pkgs(pkgs, |p| root_caller || allowed_driver(p))
            }
            Self::FlatpakInstall { refs, .. } => {
                if refs.is_empty() {
                    return Err("no flatpaks".to_owned());
                }
                refs.iter().try_for_each(|r| {
                    check(word(r, "._/-") && !r.starts_with(['-', '/']), "flatpak", r)
                })
            }
            Self::EnableRepo { repo } => {
                root_only("a custom repo")?;
                check(
                    !repo.starts_with('-')
                        && !repo.starts_with("http://")
                        && !repo.contains(char::is_control),
                    "repo",
                    repo,
                )
            }
            Self::CoprEnable { copr } => {
                root_only("a copr")?;
                check(word(copr, "/._-") && !copr.starts_with('-'), "copr", copr)
            }
            Self::RunScript { .. } => root_only("a custom script"),
            Self::Akmods { kernel } => {
                check(word(kernel, "._+-") && !kernel.starts_with(['-', '.']), "kernel", kernel)
            }
            Self::InstallCert { network, attempt, file } => {
                check_cert_dir(network)?;
                check_attempt(attempt)?;
                let (kind, ext) = file.split_once('.').unwrap_or((file, ""));
                check(
                    matches!(kind, "ca" | "client" | "key" | "pac")
                        && ext.len() <= 8
                        && ext.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                        && (ext.is_empty() == !file.contains('.')),
                    "certificate file",
                    file,
                )
            }
            Self::PruneCerts { network, keep } => {
                check_cert_dir(network)?;
                keep.as_deref().map_or(Ok(()), check_attempt)
            }
            Self::RemoveCertAttempt { network, attempt } => {
                check_cert_dir(network)?;
                check_attempt(attempt)
            }
        }
    }
}

/// `Err` naming `what` and `value` unless `ok`
fn check(ok: bool, what: &str, value: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("invalid {what} `{}`", value.escape_debug())) }
}

/// whether `s` isn't empty and only has ASCII letters, digits and `extra`
fn word(s: &str, extra: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

/// a username the OOBE would take, which can't be an option or a reserved name like root
fn check_user(user: &str) -> Result<(), String> {
    match crate::passwd::username_syntax_problem(user) {
        None => Ok(()),
        Some(problem) => {
            Err(format!("username `{}` can't be used: {problem:?}", user.escape_debug()))
        }
    }
}

/// a full name that's a GECOS field already
fn check_fullname(fullname: &str) -> Result<(), String> {
    check(crate::passwd::sanitize_fullname(fullname) == fullname, "full name", fullname)
}

/// a tweak's directory name
fn check_tweak_id(id: &str) -> Result<(), String> {
    check(word(id, "._-") && !id.starts_with(['.', '-']), "tweak", id)
}

/// a network's certificate folder, see `cert_name` in `src/network.rs`
fn check_cert_dir(name: &str) -> Result<(), String> {
    check(word(name, "-") && !name.starts_with('-'), "network", name)
}

/// a login attempt's folder, a timestamp
fn check_attempt(attempt: &str) -> Result<(), String> {
    check(word(attempt, "") && attempt.chars().all(|c| c.is_ascii_digit()), "attempt", attempt)
}

/// package names or `@group`s, which mustn't look like options or globs, each `allowed`
fn check_pkgs(pkgs: &[String], allowed: impl Fn(&str) -> bool) -> Result<(), String> {
    if pkgs.is_empty() {
        return Err("no packages".to_owned());
    }
    pkgs.iter().try_for_each(|p| {
        check(word(p, "@_+.:-") && !p.starts_with(['-', '.', ':']), "package", p)?;
        if allowed(p) { Ok(()) } else { Err(format!("package `{p}` isn't one Taidan installs")) }
    })
}

/// whether `pkg` is one of the extras Taidan installs: codecs, the input method frameworks and
/// methods, and language packs
fn allowed_extra(pkg: &str) -> bool {
    use crate::i18n::I18nImf;
    let langpack = ["ultramarine-langpacks-", "langpacks-"]
        .iter()
        .find_map(|prefix| pkg.strip_prefix(prefix))
        .is_some_and(|lang| word(lang, "_@"));
    langpack
        || crate::steps::Codecs::codecs().contains(&pkg)
        || [I18nImf::Fcitx5, I18nImf::IBus]
            .iter()
            .any(|&imf| crate::steps::imf_packages(imf).contains(&pkg))
        || crate::i18n::IMS
            .values()
            .flat_map(|ims| ims.values())
            .any(|im| im.fcitx5_pkg == Some(pkg) || im.ibus_pkg == Some(pkg))
        // see `DnfDownloadApps::pre`
        || pkg == "fcitx5-chinese-addons"
}

/// whether `pkg` is one of the driver packages Taidan installs
fn allowed_driver(pkg: &str) -> bool {
    crate::steps::driver_packages().contains(&pkg)
}

/// runs the helper, returns its exit status: 0 if the verb worked, what the program it ran
/// exited with if that failed, 2 for a request it doesn't take
#[must_use]
pub fn main() -> i32 {
    match serve() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("taidan-helper: {e}");
            2
        }
    }
}

fn serve() -> Result<i32, String> {
    if uzers::get_effective_uid() != 0 {
        return Err("must be run as root, through pkexec".to_owned());
    }
    let name = std::env::args().nth(1).ok_or("usage: taidan-helper <verb> <request")?;
    let mut stdin = std::io::stdin().lock();
    let mut line = String::new();
    (&mut stdin)
        .take(MAX_REQUEST)
        .read_line(&mut line)
        .map_err(|e| format!("cannot read the request: {e}"))?;
    let request: Request = if line.trim().is_empty() {
        let verb = serde_json::from_value(serde_json::json!({ "verb": name }))
            .map_err(|e| format!("`{name}` needs a request on stdin: {e}"))?;
        Request { verb, proxy: None }
    } else {
        serde_json::from_str(&line).map_err(|e| format!("invalid request: {e}"))?
    };
    let verb = request.verb;
    if verb.name() != name {
        return Err(format!("the request is for `{}`, not `{name}`", verb.name()));
    }
    let mut input = vec![];
    // one more, to tell an input that's too long from one that's just long enough
    stdin
        .take(MAX_INPUT + 1)
        .read_to_end(&mut input)
        .map_err(|e| format!("cannot read the input: {e}"))?;
    if input.len() as u64 > MAX_INPUT {
        return Err(format!("the input is over {MAX_INPUT} bytes"));
    }
    let proxy = match request.proxy {
        Some(p)
            if crate::proxy::Proxy::new(&p.url, &p.no_proxy).is_none()
                || p.url.contains(char::is_control) =>
        {
            return Err("invalid proxy".to_owned());
        }
        p => p,
    };
    let caller = caller_uid();
    // defense in depth, the polkit rule is gone by then too
    if !crate::marked_unconfigured() && !verb.after_setup() {
        return Err(format!("the setup is done, `{name}` isn't taken anymore"));
    }
    verb.validate(caller == 0)?;
    let ctx = Ctx { caller, input, proxy, network: verb.uses_network() };
    ctx.execute(&verb)
}

/// the uid that ran `pkexec`, or the helper itself
fn caller_uid() -> u32 {
    std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|uid| uid.parse().ok())
        .unwrap_or_else(uzers::get_current_uid)
}

/// what a request runs with
struct Ctx {
    caller: u32,
    input: Vec<u8>,
    proxy: Option<crate::proxy::Proxy>,
    /// whether what the verb runs gets [`Self::proxy`], see [`Verb::uses_network`]
    network: bool,
}

/// `Err` with the context, for `?` on [`std::io::Result`]
fn io(what: &str) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |e| format!("{what}: {e}")
}

impl Ctx {
    /// a command with a clean environment: [`SAFE_PATH`], root's home, the locale `pkexec` kept,
    /// and the proxy
    fn cmd(&self, program: &str) -> Command {
        let mut cmd = Command::new(program);
        // `HOME` as `pkexec` sets it for root
        cmd.env_clear().env("PATH", SAFE_PATH).env("HOME", "/root").current_dir("/");
        for (key, value) in std::env::vars_os() {
            let k = key.as_encoded_bytes();
            if k == b"LANG" || k == b"LANGUAGE" || k.starts_with(b"LC_") {
                cmd.env(key, value);
            }
        }
        if let Some(proxy) = self.proxy.as_ref().filter(|_| self.network) {
            cmd.envs(proxy.vars());
        }
        cmd
    }

    /// runs `cmd` with `input` on stdin, or none, its output goes where the helper's does
    fn run(cmd: &mut Command, input: Option<&[u8]>) -> Result<i32, String> {
        let name = cmd.get_program().to_string_lossy().into_owned();
        cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
        let mut child = cmd.spawn().map_err(|e| format!("cannot run {name}: {e}"))?;
        if let Some(input) = input
            && let Some(mut stdin) = child.stdin.take()
        {
            // it may exit without reading all of it, which shows in its status
            _ = stdin.write_all(input);
        }
        let status = child.wait().map_err(|e| format!("cannot wait for {name}: {e}"))?;
        Ok(status.code().unwrap_or(1))
    }

    /// [`Self::run`] for `program` with `args`
    fn exec<S: AsRef<std::ffi::OsStr>>(&self, program: &str, args: &[S]) -> Result<i32, String> {
        Self::run(self.cmd(program).args(args), None)
    }

    /// the input, at most `max` bytes of it
    fn input(&self, max: usize) -> Result<&[u8], String> {
        if self.input.len() > max {
            return Err(format!("the input is over {max} bytes"));
        }
        Ok(&self.input)
    }

    /// the first line of the input, a password or a hash, without its newline
    fn input_line(&self) -> Result<String, String> {
        let input = std::str::from_utf8(&self.input).map_err(|_| "the input isn't UTF-8")?;
        let line = input.strip_suffix('\n').unwrap_or(input);
        if line.is_empty() || line.contains(['\n', '\0']) {
            return Err("expected one line of input".to_owned());
        }
        Ok(line.to_owned())
    }

    #[allow(clippy::too_many_lines)]
    fn execute(&self, verb: &Verb) -> Result<i32, String> {
        match verb {
            Verb::MarkConfigured => {
                for marker in ["/var/.unconfigured", "/.unconfigured"] {
                    remove_any(Path::new(marker)).map_err(io(marker))?;
                }
                self.exec("systemctl", &["disable", "taidan-initial-setup-reconfiguration"])
            }
            Verb::Reboot => self.exec("systemctl", &["reboot"]),
            Verb::PowerOff => self.exec("systemctl", &["poweroff"]),
            Verb::DebugShell { wayland_display } => {
                let cmdline =
                    std::fs::read_to_string("/proc/cmdline").map_err(io("/proc/cmdline"))?;
                if !cmdline_has_debug_shell(&cmdline) {
                    return Err("boot with taidan.debug_shell for the debug terminal".to_owned());
                }
                Self::run(
                    self.cmd("xdg-terminal-exec")
                        .env("WAYLAND_DISPLAY", wayland_display)
                        .env("XDG_RUNTIME_DIR", format!("/run/user/{}", self.caller))
                        .args(["--", "sh", "-c", DEBUG_SHELL_SCRIPT]),
                    None,
                )
            }
            Verb::StartPortal { socket, lang, uri } => {
                self.start_portal(socket, lang.as_deref(), uri)
            }
            Verb::StopPortal => self.exec("systemctl", &["stop", PORTAL_UNIT]),
            Verb::KillLeftovers => self.kill_leftovers("-TERM"),
            Verb::KillLeftoversNow => self.kill_leftovers("-KILL"),
            Verb::DropPolkitRule => {
                remove_any(Path::new(POLKIT_RULE)).map_err(io(POLKIT_RULE))?;
                Ok(0)
            }
            Verb::RecordCreated { user, keep_home } => {
                if user_exists(user) {
                    return Err(format!("`{user}` exists already, only a new user is recorded"));
                }
                crate::passwd::write_record(user, *keep_home)
                    .map_err(io("cannot write the record"))?;
                Ok(0)
            }
            Verb::ClearRecord => {
                remove_any(Path::new(crate::passwd::CREATED_RECORD))
                    .map_err(io(crate::passwd::CREATED_RECORD))?;
                Ok(0)
            }
            Verb::RemoveUser { user, keep_home } => self.remove_user(user, *keep_home),
            Verb::UserAdd { user, fullname, uid } => self.useradd(user, fullname, *uid),
            Verb::HomedCreate { user, fullname, skel } => {
                self.homed_create(user, fullname, skel.as_deref())
            }
            Verb::SetAvatar { user } => {
                check_created(user)?;
                let image = self.input(MAX_INPUT.try_into().unwrap_or(usize::MAX))?;
                let icon = format!("/var/lib/AccountsService/icons/{user}");
                write_atomic(Path::new(&icon), image, 0o644).map_err(io(&icon))?;
                // a new user has no settings there yet, so this doesn't overwrite anything of theirs
                let dest = format!("/var/lib/AccountsService/users/{user}");
                let content = format!("[User]\nIcon={icon}\nSystemAccount=false\n");
                write_atomic(Path::new(&dest), content.as_bytes(), 0o600).map_err(io(&dest))?;
                Ok(0)
            }
            Verb::UnlockUser { user } => {
                check_created(user)?;
                self.exec("usermod", &["-U", "--", user])
            }
            Verb::HomedUnlock { user } => {
                check_created(user)?;
                let password = self.input_line()?;
                Self::run(
                    self.cmd("homectl").env("PASSWORD", password).args([
                        "update",
                        "--locked=no",
                        "--",
                        user,
                    ]),
                    None,
                )
            }
            Verb::AddToVideo { user } => {
                check_created(user)?;
                self.exec("usermod", &["-aG", "video", "--", user])
            }
            Verb::AsUser { user, argv } => {
                check_created(user)?;
                let input = (!self.input.is_empty()).then_some(self.input.as_slice());
                Self::run(self.cmd("runuser").args(["-u", user, "--"]).args(argv), input)
            }
            Verb::AsUserHomed { user, argv } => {
                check_created(user)?;
                let password = self.input_line()?;
                // homectl takes the password from `$PASSWORD`, which what runs in the home doesn't
                // get
                Self::run(
                    self.cmd("homectl")
                        .env("PASSWORD", password)
                        .args([
                            "with", "--", user, "env", "-u", "PASSWORD", "runuser", "-u", user,
                            "--",
                        ])
                        .args(argv),
                    None,
                )
            }
            Verb::RunTweak { id, on } => {
                let dir = Path::new(TWEAKS_DIR).join(id);
                // a real directory, not a symlink out of the tweaks
                if !std::fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
                    return Err(format!("no tweak `{id}`"));
                }
                let tweak = crate::tweaks::Tweak::from_dir(dir)
                    .map_err(|e| format!("invalid tweak `{id}`: {e}"))?;
                let up = tweak.path.join("up");
                let up = up.to_str().ok_or("invalid tweak path")?;
                Self::run(self.cmd(up).arg(if *on { "1" } else { "0" }), Some(&self.input))
            }
            Verb::RecordTweaks { ids } => {
                crate::tweaks::write_on_record(ids)
                    .map_err(io("cannot write the tweaks record"))?;
                Ok(0)
            }
            Verb::ClearTweakRecord => {
                remove_any(Path::new(crate::tweaks::ON_RECORD))
                    .map_err(io(crate::tweaks::ON_RECORD))?;
                Ok(0)
            }
            Verb::ProxyPersist => {
                let proxy = self.request_proxy()?;
                proxy.persist_as_root()?;
                Ok(0)
            }
            Verb::ProxyRepoOverrides => {
                let proxy = self.request_proxy()?;
                proxy
                    .write_repo_overrides_as_root()
                    .map_err(io("cannot write the dnf proxy overrides"))?;
                Ok(0)
            }
            Verb::ProxyClear => {
                crate::proxy::clear_as_root().map_err(io("cannot remove the saved proxy"))?;
                Ok(0)
            }
            Verb::Geoclue { on } => {
                self.exec("systemctl", &[if *on { "unmask" } else { "mask" }, "geoclue.service"])
            }
            Verb::EnableNtp => self.exec("timedatectl", &["set-ntp", "true"]),
            Verb::SetTimezone { timezone } => {
                if !Path::new("/usr/share/zoneinfo").join(timezone).is_file() {
                    return Err(format!("no time zone `{timezone}`"));
                }
                self.exec("timedatectl", &["set-timezone", "--", timezone])
            }
            Verb::SetHostname { hostname, pretty } => {
                let kind = if *pretty { "--pretty" } else { "--static" };
                self.exec("hostnamectl", &["hostname", kind, "--", hostname])
            }
            Verb::SetLocale { locale } => {
                self.exec("localectl", &["set-locale", &format!("LANG={locale}")])
            }
            Verb::SetX11Keymap { layouts, variants, options } => {
                self.exec("localectl", &["set-x11-keymap", layouts, "", variants, options])
            }
            Verb::RepoQuery { pkgs } => {
                let mut args = vec!["-q", "repoquery", "--latest-limit", "1", "--qf", "%{name}\\n"];
                args.extend(pkgs.iter().map(String::as_str));
                self.exec("dnf5", &args)
            }
            Verb::EnableRepo { repo } => {
                if repo.starts_with("https://") {
                    // overwritten, so a retried install doesn't fail on the file the first attempt
                    // added
                    let from = format!("--from-repofile={repo}");
                    self.exec("dnf5", &["config-manager", "addrepo", "--overwrite", &from])
                } else {
                    self.exec("dnf5", &["config-manager", "setopt", &format!("{repo}.enabled=1")])
                }
            }
            Verb::CoprEnable { copr } => self.exec("dnf5", &["copr", "enable", "-y", copr]),
            Verb::DnfExtras { pkgs, download_only } => {
                let mut args = crate::steps::DNF_EXTRAS_ARGS.to_vec();
                if *download_only {
                    args.push("--downloadonly");
                }
                args.extend(pkgs.iter().map(String::as_str));
                self.exec("dnf5", &args)
            }
            Verb::DnfSwapFfmpeg => {
                self.exec("dnf5", &["swap", "-y", "--allowerasing", "ffmpeg-free", "ffmpeg"])
            }
            Verb::DnfDrivers { pkgs } => {
                let mut args = vec!["in", "-y", "--allowerasing", "--best"];
                args.extend(pkgs.iter().map(String::as_str));
                self.exec("dnf", &args)
            }
            Verb::RpmOstreeInstall { pkgs } => {
                // idempotent, so a retried install doesn't fail on packages it already layered
                let mut args = vec!["install", "-y", "--idempotent"];
                args.extend(pkgs.iter().map(String::as_str));
                self.exec("rpm-ostree", &args)
            }
            Verb::FlatpakInstall { refs, download_only } => {
                let stage = if *download_only { "--no-deploy" } else { "--no-pull" };
                let mut args = vec!["install", "-y", "--noninteractive", stage];
                args.extend(refs.iter().map(String::as_str));
                self.exec("flatpak", &args)
            }
            Verb::RunScript { script } => self.exec("sh", &["-c", script]),
            Verb::DnfUpgradeOffline => self.exec("dnf5", &["upgrade", "--offline", "-y"]),
            Verb::DnfOfflineClean => self.exec("dnf5", &["offline", "clean", "-y"]),
            // what `dnf5 offline reboot` does but the reboot, which is up to the user
            Verb::DnfOfflineArm => Self::run(
                self.cmd("dnf5")
                    .env("DNF_SYSTEM_UPGRADE_NO_REBOOT", "1")
                    .args(["offline", "reboot", "-y"]),
                None,
            ),
            // RPM Fusion's own kargs, as its `%posttrans` sets with grubby on dnf systems. simpledrm
            // stays, the LUKS prompt and a failed driver need it, and nvidia-drm hands off from it.
            // append-if-missing so a retry doesn't duplicate kargs. rpm-ostree exits 0 on no changes
            Verb::NvidiaKargs => self.exec(
                "rpm-ostree",
                &[
                    "kargs",
                    "--append-if-missing=rd.driver.blacklist=nouveau,nova_core",
                    "--append-if-missing=modprobe.blacklist=nouveau,nova_core",
                ],
            ),
            Verb::NvidiaPrimaryGpu => {
                const DEST: &str = "/etc/X11/xorg.conf.d/10-nvidia-primary.conf";
                write_atomic(
                    Path::new(DEST),
                    crate::steps::NVIDIA_PRIMARY_GPU_CONF.as_bytes(),
                    0o644,
                )
                .map_err(io(DEST))?;
                Ok(0)
            }
            Verb::DefaultKernel => self.exec("grubby", &["--default-kernel"]),
            Verb::Akmods { kernel } => self.exec("akmods", &["--kernels", kernel]),
            Verb::InstallCert { network, attempt, file } => {
                let dest = Path::new(CERT_DIR).join(network).join(attempt).join(file);
                let content = self.input(1 << 20)?;
                write_atomic(&dest, content, 0o600).map_err(io(&dest.to_string_lossy()))?;
                Ok(0)
            }
            Verb::PruneCerts { network, keep } => {
                let dir = Path::new(CERT_DIR).join(network);
                let entries = match std::fs::read_dir(&dir) {
                    Ok(entries) => entries,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
                    Err(e) => return Err(format!("{}: {e}", dir.display())),
                };
                for entry in entries {
                    let entry = entry.map_err(io(&dir.to_string_lossy()))?;
                    if keep.as_deref().is_some_and(|keep| entry.file_name() == *keep) {
                        continue;
                    }
                    remove_any(&entry.path()).map_err(io(&entry.path().to_string_lossy()))?;
                }
                Ok(0)
            }
            Verb::RemoveCertAttempt { network, attempt } => {
                let dir = Path::new(CERT_DIR).join(network).join(attempt);
                remove_any(&dir).map_err(io(&dir.to_string_lossy()))?;
                Ok(0)
            }
        }
    }

    /// the request's proxy, for the verbs that save it
    fn request_proxy(&self) -> Result<&crate::proxy::Proxy, String> {
        self.proxy.as_ref().ok_or_else(|| "the request has no proxy".to_owned())
    }

    /// starts the captive portal browser, see [`Verb::StartPortal`]. It waits for the browser to
    /// quit, which the caller watches for
    fn start_portal(&self, socket: &str, lang: Option<&str>, uri: &str) -> Result<i32, String> {
        // the caller's own display, not another session's
        if !socket.starts_with(&format!("/run/user/{}/", self.caller)) {
            return Err(format!("`{socket}` isn't in the caller's runtime dir"));
        }
        let meta = std::fs::symlink_metadata(socket).map_err(io(socket))?;
        if !meta.file_type().is_socket() || meta.uid() != self.caller {
            return Err(format!("`{socket}` isn't the caller's socket"));
        }
        let home = format!("/run/{PORTAL_USER}");
        let mut cmd = self.cmd("systemd-run");
        cmd.args(["--quiet", "--wait", "--collect"])
            .arg(format!("--unit={PORTAL_UNIT}"))
            .arg(format!("--uid={PORTAL_USER}"))
            .arg(format!("--property=RuntimeDirectory={PORTAL_USER}"))
            .args([
                "--property=NoNewPrivileges=yes",
                "--property=PrivateTmp=yes",
                "--property=ProtectHome=yes",
                "--property=ProtectSystem=full",
            ])
            .arg(format!("--property=BindReadOnlyPaths={socket}:{home}/{PORTAL_DISPLAY}"))
            .arg(format!("--setenv=WAYLAND_DISPLAY={PORTAL_DISPLAY}"))
            .arg(format!("--setenv=XDG_RUNTIME_DIR={home}"))
            .arg(format!("--setenv=HOME={home}"));
        if let Some(lang) = lang {
            cmd.arg(format!("--setenv=LANG={lang}"));
        }
        // through a file rather than `--setenv`, since anyone can read a command's arguments and
        // the URL may carry a password. WebKit then uses GLib's default proxy resolver, and the
        // libproxy one reads `http_proxy` and friends. GNOME's would go by its own settings
        // instead, so it's picked explicitly
        if let Some(proxy) = &self.proxy {
            let mut vars = String::new();
            for (key, value) in
                proxy.vars().into_iter().chain([("GIO_USE_PROXY_RESOLVER", "libproxy")])
            {
                // quoted for systemd, the URL has no control characters
                let value = value.replace('\\', "\\\\").replace('"', "\\\"");
                vars.extend([key, "=\"", &value, "\"\n"]);
            }
            let file = Path::new(RUN_DIR).join("portal-proxy.env");
            ensure_run_dir().map_err(io(RUN_DIR))?;
            match write_atomic(&file, vars.as_bytes(), 0o600) {
                // read by systemd, as root
                Ok(()) => _ = cmd.arg(format!("--property=EnvironmentFile={}", file.display())),
                Err(e) => {
                    eprintln!("taidan-helper: cannot pass the proxy to the portal browser: {e}")
                }
            }
        }
        cmd.args(["--", PORTAL_BROWSER, "--platform", "gtk4", uri]);
        Self::run(&mut cmd, None)
    }

    /// sends `signal` to the root processes in the helper's cgroup, which `pkexec` keeps the
    /// caller's. That's what an earlier taidan started as root and left behind
    fn kill_leftovers(&self, signal: &str) -> Result<i32, String> {
        let cgroup =
            std::fs::read_to_string("/proc/self/cgroup").map_err(io("/proc/self/cgroup"))?;
        let cgroup = cgroup.lines().find_map(|l| l.strip_prefix("0::")).ok_or("no cgroup v2")?;
        let procs = format!("/sys/fs/cgroup{cgroup}/cgroup.procs");
        let me = std::process::id().to_string();
        let pids = std::fs::read_to_string(&procs).map_err(io(&procs))?;
        let pids = pids
            .lines()
            .filter(|pid| *pid != me && word(pid, ""))
            .filter(|pid| std::fs::metadata(format!("/proc/{pid}")).is_ok_and(|m| m.uid() == 0))
            .collect::<Vec<_>>();
        if pids.is_empty() {
            return Ok(0);
        }
        let mut args = vec![signal];
        args.extend(pids);
        self.exec("kill", &args)
    }

    /// see [`Verb::RemoveUser`]
    fn remove_user(&self, user: &str, keep_home: bool) -> Result<i32, String> {
        let record = crate::passwd::record()
            .filter(|r| r.user == user)
            .ok_or_else(|| format!("`{user}` isn't the user an earlier attempt made"))?;
        let Some(found) = uzers::get_user_by_name(user) else { return Ok(0) };
        let uid = found.uid();
        if !crate::passwd::removable(user, uid) {
            return Err(format!("`{user}` isn't a regular user (uid {uid})"));
        }
        let logins = crate::passwd::login_state(user);
        if logins == Some(true) {
            return Err(format!("`{user}` has logged in, it's someone's account now"));
        }
        // if it can't be told whether it was used, its home stays
        let keep_home = keep_home || record.keep_home || logins.is_none();
        let code = if crate::passwd::HOMED_UIDS.contains(&uid) {
            // homed can't remove a user and keep its home
            if logins.is_none() {
                return Err(format!("cannot tell whether `{user}` has been used"));
            }
            self.exec("homectl", &["remove", user])?
        } else if keep_home {
            self.exec("userdel", &["--", user])?
        } else {
            self.exec("userdel", &["-r", "--", user])?
        };
        if code != 0 {
            return Ok(code);
        }
        for dir in ["icons", "users"] {
            let path = format!("/var/lib/AccountsService/{dir}/{user}");
            if let Err(e) = remove_any(Path::new(&path)) {
                eprintln!("taidan-helper: cannot remove {path}: {e}");
            }
        }
        Ok(0)
    }

    /// see [`Verb::UserAdd`]
    fn useradd(&self, user: &str, fullname: &str, uid: Option<u32>) -> Result<i32, String> {
        check_new_recorded(user)?;
        let hash = self.input_line()?;
        check_hash(&hash)?;
        // a home kept from a reinstall stays its old owner's, so the user gets that uid back
        if let Some(uid) = uid
            && crate::steps::previous_uid(user) != Some(uid)
        {
            return Err(format!("uid {uid} doesn't own `{user}`'s home"));
        }
        let uid = uid.map(|u| u.to_string());
        let mut args = vec!["-c", fullname, "-m", "-G", "wheel"];
        if let Some(uid) = &uid {
            args.extend(["-u", uid]);
        }
        args.extend(["--", user]);
        let code = self.exec("useradd", &args)?;
        if code != 0 {
            return Ok(code);
        }
        write_created(user)?;
        let input = format!("{user}:{hash}\n");
        let code = Self::run(self.cmd("chpasswd").arg("-e"), Some(input.as_bytes()))?;
        if code != 0 {
            return Ok(code);
        }
        // until the installation finishes, see `steps::unlock_user`
        self.exec("usermod", &["-L", "--", user])
    }

    /// see [`Verb::HomedCreate`]
    fn homed_create(&self, user: &str, fullname: &str, skel: Option<&Path>) -> Result<i32, String> {
        check_new_recorded(user)?;
        let identity = homed_identity(&self.input)?;
        let mut args: Vec<OsString> = vec![
            "create".into(),
            "--identity=-".into(),
            "--real-name".into(),
            fullname.into(),
            // groups that don't exist are left out
            "--member-of=wheel,video".into(),
            "--enforce-password-policy=no".into(),
        ];
        if let Some(skel) = skel {
            // copied into the new home as root, so it must be the caller's own, not, say, /root
            let meta = std::fs::symlink_metadata(skel).map_err(io(&skel.to_string_lossy()))?;
            let canonical = std::fs::canonicalize(skel).map_err(io(&skel.to_string_lossy()))?;
            if !meta.is_dir() || meta.uid() != self.caller || canonical != skel {
                return Err(format!("`{}` isn't a directory of the caller's", skel.display()));
            }
            let mut arg = OsString::from("--skel=");
            arg.push(skel);
            args.push(arg);
        }
        args.push(user.into());
        let code = Self::run(self.cmd("homectl").args(&args), Some(&identity))?;
        if code == 0 {
            write_created(user)?;
        }
        Ok(code)
    }
}

/// the script the debug terminal runs, a warning and then a shell
const DEBUG_SHELL_SCRIPT: &str = "echo 'Taidan has detected Alt+Shift+T. A terminal with superuser privilege is opened for debugging purposes. Only proceed if you know what you are doing, OTHERWISE YOU MAY RISK DATA LOSS OR CAUSE DAMAGES TO YOUR DEVICE. To exit the terminal, press Ctrl+D.' && sh";

/// whether `taidan.debug_shell` is on the kernel command line, either bare or set to a true value
/// like systemd's `systemd.debug_shell`
#[must_use]
pub fn cmdline_has_debug_shell(cmdline: &str) -> bool {
    cmdline.split_ascii_whitespace().any(|arg| match arg.split_once('=') {
        None => arg == "taidan.debug_shell",
        Some((key, value)) => {
            key == "taidan.debug_shell" && matches!(value, "1" | "yes" | "y" | "true" | "on")
        }
    })
}

/// a crypt(3) hash for `chpasswd -e`, which mustn't end the line or the field early
fn check_hash(hash: &str) -> Result<(), String> {
    if hash.starts_with('$')
        && hash.len() <= 512
        && hash.chars().all(|c| c.is_ascii_graphic() && c != ':')
    {
        Ok(())
    } else {
        Err("invalid password hash".to_owned())
    }
}

/// the identity `homectl create` gets, rebuilt from only the fields Taidan sets, so nothing else
/// (a uid, more groups, a different home) can slip in
fn homed_identity(input: &[u8]) -> Result<Vec<u8>, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Identity {
        secret: Secret,
        privileged: Privileged,
        locked: bool,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Secret {
        password: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    struct Privileged {
        hashed_password: Vec<String>,
    }
    let id: Identity =
        serde_json::from_slice(input).map_err(|e| format!("invalid identity: {e}"))?;
    if id.secret.password.len() != 1 || id.privileged.hashed_password.len() != 1 {
        return Err("expected one password and one hash".to_owned());
    }
    id.privileged.hashed_password.iter().try_for_each(|h| check_hash(h))?;
    let out = serde_json::json!({
        "secret": { "password": id.secret.password },
        "privileged": { "hashedPassword": id.privileged.hashed_password },
        "locked": id.locked,
    });
    serde_json::to_vec(&out).map_err(|e| e.to_string())
}

fn user_exists(user: &str) -> bool {
    uzers::get_user_by_name(user).is_some()
}

/// `user` is the one in [`crate::passwd::CREATED_RECORD`] and doesn't exist yet
fn check_new_recorded(user: &str) -> Result<(), String> {
    if crate::passwd::recorded_user().as_deref() != Some(user) {
        return Err(format!("`{user}` isn't recorded, see record-created"));
    }
    if user_exists(user) {
        return Err(format!("`{user}` exists already"));
    }
    Ok(())
}

/// `user` is the one Taidan made, going by [`crate::passwd::CREATED_RECORD`] or, once that's
/// cleared, [`CREATED_USER`], and a regular user
fn check_created(user: &str) -> Result<(), String> {
    let recorded = crate::passwd::recorded_user().as_deref() == Some(user);
    let created = std::fs::read_to_string(CREATED_USER).is_ok_and(|u| u.trim() == user);
    if !recorded && !created {
        return Err(format!("`{user}` isn't the user Taidan made"));
    }
    match uzers::get_user_by_name(user) {
        Some(u) if !crate::passwd::regular_uid(u.uid()) => {
            Err(format!("`{user}` isn't a regular user (uid {})", u.uid()))
        }
        _ => Ok(()),
    }
}

/// [`RUN_DIR`], only root's
fn ensure_run_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(RUN_DIR)?;
    std::fs::set_permissions(RUN_DIR, std::fs::Permissions::from_mode(0o700))
}

/// writes `user` to [`CREATED_USER`]
fn write_created(user: &str) -> Result<(), String> {
    ensure_run_dir().map_err(io(RUN_DIR))?;
    write_atomic(Path::new(CREATED_USER), format!("{user}\n").as_bytes(), 0o600)
        .map_err(io(CREATED_USER))
}

/// removes `path`, a directory with what's in it, or nothing if it isn't there. A symlink is
/// removed, never followed
fn remove_any(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
    }
}

/// replaces `path` with `content`, with `mode`, all at once and on disk before it returns, so a
/// full disk or a power cut can't leave it half written. Makes the folders it's in
pub(crate) fn write_atomic(path: &Path, content: &[u8], mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let dir = path.parent().ok_or_else(|| std::io::Error::other("no parent directory"))?;
    let name = path.file_name().ok_or_else(|| std::io::Error::other("no file name"))?;
    std::fs::create_dir_all(dir)?;
    let mut tmp_name = OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".taidan-tmp");
    let tmp = dir.join(tmp_name);
    remove_any(&tmp)?;
    let mut file =
        std::fs::OpenOptions::new().write(true).create_new(true).mode(mode).open(&tmp)?;
    // the mode is masked by the umask
    file.set_permissions(std::fs::Permissions::from_mode(mode))?;
    file.write_all(content)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)?;
    std::fs::File::open(dir)?.sync_all()
}

// the OOBE's side

/// the helper to run, which debug builds take from `TAIDAN_HELPER` if it's set
fn program() -> OsString {
    #[cfg(debug_assertions)]
    if let Some(helper) = std::env::var_os("TAIDAN_HELPER") {
        return helper;
    }
    PATH.into()
}

/// what goes on the helper's stdin for `verb`: the request, with the installation's proxy, then
/// `input`
#[must_use]
pub fn stdin(verb: &Verb, input: &[u8]) -> Vec<u8> {
    stdin_with_proxy(verb, input, crate::proxy::get().as_ref())
}

/// [`stdin`] with `proxy` rather than the installation's, for before the installation starts
#[must_use]
pub fn stdin_with_proxy(verb: &Verb, input: &[u8], proxy: Option<&crate::proxy::Proxy>) -> Vec<u8> {
    let request = Request { verb: verb.clone(), proxy: proxy.cloned() };
    // a struct of strings, numbers and paths that came from `str`s, which always serializes
    let mut out = serde_json::to_vec(&request).unwrap_or_default();
    out.push(b'\n');
    out.extend_from_slice(input);
    out
}

/// `pkexec` running the helper for `verb`, whose stdin must get [`stdin`]
#[must_use]
pub fn command(verb: &Verb) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("pkexec");
    cmd.arg(program()).arg(verb.name());
    cmd
}

/// [`command`], for the standard library's [`Command`]
#[must_use]
pub fn std_command(verb: &Verb) -> Command {
    let mut cmd = Command::new("pkexec");
    cmd.arg(program()).arg(verb.name());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_owned()
    }

    /// one of every verb
    fn every_verb() -> Vec<Verb> {
        let user = s("cypress");
        vec![
            Verb::MarkConfigured,
            Verb::Reboot,
            Verb::PowerOff,
            Verb::DebugShell { wayland_display: s("wayland-0") },
            Verb::StartPortal {
                socket: s("/run/user/990/wayland-0"),
                lang: Some(s("en_US.UTF-8")),
                uri: s("http://nmcheck.gnome.org/check_network_status.txt"),
            },
            Verb::StopPortal,
            Verb::KillLeftovers,
            Verb::KillLeftoversNow,
            Verb::DropPolkitRule,
            Verb::RecordCreated { user: user.clone(), keep_home: false },
            Verb::ClearRecord,
            Verb::RemoveUser { user: user.clone(), keep_home: true },
            Verb::UserAdd { user: user.clone(), fullname: s("Cypress Reed"), uid: Some(1000) },
            Verb::HomedCreate {
                user: user.clone(),
                fullname: s("Cypress"),
                skel: Some("/tmp/libtaidan-x/skel".into()),
            },
            Verb::SetAvatar { user: user.clone() },
            Verb::UnlockUser { user: user.clone() },
            Verb::HomedUnlock { user: user.clone() },
            Verb::AddToVideo { user: user.clone() },
            Verb::AsUser {
                user: user.clone(),
                argv: vec![s("kwriteconfig6"), s("--file"), s("kxkbrc")],
            },
            Verb::AsUserHomed { user, argv: vec![s("sh"), s("-c"), s("true")] },
            Verb::RunTweak { id: s("mtu-probing"), on: true },
            Verb::RecordTweaks { ids: vec![s("mtu-probing"), s("?cachyos-kernel")] },
            Verb::ClearTweakRecord,
            Verb::ProxyPersist,
            Verb::ProxyRepoOverrides,
            Verb::ProxyClear,
            Verb::Geoclue { on: false },
            Verb::EnableNtp,
            Verb::SetTimezone { timezone: s("America/Denver") },
            Verb::SetHostname { hostname: s("cypresss-laptop"), pretty: false },
            Verb::SetLocale { locale: s("en_US.UTF-8") },
            Verb::SetX11Keymap {
                layouts: s("us,ru"),
                variants: s(","),
                options: s("grp:alt_shift_toggle"),
            },
            Verb::RepoQuery { pkgs: vec![s("langpacks-pt_BR")] },
            Verb::EnableRepo { repo: s("terra") },
            Verb::CoprEnable { copr: s("owner/project") },
            Verb::DnfExtras { pkgs: vec![s("@multimedia")], download_only: true },
            Verb::DnfSwapFfmpeg,
            Verb::DnfDrivers { pkgs: vec![s("akmod-nvidia")] },
            Verb::RpmOstreeInstall { pkgs: vec![s("broadcom-wl")] },
            Verb::FlatpakInstall { refs: vec![s("org.mozilla.firefox")], download_only: false },
            Verb::RunScript { script: s("true") },
            Verb::DnfUpgradeOffline,
            Verb::DnfOfflineClean,
            Verb::DnfOfflineArm,
            Verb::NvidiaKargs,
            Verb::NvidiaPrimaryGpu,
            Verb::DefaultKernel,
            Verb::Akmods { kernel: s("6.17.1-300.fc43.x86_64") },
            Verb::InstallCert {
                network: s("corp-wifi-436f7270"),
                attempt: s("1760000000"),
                file: s("ca.pem"),
            },
            Verb::PruneCerts { network: s("wired"), keep: Some(s("1760000000")) },
            Verb::RemoveCertAttempt { network: s("wired"), attempt: s("1760000000") },
        ]
    }

    #[test]
    fn names_are_the_serde_tags() {
        for verb in every_verb() {
            let json = serde_json::to_value(&verb).unwrap();
            assert_eq!(json["verb"], verb.name(), "{verb:?}");
            assert_eq!(serde_json::from_value::<Verb>(json).unwrap(), verb);
        }
    }

    #[test]
    fn samples_are_valid_for_root() {
        for verb in every_verb() {
            assert_eq!(verb.validate(true), Ok(()), "{verb:?}");
        }
    }

    #[test]
    fn root_only_verbs() {
        for verb in [
            Verb::EnableRepo { repo: s("terra") },
            Verb::CoprEnable { copr: s("owner/project") },
            Verb::RunScript { script: s("true") },
        ] {
            assert!(verb.validate(false).is_err(), "{verb:?}");
            assert!(verb.validate(true).is_ok(), "{verb:?}");
        }
        assert!(Verb::EnableRepo { repo: s("http://evil/x.repo") }.validate(true).is_err());
        assert!(Verb::EnableRepo { repo: s("--setopt=x") }.validate(true).is_err());
    }

    #[test]
    fn unknown_fields_and_verbs_are_refused() {
        let parse = |j: &str| serde_json::from_str::<Request>(j);
        assert!(parse(r#"{"verb":{"verb":"reboot"}}"#).is_ok());
        // serde ignores extra fields on a unit variant, harmless since there's nothing to read
        // them into, but not on the others
        assert!(parse(r#"{"verb":{"verb":"set-locale","locale":"C","argv":["sh"]}}"#).is_err());
        assert!(parse(r#"{"verb":{"verb":"as-user","user":"a","argv":["sh"],"env":[]}}"#).is_err());
        assert!(parse(r#"{"verb":{"verb":"exec","argv":["sh"]}}"#).is_err());
        assert!(parse(r#"{"verb":{"verb":"reboot"},"env":{"LD_PRELOAD":"x"}}"#).is_err());
        assert!(parse(r#"{"verb":{"verb":"set-locale"}}"#).is_err());
    }

    #[test]
    fn users() {
        let ok = |u: &str| Verb::UnlockUser { user: s(u) }.validate(false).is_ok();
        assert!(ok("cypress"));
        assert!(ok("a-b_c1"));
        for bad in ["root", "taidan", "nobody", "-r", "Cypress", "a b", "a:b", "", "../x", "a/b"] {
            assert!(!ok(bad), "{bad}");
        }
        let add = |fullname: &str, uid| {
            Verb::UserAdd { user: s("cypress"), fullname: s(fullname), uid }.validate(false)
        };
        assert!(add("Cypress Reed", None).is_ok());
        assert!(add("Cypress Reed", Some(1001)).is_ok());
        assert!(add("Cypress Reed", Some(0)).is_err());
        assert!(add("Cypress Reed", Some(60001)).is_err());
        assert!(add("evil:0:0", None).is_err());
        assert!(add("two\nlines", None).is_err());
        assert!(add("a,b", None).is_err());
        let skel = |p: &str| {
            Verb::HomedCreate { user: s("cypress"), fullname: s("C"), skel: Some(p.into()) }
                .validate(false)
        };
        assert!(skel("/tmp/libtaidan-x/skel").is_ok());
        assert!(skel("skel").is_err());
    }

    #[test]
    fn as_user_argv() {
        let ok = |argv: &[&str]| {
            Verb::AsUser { user: s("cypress"), argv: argv.iter().map(|a| s(a)).collect() }
                .validate(false)
                .is_ok()
        };
        assert!(ok(&["kwriteconfig6", "--file", "x"]));
        assert!(!ok(&[]));
        assert!(!ok(&["--preserve-environment"]));
        assert!(!ok(&[""]));
        assert!(!ok(&["sh", "a\0b"]));
        assert!(Verb::AsUser { user: s("root"), argv: vec![s("sh")] }.validate(true).is_err());
    }

    #[test]
    fn packages() {
        let extras = |pkgs: &[&str], root| {
            Verb::DnfExtras { pkgs: pkgs.iter().map(|p| s(p)).collect(), download_only: false }
                .validate(root)
        };
        assert!(
            extras(&["@multimedia", "langpacks-pt_BR", "ultramarine-langpacks-zh"], false).is_ok()
        );
        assert!(
            extras(&["fcitx5-autostart", "ibus-wayland", "fcitx5-chinese-addons"], false).is_ok()
        );
        assert!(extras(&["fcitx5-mozc", "ibus-chewing"], false).is_ok());
        assert!(extras(&["sudo"], false).is_err());
        assert!(extras(&["sudo"], true).is_ok());
        assert!(extras(&[], true).is_err());
        assert!(extras(&["--setopt=reposdir=/tmp"], true).is_err());
        assert!(extras(&["-y"], true).is_err());
        assert!(extras(&["kernel*"], true).is_err());
        assert!(extras(&["/tmp/evil.rpm"], true).is_err());
        assert!(extras(&["https://evil/x.rpm"], true).is_err());
        assert!(extras(&["langpacks-../x"], false).is_err());
        let drivers = |pkgs: &[&str], root| {
            Verb::DnfDrivers { pkgs: pkgs.iter().map(|p| s(p)).collect() }.validate(root)
        };
        assert!(
            drivers(&["nvidia-gpu-firmware", "akmod-nvidia-580xx", "broadcom-wl"], false).is_ok()
        );
        assert!(drivers(&["@multimedia"], false).is_err());
        let ostree = Verb::RpmOstreeInstall { pkgs: vec![s("akmod-wl"), s("broadcom-wl")] };
        assert!(ostree.validate(false).is_ok());
        let flatpak = |r: &str| {
            Verb::FlatpakInstall { refs: vec![s(r)], download_only: true }.validate(false)
        };
        assert!(flatpak("org.mozilla.firefox").is_ok());
        assert!(flatpak("app/org.gimp.GIMP/x86_64/stable").is_ok());
        assert!(flatpak("--from=https://evil/x.flatpakref").is_err());
        assert!(flatpak("/tmp/x.flatpak").is_err());
    }

    #[test]
    fn tweaks() {
        let ok = |id: &str| Verb::RunTweak { id: s(id), on: true }.validate(false).is_ok();
        assert!(ok("mtu-probing"));
        assert!(ok("cachyos_kernel"));
        for bad in ["", "..", "../../bin", "a/b", ".hidden", "-x", "a b"] {
            assert!(!ok(bad), "{bad}");
        }
        let record = |ids: &[&str]| {
            Verb::RecordTweaks { ids: ids.iter().map(|i| s(i)).collect() }.validate(false)
        };
        assert!(record(&["a", "?b"]).is_ok());
        assert!(record(&[]).is_ok());
        assert!(record(&["a\nroot"]).is_err());
        assert!(record(&["??b"]).is_err());
    }

    #[test]
    fn system_settings() {
        let tz = |t: &str| Verb::SetTimezone { timezone: s(t) }.validate(false).is_ok();
        assert!(tz("America/Argentina/Buenos_Aires"));
        assert!(tz("Etc/GMT+5"));
        assert!(tz("UTC"));
        for bad in ["/etc/passwd", "../../etc/passwd", "America/../../x", "a//b", "", "a b"] {
            assert!(!tz(bad), "{bad}");
        }
        let host =
            |h: &str, pretty| Verb::SetHostname { hostname: s(h), pretty }.validate(false).is_ok();
        assert!(host("cypresss-laptop", false));
        assert!(!host("-x", false));
        assert!(!host("a b", false));
        assert!(!host(&"a".repeat(65), false));
        assert!(host("Cypress's Laptop", true));
        assert!(!host("a\nb", true));
        assert!(!host("  ", true));
        let locale = |l: &str| Verb::SetLocale { locale: s(l) }.validate(false).is_ok();
        assert!(locale("pt_BR.UTF-8"));
        assert!(locale("sr_RS@latin"));
        assert!(!locale("en_US.UTF-8 LC_ALL=C"));
        assert!(!locale("--help"));
        let keymap = |l: &str, o: &str| {
            Verb::SetX11Keymap { layouts: s(l), variants: s(""), options: s(o) }
                .validate(false)
                .is_ok()
        };
        assert!(keymap("us,ru", "grp:win_space_toggle"));
        assert!(!keymap("us ru", ""));
        assert!(!keymap("us", "a\nb"));
        let kernel = |k: &str| Verb::Akmods { kernel: s(k) }.validate(false).is_ok();
        assert!(kernel("6.17.1-300.fc43.x86_64+debug"));
        assert!(!kernel("--all"));
        assert!(!kernel("../x"));
    }

    #[test]
    fn certs() {
        let file = |f: &str| {
            Verb::InstallCert { network: s("wired"), attempt: s("123"), file: s(f) }
                .validate(false)
                .is_ok()
        };
        assert!(file("ca.pem"));
        assert!(file("key"));
        assert!(file("pac.p12"));
        for bad in ["ca.", "evil.pem", "ca.pem/x", "ca.PEM", "ca.toolongext", "../ca.pem", "ca.p.m"]
        {
            assert!(!file(bad), "{bad}");
        }
        let net = |n: &str, a: &str| {
            Verb::RemoveCertAttempt { network: s(n), attempt: s(a) }.validate(false).is_ok()
        };
        assert!(net("cafe-436166c3a9", "1760000000"));
        assert!(!net("..", "1"));
        assert!(!net("wired", ".."));
        assert!(!net("wired", ""));
        assert!(!net("a/b", "1"));
        assert!(Verb::PruneCerts { network: s("wired"), keep: None }.validate(false).is_ok());
        assert!(
            Verb::PruneCerts { network: s("wired"), keep: Some(s("../x")) }
                .validate(false)
                .is_err()
        );
    }

    #[test]
    fn portal_and_debug_shell() {
        let portal = |socket: &str, uri: &str| {
            Verb::StartPortal { socket: s(socket), lang: None, uri: s(uri) }.validate(false).is_ok()
        };
        assert!(portal("/run/user/990/wayland-0", "https://example.com/"));
        assert!(!portal("/run/user/990/wayland-0", "file:///etc/shadow"));
        assert!(!portal("/run/user/990/wayland-0", "javascript:alert(1)"));
        assert!(!portal("/tmp/wayland-0", "http://example.com/"));
        assert!(!portal("/run/user/990/../0/wayland-0", "http://example.com/"));
        assert!(!portal("/run/user/990/a:b", "http://example.com/"));
        let lang = Verb::StartPortal {
            socket: s("/run/user/1/w"),
            lang: Some(s("C\nX=1")),
            uri: s("http://a/"),
        };
        assert!(lang.validate(false).is_err());
        let display = |d: &str| Verb::DebugShell { wayland_display: s(d) }.validate(false).is_ok();
        assert!(display("wayland-0"));
        assert!(!display("../../../tmp/sock"));
        assert!(!display(""));
    }

    #[test]
    fn debug_shell_needs_the_flag() {
        assert!(cmdline_has_debug_shell("BOOT_IMAGE=/vmlinuz ro taidan.debug_shell quiet\n"));
        assert!(cmdline_has_debug_shell("ro taidan.debug_shell=1"));
        assert!(cmdline_has_debug_shell("ro taidan.debug_shell=yes"));
        assert!(!cmdline_has_debug_shell("BOOT_IMAGE=/vmlinuz ro rhgb quiet\n"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shell=0"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shell=no"));
        assert!(!cmdline_has_debug_shell("ro systemd.debug_shell"));
        assert!(!cmdline_has_debug_shell("ro taidan.debug_shellx"));
    }

    #[test]
    fn hashes_and_identities() {
        assert!(check_hash("$y$j9T$abc$def/.").is_ok());
        assert!(check_hash("plain").is_err());
        assert!(check_hash("$y$a:b").is_err());
        assert!(check_hash("$y$a\nroot:x").is_err());
        let identity = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"]},"locked":true}"#;
        let out: serde_json::Value =
            serde_json::from_slice(&homed_identity(identity).unwrap()).unwrap();
        assert_eq!(out["locked"], true);
        assert_eq!(out["privileged"]["hashedPassword"][0], "$y$x");
        let extra = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"]},"locked":false,"uid":0}"#;
        assert!(homed_identity(extra).is_err());
        let groups = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"],"memberOf":["root"]},"locked":false}"#;
        assert!(homed_identity(groups).is_err());
        let two = br#"{"secret":{"password":["a","b"]},"privileged":{"hashedPassword":["$y$x"]},"locked":false}"#;
        assert!(homed_identity(two).is_err());
    }

    #[test]
    fn after_setup_is_narrow() {
        let after: Vec<_> =
            every_verb().into_iter().filter(Verb::after_setup).map(|v| v.name()).collect();
        assert_eq!(
            after,
            [
                "mark-configured",
                "reboot",
                "power-off",
                "stop-portal",
                "kill-leftovers",
                "kill-leftovers-now",
                "drop-polkit-rule",
                "clear-record",
                "as-user",
                "as-user-homed",
                "clear-tweak-record",
            ]
        );
    }

    #[test]
    fn stdin_is_a_request_line_then_the_input() {
        let verb = Verb::SetAvatar { user: s("cypress") };
        let bytes = stdin(&verb, b"\x89PNG\n\0binary");
        let newline = bytes.iter().position(|&b| b == b'\n').unwrap();
        let request: Request = serde_json::from_slice(&bytes[..newline]).unwrap();
        assert_eq!(request.verb, verb);
        assert_eq!(&bytes[newline + 1..], b"\x89PNG\n\0binary");
    }

    #[test]
    fn writes_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/file");
        write_atomic(&path, b"one", 0o600).unwrap();
        write_atomic(&path, b"two", 0o644).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o644);
        assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        remove_any(&dir.path().join("a")).unwrap();
        remove_any(&dir.path().join("a")).unwrap();
        assert!(!dir.path().join("a").exists());
    }
}
