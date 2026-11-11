use std::{
    ffi::OsString,
    io::{BufRead as _, Read as _, Write as _},
    os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const PATH: &str = "/usr/libexec/taidan-helper";

const SAFE_PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

const RUN_DIR: &str = "/run/taidan-helper";

const CREATED_USER_THIS_BOOT: &str = "/run/taidan-helper/created-user";

pub const CERT_DIR: &str = "/etc/pki/taidan/8021x";

const TWEAKS_DIR: &str = "/usr/share/taidan/tweaks";

const POLKIT_RULE: &str = "/run/polkit-1/rules.d/100-taidan.rules";

pub const GDM_DCONF_PROFILE: &str = "/usr/share/dconf/profile/gdm";

const GDM_SCREEN_READER: &str = "/etc/dconf/db/gdm.d/50-taidan-screen-reader";
const GDM_SCREEN_READER_KEYFILE: &str = "# written by Taidan, the screen reader was on during setup
[org/gnome/desktop/a11y/applications]
screen-reader-enabled=true
";

const GEOCLUE_UNIT: &str = "geoclue.service";
const LOCATION_OFF: &str = "/etc/dconf/db/local.d/50-taidan-location";
const LOCATION_OFF_KEYFILE: &str = "# written by Taidan, location services were off during setup
[org/gnome/system/location]
enabled=false
";

const PORTAL_UNIT: &str = "taidan-portal.service";
const PORTAL_USER: &str = "taidan-portal";
const PORTAL_DISPLAY: &str = "wayland-0";
const PORTAL_BROWSER: &str = "/usr/bin/cog";

const MAX_REQUEST_BYTES: u64 = 1 << 20;
const MAX_INPUT_BYTES: u64 = 32 << 20;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verb", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Verb {
    MarkConfigured,
    Reboot,
    PowerOff,
    DebugShell { wayland_display: String },
    StartPortal { socket: String, lang: Option<String>, uri: String },
    StopPortal,
    KillLeftovers,
    KillLeftoversNow,
    DropPolkitRule,
    RecordCreated { user: String, keep_home: bool },
    ClearRecord,
    RemoveUser { user: String, keep_home: bool },
    UserAdd { user: String, fullname: String, uid: Option<u32> },
    HomedCreate { user: String, fullname: String, skel: Option<PathBuf> },
    SetAvatar { user: String },
    UnlockUser { user: String },
    HomedUnlock { user: String },
    AddToVideo { user: String },
    AsUser { user: String, argv: Vec<String> },
    AsUserHomed { user: String, argv: Vec<String> },
    RunTweak { id: String, on: bool },
    RecordTweaks { ids: Vec<String> },
    ClearTweakRecord,
    ProxyPersist,
    ProxyRepoOverrides,
    ProxyClear,
    Location { on: bool },
    GdmScreenReader { on: bool },
    EnableNtp,
    SetTimezone { timezone: String },
    SetHostname { hostname: String, pretty: bool },
    SetLocale { locale: String },
    SetX11Keymap { layouts: String, variants: String, options: String },
    RepoQuery { pkgs: Vec<String> },
    EnableRepo { repo: String },
    CoprEnable { copr: String },
    DnfExtras { pkgs: Vec<String>, download_only: bool },
    DnfSwapFfmpeg,
    DnfDrivers { pkgs: Vec<String> },
    RpmOstreeInstall { pkgs: Vec<String> },
    FlatpakInstall { refs: Vec<String>, download_only: bool },
    RunScript { script: String },
    DnfUpgradeOffline,
    DnfOfflineClean,
    DnfOfflineArm,
    NvidiaKargs,
    NvidiaPrimaryGpu,
    DefaultKernel,
    Akmods { kernel: String },
    InstallCert { network: String, attempt: String, file: String },
    PruneCerts { network: String, keep: Option<String> },
    RemoveCertAttempt { network: String, attempt: String },
}

// `pkexec` clears the environment and logs the arguments, so the request goes on stdin
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    verb: Verb,
    #[serde(default)]
    proxy: Option<crate::proxy::Proxy>,
}

impl Verb {
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
            Self::Location { .. } => "location",
            Self::GdmScreenReader { .. } => "gdm-screen-reader",
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
                | Self::AsUser { .. }
                | Self::AsUserHomed { .. }
                | Self::GdmScreenReader { .. }
        )
    }

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
            | Self::Location { .. }
            | Self::GdmScreenReader { .. }
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
                check_pkgs(pkgs, |p| root_caller || crate::steps::driver_packages().contains(&p))
            }
            Self::FlatpakInstall { refs, .. } => {
                if refs.is_empty() {
                    return Err("no flatpaks".to_owned());
                }
                refs.iter().try_for_each(|r| check(flatpak_ref(r), "flatpak", r))
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

fn check(ok: bool, what: &str, value: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("invalid {what} `{}`", value.escape_debug())) }
}

fn word(s: &str, extra: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

fn flatpak_ref(r: &str) -> bool {
    const MIN_APP_ID_PARTS: usize = 3;
    let id = |id: &str| {
        let lower = id.to_ascii_lowercase();
        id.split('.').count() >= MIN_APP_ID_PARTS
            && id.split('.').all(|part| word(part, "_-") && !part.starts_with('-'))
            && !lower.ends_with(".flatpakref")
            && !lower.ends_with(".flatpak")
    };
    match r.split('/').collect::<Vec<_>>().as_slice() {
        [app] => id(app),
        ["app", app, arch, branch] => {
            id(app) && word(arch, "_") && word(branch, "._-") && !branch.starts_with(['-', '.'])
        }
        _ => false,
    }
}

fn check_user(user: &str) -> Result<(), String> {
    match crate::passwd::username_syntax_problem(user) {
        None => Ok(()),
        Some(problem) => {
            Err(format!("username `{}` can't be used: {problem:?}", user.escape_debug()))
        }
    }
}

fn check_fullname(fullname: &str) -> Result<(), String> {
    check(crate::passwd::sanitize_fullname(fullname) == fullname, "full name", fullname)
}

fn check_tweak_id(id: &str) -> Result<(), String> {
    check(word(id, "._-") && !id.starts_with(['.', '-']), "tweak", id)
}

fn check_cert_dir(name: &str) -> Result<(), String> {
    check(word(name, "-") && !name.starts_with('-'), "network", name)
}

fn check_attempt(attempt: &str) -> Result<(), String> {
    check(word(attempt, "") && attempt.chars().all(|c| c.is_ascii_digit()), "attempt", attempt)
}

fn check_pkgs(pkgs: &[String], allowed: impl Fn(&str) -> bool) -> Result<(), String> {
    if pkgs.is_empty() {
        return Err("no packages".to_owned());
    }
    pkgs.iter().try_for_each(|p| {
        check(word(p, "@_+.:-") && !p.starts_with(['-', '.', ':']), "package", p)?;
        if allowed(p) { Ok(()) } else { Err(format!("package `{p}` isn't one Taidan installs")) }
    })
}

fn allowed_extra(pkg: &str) -> bool {
    use crate::i18n::I18nImf;
    let langpack = ["ultramarine-langpacks-", "langpacks-", "glibc-langpack-"]
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
        || pkg == "fcitx5-chinese-addons"
}

#[must_use]
pub fn main() -> i32 {
    serve().unwrap_or_else(|e| {
        eprintln!("taidan-helper: {e}");
        2
    })
}

fn serve() -> Result<i32, String> {
    if uzers::get_effective_uid() != 0 {
        return Err("must be run as root, through pkexec".to_owned());
    }
    let name = std::env::args().nth(1).ok_or("usage: taidan-helper <verb> <request")?;
    let mut stdin = std::io::stdin().lock();
    let mut line = String::new();
    (&mut stdin)
        .take(MAX_REQUEST_BYTES)
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
    stdin
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut input)
        .map_err(|e| format!("cannot read the input: {e}"))?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err(format!("the input is over {MAX_INPUT_BYTES} bytes"));
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
    let setup_done = !crate::marked_unconfigured();
    if caller != 0 && setup_done && !verb.after_setup() {
        return Err(format!("the setup is done, `{name}` isn't taken anymore"));
    }
    verb.validate(caller == 0)?;
    let ctx = Ctx { caller, input, proxy, pass_proxy: verb.uses_network() };
    ctx.execute(&verb)
}

fn caller_uid() -> u32 {
    std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|uid| uid.parse().ok())
        .unwrap_or_else(uzers::get_current_uid)
}

struct Ctx {
    caller: u32,
    input: Vec<u8>,
    proxy: Option<crate::proxy::Proxy>,
    pass_proxy: bool,
}

fn io(what: &str) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |e| format!("{what}: {e}")
}

impl Ctx {
    fn cmd(&self, program: &str) -> Command {
        let mut cmd = Command::new(program);
        cmd.env_clear().env("PATH", SAFE_PATH).env("HOME", "/root").current_dir("/");
        for (key, value) in std::env::vars_os() {
            let k = key.as_encoded_bytes();
            if k == b"LANG" || k == b"LANGUAGE" || k.starts_with(b"LC_") {
                cmd.env(key, value);
            }
        }
        if let Some(proxy) = self.proxy.as_ref().filter(|_| self.pass_proxy) {
            cmd.envs(proxy.vars());
        }
        cmd
    }

    fn run(cmd: &mut Command, input: Option<&[u8]>) -> Result<i32, String> {
        let name = cmd.get_program().to_string_lossy().into_owned();
        cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
        let mut child = cmd.spawn().map_err(|e| format!("cannot run {name}: {e}"))?;
        if let Some(input) = input
            && let Some(mut stdin) = child.stdin.take()
        {
            _ = stdin.write_all(input);
        }
        let status = child.wait().map_err(|e| format!("cannot wait for {name}: {e}"))?;
        Ok(status.code().unwrap_or(1))
    }

    fn exec<S: AsRef<std::ffi::OsStr>>(&self, program: &str, args: &[S]) -> Result<i32, String> {
        Self::run(self.cmd(program).args(args), None)
    }

    fn input(&self, max: usize) -> Result<&[u8], String> {
        if self.input.len() > max {
            return Err(format!("the input is over {max} bytes"));
        }
        Ok(&self.input)
    }

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
                let created = std::fs::read_to_string(CREATED_USER_THIS_BOOT).ok();
                let live = [crate::passwd::recorded_user(), created.map(|u| u.trim().to_owned())]
                    .into_iter()
                    .flatten()
                    .find(|u| user_exists(u));
                if let Some(live) = live {
                    return Err(format!("`{live}` was made already, remove it first"));
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
                let icon = format!("/var/lib/AccountsService/icons/{user}");
                write_atomic(Path::new(&icon), &self.input, 0o644).map_err(io(&icon))?;
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
                // homectl reads the password from `$PASSWORD`
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
                if !is_real_dir(&dir) {
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
            Verb::Location { on } => {
                // a masked GeoClue would keep GNOME's location switch from turning it back on
                let gnome = Path::new("/usr/bin/gnome-shell").exists();
                let mask = !on && !gnome;
                let code =
                    self.exec("systemctl", &[if mask { "mask" } else { "unmask" }, GEOCLUE_UNIT])?;
                if code != 0 {
                    return Ok(code);
                }
                self.dconf_keyfile(LOCATION_OFF, (!on && gnome).then_some(LOCATION_OFF_KEYFILE))
            }
            Verb::GdmScreenReader { on } => {
                if !Path::new(GDM_DCONF_PROFILE).exists() {
                    return Ok(0);
                }
                self.dconf_keyfile(GDM_SCREEN_READER, on.then_some(GDM_SCREEN_READER_KEYFILE))
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
                crate::proxy::rpm_ostreed_as_root(self.proxy.as_ref())?;
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
            Verb::DnfOfflineArm => Self::run(
                self.cmd("dnf5")
                    .env("DNF_SYSTEM_UPGRADE_NO_REBOOT", "1")
                    .args(["offline", "reboot", "-y"]),
                None,
            ),
            // RPM Fusion's `%posttrans` kargs, except simpledrm stays since the LUKS prompt and a
            // failed driver need it
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

    fn dconf_keyfile(&self, path: &str, content: Option<&str>) -> Result<i32, String> {
        if !Path::new("/usr/bin/dconf").exists() {
            return Ok(0);
        }
        let file = Path::new(path);
        match content {
            Some(content) => write_atomic(file, content.as_bytes(), 0o644).map_err(io(path))?,
            None if std::fs::symlink_metadata(file).is_ok() => {
                remove_any(file).map_err(io(path))?;
            }
            None => return Ok(0),
        }
        self.exec("dconf", &["update"])
    }

    fn request_proxy(&self) -> Result<&crate::proxy::Proxy, String> {
        self.proxy.as_ref().ok_or_else(|| "the request has no proxy".to_owned())
    }

    fn start_portal(&self, socket: &str, lang: Option<&str>, uri: &str) -> Result<i32, String> {
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
        if let Some(proxy) = &self.proxy {
            ensure_run_dir().map_err(io(RUN_DIR))?;
            match write_portal_proxy_env(proxy) {
                Ok(file) => _ = cmd.arg(format!("--property=EnvironmentFile={}", file.display())),
                Err(e) => {
                    eprintln!("taidan-helper: cannot pass the proxy to the portal browser: {e}")
                }
            }
        }
        cmd.args(["--", PORTAL_BROWSER, "--platform", "gtk4", uri]);
        Self::run(&mut cmd, None)
    }

    fn kill_leftovers(&self, signal: &str) -> Result<i32, String> {
        // `pkexec` keeps the helper in the caller's cgroup
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
        let login_unknown = logins.is_none();
        let keep_home = keep_home || record.keep_home || login_unknown;
        let code = if crate::passwd::HOMED_UIDS.contains(&uid) {
            // homectl can't remove a user and keep its home
            if login_unknown {
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

    fn useradd(&self, user: &str, fullname: &str, uid: Option<u32>) -> Result<i32, String> {
        check_new_recorded(user)?;
        let hash = self.input_line()?;
        check_crypt_hash(&hash)?;
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
        self.exec("usermod", &["-L", "--", user])
    }

    fn check_caller_owned_dir(&self, dir: &Path) -> Result<(), String> {
        let meta = std::fs::symlink_metadata(dir).map_err(io(&dir.to_string_lossy()))?;
        let canonical = std::fs::canonicalize(dir).map_err(io(&dir.to_string_lossy()))?;
        if !meta.is_dir() || meta.uid() != self.caller || canonical != dir {
            return Err(format!("`{}` isn't a directory of the caller's", dir.display()));
        }
        Ok(())
    }

    fn homed_create(&self, user: &str, fullname: &str, skel: Option<&Path>) -> Result<i32, String> {
        check_new_recorded(user)?;
        let identity = sanitized_homed_identity(&self.input)?;
        let mut args: Vec<OsString> = vec![
            "create".into(),
            "--identity=-".into(),
            "--real-name".into(),
            fullname.into(),
            // homectl leaves out groups that don't exist
            "--member-of=wheel,video".into(),
            "--enforce-password-policy=no".into(),
        ];
        if let Some(skel) = skel {
            self.check_caller_owned_dir(skel)?;
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

// GLib's libproxy resolver reads `http_proxy` and friends, GNOME's would follow its own settings
fn write_portal_proxy_env(proxy: &crate::proxy::Proxy) -> std::io::Result<PathBuf> {
    let mut vars = String::new();
    for (key, value) in proxy.vars().into_iter().chain([("GIO_USE_PROXY_RESOLVER", "libproxy")]) {
        vars.extend([key, "=\"", &systemd_quote(value), "\"\n"]);
    }
    let file = Path::new(RUN_DIR).join("portal-proxy.env");
    write_atomic(&file, vars.as_bytes(), 0o600)?;
    Ok(file)
}

fn systemd_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

const DEBUG_SHELL_SCRIPT: &str = "echo 'Taidan has detected Alt+Shift+T. A terminal with superuser privilege is opened for debugging purposes. Only proceed if you know what you are doing, OTHERWISE YOU MAY RISK DATA LOSS OR CAUSE DAMAGES TO YOUR DEVICE. To exit the terminal, press Ctrl+D.' && sh";

#[must_use]
pub fn cmdline_has_debug_shell(cmdline: &str) -> bool {
    cmdline.split_ascii_whitespace().any(|arg| match arg.split_once('=') {
        None => arg == "taidan.debug_shell",
        Some((key, value)) => {
            key == "taidan.debug_shell" && matches!(value, "1" | "yes" | "y" | "true" | "on")
        }
    })
}

fn check_crypt_hash(hash: &str) -> Result<(), String> {
    if hash.starts_with('$')
        && hash.len() <= 512
        && hash.chars().all(|c| c.is_ascii_graphic() && c != ':')
    {
        Ok(())
    } else {
        Err("invalid password hash".to_owned())
    }
}

fn sanitized_homed_identity(input: &[u8]) -> Result<Vec<u8>, String> {
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
    check_crypt_hash(&id.privileged.hashed_password[0])?;
    let out = serde_json::json!({
        "secret": { "password": id.secret.password },
        "privileged": { "hashedPassword": id.privileged.hashed_password },
        "locked": id.locked,
    });
    serde_json::to_vec(&out).map_err(|e| e.to_string())
}

fn is_real_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

fn user_exists(user: &str) -> bool {
    uzers::get_user_by_name(user).is_some()
}

fn check_new_recorded(user: &str) -> Result<(), String> {
    if crate::passwd::recorded_user().as_deref() != Some(user) {
        return Err(format!("`{user}` isn't recorded, see record-created"));
    }
    if user_exists(user) {
        return Err(format!("`{user}` exists already"));
    }
    Ok(())
}

fn check_created(user: &str) -> Result<(), String> {
    let recorded = crate::passwd::recorded_user().as_deref() == Some(user);
    let created = std::fs::read_to_string(CREATED_USER_THIS_BOOT).is_ok_and(|u| u.trim() == user);
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

fn ensure_run_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(RUN_DIR)?;
    std::fs::set_permissions(RUN_DIR, std::fs::Permissions::from_mode(0o700))
}

fn write_created(user: &str) -> Result<(), String> {
    ensure_run_dir().map_err(io(RUN_DIR))?;
    write_atomic(Path::new(CREATED_USER_THIS_BOOT), format!("{user}\n").as_bytes(), 0o600)
        .map_err(io(CREATED_USER_THIS_BOOT))
}

fn remove_any(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
    }
}

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
    file.set_permissions(std::fs::Permissions::from_mode(mode))?;
    file.write_all(content)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)?;
    std::fs::File::open(dir)?.sync_all()
}

fn program() -> OsString {
    #[cfg(debug_assertions)]
    if let Some(helper) = std::env::var_os("TAIDAN_HELPER") {
        return helper;
    }
    PATH.into()
}

#[must_use]
pub fn stdin(verb: &Verb, input: &[u8]) -> Vec<u8> {
    stdin_with_proxy(verb, input, crate::proxy::get().as_ref())
}

#[must_use]
pub fn stdin_with_proxy(verb: &Verb, input: &[u8], proxy: Option<&crate::proxy::Proxy>) -> Vec<u8> {
    let request = Request { verb: verb.clone(), proxy: proxy.cloned() };
    let mut out = serde_json::to_vec(&request).unwrap_or_default();
    out.push(b'\n');
    out.extend_from_slice(input);
    out
}

#[must_use]
pub fn command(verb: &Verb) -> tokio::process::Command {
    std_command(verb).into()
}

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
            Verb::Location { on: false },
            Verb::GdmScreenReader { on: true },
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
        // serde ignores extra fields on unit variants only
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
        assert!(flatpak("app/org.gimp.GIMP/aarch64/beta").is_ok());
        assert!(flatpak("io.github.some_dev.App-Name").is_ok());
        assert!(flatpak("--from=https://evil/x.flatpakref").is_err());
        assert!(flatpak("/tmp/x.flatpak").is_err());
        for bad in [
            "evil.app.flatpakref",
            "org.evil.App.flatpak",
            "org.evil.App.FLATPAKREF",
            "tmp/x.flatpakref",
            "x.flatpak",
            "./org.evil.App",
            "../org.evil.App",
            "org.evil.App/../x",
            "runtime/org.gnome.Platform/x86_64/49",
            "app/org.gimp.GIMP/x86_64",
            "app/org.gimp.GIMP/x86_64/../x",
            "org.mozilla",
            "org..firefox",
            "org.mozilla.-firefox",
        ] {
            assert!(flatpak(bad).is_err(), "{bad}");
        }
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
        assert!(check_crypt_hash("$y$j9T$abc$def/.").is_ok());
        assert!(check_crypt_hash("plain").is_err());
        assert!(check_crypt_hash("$y$a:b").is_err());
        assert!(check_crypt_hash("$y$a\nroot:x").is_err());
        let identity = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"]},"locked":true}"#;
        let out: serde_json::Value =
            serde_json::from_slice(&sanitized_homed_identity(identity).unwrap()).unwrap();
        assert_eq!(out["locked"], true);
        assert_eq!(out["privileged"]["hashedPassword"][0], "$y$x");
        let extra = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"]},"locked":false,"uid":0}"#;
        assert!(sanitized_homed_identity(extra).is_err());
        let groups = br#"{"secret":{"password":["pw"]},"privileged":{"hashedPassword":["$y$x"],"memberOf":["root"]},"locked":false}"#;
        assert!(sanitized_homed_identity(groups).is_err());
        let two = br#"{"secret":{"password":["a","b"]},"privileged":{"hashedPassword":["$y$x"]},"locked":false}"#;
        assert!(sanitized_homed_identity(two).is_err());
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
                "gdm-screen-reader",
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
