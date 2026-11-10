//! accessibility options, as the KDE config entries that turn them on in KWin and Plasma, and the
//! `gsettings` or `xfconf-query` commands for the new user on GNOME, Budgie and Xfce
//!
//! the OOBE writes the KDE ones for its own KWin session as they change, and the new user gets
//! whichever their desktop reads during setup, see [`apply_to_user`]
use crate::prelude::*;

/// what the user turned on in the OOBE
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct A11y {
    /// Orca, which Plasma's kaccess starts at login
    pub screen_reader: bool,
    /// KWin's zoom effect, at [`ZOOM`]
    pub zoom: bool,
    pub sticky_keys: bool,
    pub slow_keys: bool,
    pub bounce_keys: bool,
    /// KWin's dwell clicker (6.8+)
    pub dwell_click: bool,
    pub larger_text: bool,
    /// Atkinson Hyperlegible Next instead of the default font
    pub legible_font: bool,
    pub reduce_motion: bool,
    /// 0 for none, otherwise KWin's colorblindness correction mode + 1: protanopia,
    /// deuteranopia, tritanopia, monochrome
    pub color_filter: u8,
}

/// how far zoom is turned on, about what four presses of Meta+= give
pub const ZOOM: f64 = 2.0;

/// Plasma's built-in fonts, for when the system's kdeglobals doesn't set them
const FONT_SIZE: u8 = 10;
const SANS: &str = "Noto Sans";
const MONO: &str = "Hack";

/// how many points larger text makes the fonts
const LARGER_TEXT_STEP: u8 = 2;

/// the system's kdeglobals, most important first, as Plasma's session on Fedora sets
/// `XDG_CONFIG_DIRS`
const SYSTEM_KDEGLOBALS: [&str; 2] =
    ["/etc/xdg/kdeglobals", "/usr/share/kde-settings/kde-profile/default/xdg/kdeglobals"];

const LEGIBLE_SANS: &str = "Atkinson Hyperlegible Next";
const LEGIBLE_MONO: &str = "Atkinson Hyperlegible Mono";

/// GNOME's default font size, and the text scale its own Large Text setting uses
const GNOME_FONT_SIZE: u8 = 11;
const GNOME_LARGER_TEXT: &str = "1.25";

/// Xfce's default font size, and the DPI that makes text about as much larger as on GNOME
const XFCE_FONT_SIZE: u8 = 10;
const XFCE_LARGER_DPI: &str = "120";

/// an entry in a KDE config file, `None` deletes it so the default applies
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub file: &'static str,
    pub group: &'static str,
    pub key: &'static str,
    pub value: Option<String>,
    /// written as a bool, so KConfig reads `true`/`false`
    pub bool: bool,
}

impl Entry {
    fn bool(file: &'static str, group: &'static str, key: &'static str, on: bool) -> Self {
        Self { file, group, key, value: Some(on.to_string()), bool: true }
    }

    fn value(
        file: &'static str,
        group: &'static str,
        key: &'static str,
        value: Option<String>,
    ) -> Self {
        Self { file, group, key, value, bool: false }
    }

    /// the arguments to `kwriteconfig6` that write this, after `--file`
    #[must_use]
    pub fn kwriteconfig_args(&self, file: &str) -> Vec<String> {
        let mut args = vec![
            "--file".to_owned(),
            file.to_owned(),
            "--group".to_owned(),
            self.group.to_owned(),
            "--key".to_owned(),
            self.key.to_owned(),
        ];
        match &self.value {
            Some(v) => {
                if self.bool {
                    args.extend(["--type".to_owned(), "bool".to_owned()]);
                }
                args.push(v.clone());
            }
            None => args.push("--delete".to_owned()),
        }
        args
    }
}

/// `family` at `size` in Qt's font string format, as in kdeglobals
fn font(family: &str, size: u8) -> String {
    format!("{family},{size},-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
}

/// the fonts a new Plasma user starts with, which larger text only makes bigger
#[derive(Clone, Debug, PartialEq, Eq)]
struct SystemFonts {
    sans: String,
    mono: String,
    /// in points
    size: u8,
}

impl Default for SystemFonts {
    fn default() -> Self {
        Self { sans: SANS.to_owned(), mono: MONO.to_owned(), size: FONT_SIZE }
    }
}

impl SystemFonts {
    /// from the system's kdeglobals
    fn read() -> Self {
        let files = SYSTEM_KDEGLOBALS.iter().filter_map(|p| std::fs::read_to_string(p).ok());
        Self::from_kdeglobals(&files.collect_vec())
    }

    /// from the contents of kdeglobals files, most important first. Like KConfig, each key comes
    /// from the first file that has it
    fn from_kdeglobals(files: &[String]) -> Self {
        let get = |key: &str| {
            files.iter().find_map(|f| kconfig_value(f, "General", key)).and_then(parse_font)
        };
        let default = Self::default();
        let sans = get("font");
        let mono = get("fixed");
        Self {
            size: sans.as_ref().and_then(|f| f.1).unwrap_or(default.size),
            sans: sans.map_or(default.sans, |f| f.0),
            mono: mono.map_or(default.mono, |f| f.0),
        }
    }
}

/// the value of `key` in `[group]` of a KDE config file, ignoring flags like `[$i]` after the key
fn kconfig_value<'a>(file: &'a str, group: &str, key: &str) -> Option<&'a str> {
    let mut in_group = false;
    for line in file.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            // `[$i]` alone on a line marks the whole file immutable, it's not a group
            if !name.starts_with('$') {
                in_group = name == group;
            }
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        let k = k.split_once("[$").map_or(k, |(k, _)| k);
        if k == key {
            return Some(v.trim());
        }
    }
    None
}

/// the family and the point size, if it has one, of a font in Qt's font string format
fn parse_font(font: &str) -> Option<(String, Option<u8>)> {
    let mut fields = font.split(',');
    let family = fields.next()?.trim();
    if family.is_empty() {
        return None;
    }
    // -1 when the font has a pixel size instead, and it may have a fraction, which is dropped
    let size = fields.next().map(str::trim).and_then(|s| {
        s.split_once('.').map_or(s, |(whole, _)| whole).parse::<u8>().ok().filter(|&s| s > 0)
    });
    Some((family.to_owned(), size))
}

impl A11y {
    /// entries KWin reads while running, so the OOBE session can apply them right away
    #[must_use]
    pub fn session_entries(&self) -> Vec<Entry> {
        vec![
            Entry::bool("kaccessrc", "Keyboard", "StickyKeys", self.sticky_keys),
            Entry::bool("kaccessrc", "Keyboard", "SlowKeys", self.slow_keys),
            Entry::bool("kaccessrc", "Keyboard", "BounceKeys", self.bounce_keys),
            Entry::value(
                "kdeglobals",
                "KDE",
                "AnimationDurationFactor",
                self.reduce_motion.then(|| "0".to_owned()),
            ),
            Entry::bool(
                "kwinrc",
                "Plugins",
                "colorblindnesscorrectionEnabled",
                self.color_filter > 0,
            ),
            Entry::value(
                "kwinrc",
                "Effect-colorblindnesscorrection",
                "Mode",
                (self.color_filter > 0).then(|| (self.color_filter - 1).to_string()),
            ),
            Entry::bool("kwinrc", "Plugins", "dwellclickerEnabled", self.dwell_click),
        ]
    }

    /// every entry [`Self::user_entries`] can write, deleted so the defaults apply, for undoing
    /// options that were carried over before they were turned off
    fn reset_entries() -> Vec<Entry> {
        let all = Self {
            screen_reader: true,
            zoom: true,
            sticky_keys: true,
            slow_keys: true,
            bounce_keys: true,
            dwell_click: true,
            larger_text: true,
            legible_font: true,
            reduce_motion: true,
            color_filter: 1,
        };
        all.user_entries().into_iter().map(|e| Entry { value: None, bool: false, ..e }).collect()
    }

    /// everything to write for the new user, nothing when no option is on so their config stays
    /// untouched
    #[must_use]
    pub fn user_entries(&self) -> Vec<Entry> {
        self.user_entries_with(&SystemFonts::read())
    }

    /// [`Self::user_entries`], with larger text and the legible font based on `system`
    fn user_entries_with(&self, system: &SystemFonts) -> Vec<Entry> {
        if *self == Self::default() {
            return vec![];
        }
        let mut entries = self.session_entries();
        entries.push(Entry::bool("kaccessrc", "ScreenReader", "Enabled", self.screen_reader));
        entries.push(Entry::value(
            "kwinrc",
            "Effect-zoom",
            "InitialZoom",
            self.zoom.then(|| ZOOM.to_string()),
        ));
        if self.larger_text || self.legible_font {
            let size = if self.larger_text {
                system.size.saturating_add(LARGER_TEXT_STEP)
            } else {
                system.size
            };
            let (sans, mono) = if self.legible_font {
                (LEGIBLE_SANS, LEGIBLE_MONO)
            } else {
                (system.sans.as_str(), system.mono.as_str())
            };
            for key in ["font", "menuFont", "toolBarFont"] {
                entries.push(Entry::value("kdeglobals", "General", key, Some(font(sans, size))));
            }
            entries.push(Entry::value(
                "kdeglobals",
                "General",
                "smallestReadableFont",
                Some(font(sans, size.saturating_sub(2).max(1))),
            ));
            entries.push(Entry::value("kdeglobals", "General", "fixed", Some(font(mono, size))));
            entries.push(Entry::value("kdeglobals", "WM", "activeFont", Some(font(sans, size))));
        }
        entries
    }
}

/// whether the installed system's desktop is Plasma
pub(crate) fn is_plasma() -> bool {
    std::path::Path::new("/usr/bin/plasmashell").exists()
}

const GSETTINGS: &str = "/usr/bin/gsettings";
const XFCONF: &str = "/usr/bin/xfconf-query";

/// `gsettings set`, or `gsettings reset` for [`None`], through a D-Bus of its own since dconf
/// writes through its service
pub(crate) fn gsettings(schema: &str, key: &str, value: Option<&str>) -> Vec<String> {
    let mut cmd = vec!["dbus-run-session", "--", "gsettings"];
    match value {
        Some(value) => cmd.extend(["set", schema, key, value]),
        None => cmd.extend(["reset", schema, key]),
    }
    cmd.into_iter().map(str::to_owned).collect()
}

/// `xfconf-query` setting a `ty` property, or resetting it for [`None`], through a D-Bus of its
/// own like [`gsettings`]. `-n` creates the property in case the defaults don't have it
pub(crate) fn xfconf(channel: &str, property: &str, ty: &str, value: Option<&str>) -> Vec<String> {
    let mut cmd = vec!["dbus-run-session", "--", "xfconf-query", "-c", channel, "-p", property];
    match value {
        Some(value) => cmd.extend(["-n", "-t", ty, "-s", value]),
        None => cmd.push("-r"),
    }
    cmd.into_iter().map(str::to_owned).collect()
}

/// runs the next `$1` arguments as a command, then the next after that, and so on, failing if any
/// of them do. `eval` only ever sees `"${1}" "${2}"…`, never the arguments themselves
const RUN_ALL: &str = r#"
status=0
while [ $# -gt 0 ]; do
    n=$1
    shift
    c=
    i=1
    while [ "$i" -le "$n" ]; do
        c="$c \"\${$i}\""
        i=$((i + 1))
    done
    eval "$c" || status=1
    shift "$n"
done
if [ -n "$TAIDAN_XFCONF_WAIT" ]; then sleep "$TAIDAN_XFCONF_WAIT"; fi
exit "$status"
"#;

/// how long xfconfd waits before saving a change, it saves sooner when its D-Bus goes away, but
/// `dbus-run-session` doesn't wait for that, and the next xfconfd could read the old file
const XFCONFD_SAVE_DELAY: &str = "6";

/// [`gsettings`] and [`xfconf`] commands run one after the other through a single D-Bus, so one
/// dconf or xfconf daemon has them all, rather than one for each that may not have saved before
/// the next reads its files. Other commands run as they are. [`None`] if there are none
pub(crate) fn one_session(cmds: Vec<Vec<String>>) -> Option<Vec<String>> {
    if cmds.is_empty() {
        return None;
    }
    let mut xfconf = false;
    let mut bus = false;
    let mut args = vec![];
    for cmd in cmds {
        let cmd = match cmd.as_slice() {
            [run, dashes, rest @ ..] if run == "dbus-run-session" && dashes == "--" => {
                bus = true;
                rest.to_vec()
            }
            _ => cmd,
        };
        xfconf |= cmd.first().is_some_and(|c| c == "xfconf-query");
        // through `env`, for the variables some start with
        args.push((cmd.len() + 1).to_string());
        args.push("env".to_owned());
        args.extend(cmd);
    }
    // a D-Bus only if one of them wanted it, it may not be installed otherwise
    let mut out = if bus { vec!["dbus-run-session".to_owned(), "--".to_owned()] } else { vec![] };
    out.push("env".to_owned());
    out.push(format!("TAIDAN_XFCONF_WAIT={}", if xfconf { XFCONFD_SAVE_DELAY } else { "" }));
    out.extend(["sh", "-c", RUN_ALL, "sh"].map(str::to_owned));
    out.extend(args);
    Some(out)
}

/// `env` arguments running `cmd` with `home` as the home directory
pub(crate) fn in_home(home: &std::path::Path, cmd: Vec<String>) -> Vec<String> {
    let mut args = vec![
        // not the OOBE session's bus, or its KWin and apps would hear about the change
        "-u".to_owned(),
        "DBUS_SESSION_BUS_ADDRESS".to_owned(),
        format!("HOME={}", home.display()),
        format!("XDG_CONFIG_HOME={}", home.join(".config").display()),
    ];
    args.extend(cmd);
    args
}

/// starts Orca at login on desktops other than GNOME, whose shell starts it from
/// `screen-reader-enabled`. Orca's own autostart entry is only for GNOME and a few others, so this
/// one in the user's config takes its place
const ORCA_AUTOSTART: &str = "[Desktop Entry]
Type=Application
Name=Orca Screen Reader
Exec=orca --replace
NoDisplay=true
NotShowIn=GNOME;KDE;
X-GNOME-AutoRestart=true
";

/// where [`ORCA_AUTOSTART`] goes in the home, the name of Orca's own entry so it replaces it
const ORCA_AUTOSTART_PATH: &str = ".config/autostart/orca-autostart.desktop";

impl A11y {
    /// the commands that write the options into the config under `HOME` on a desktop other than
    /// Plasma, `xfconf-query` on Xfce and `gsettings` elsewhere. Zoom, dwell click and the color
    /// filter have nothing to go to on Xfce, and GNOME has no color filter
    #[must_use]
    pub fn desktop_commands(&self, edition: &str) -> Vec<Vec<String>> {
        if *self == Self::default() {
            return vec![];
        }
        self.all_desktop_commands(edition)
    }

    /// [`Self::desktop_commands`], even when no option is on, which resets them all
    fn all_desktop_commands(&self, edition: &str) -> Vec<Vec<String>> {
        if crate::theme::uses_xfconf(edition) {
            return self.xfconf_commands();
        }
        let flag = |on: bool| Some(if on { "true" } else { "false" });
        let keyboard = "org.gnome.desktop.a11y.keyboard";
        let apps = "org.gnome.desktop.a11y.applications";
        let interface = "org.gnome.desktop.interface";
        let mag = ZOOM.to_string();
        let font = |family: &str| format!("'{family} {GNOME_FONT_SIZE}'");
        let (sans, mono) = (font(LEGIBLE_SANS), font(LEGIBLE_MONO));
        vec![
            gsettings(apps, "screen-reader-enabled", flag(self.screen_reader)),
            // for Budgie, whose apps only talk to Orca with this on
            gsettings(interface, "toolkit-accessibility", flag(self.screen_reader)),
            gsettings(apps, "screen-magnifier-enabled", flag(self.zoom)),
            gsettings(
                "org.gnome.desktop.a11y.magnifier",
                "mag-factor",
                self.zoom.then_some(mag.as_str()),
            ),
            gsettings(keyboard, "stickykeys-enable", flag(self.sticky_keys)),
            gsettings(keyboard, "slowkeys-enable", flag(self.slow_keys)),
            gsettings(keyboard, "bouncekeys-enable", flag(self.bounce_keys)),
            gsettings(
                "org.gnome.desktop.a11y.mouse",
                "dwell-click-enabled",
                flag(self.dwell_click),
            ),
            gsettings(
                interface,
                "text-scaling-factor",
                self.larger_text.then_some(GNOME_LARGER_TEXT),
            ),
            gsettings(interface, "font-name", self.legible_font.then_some(sans.as_str())),
            gsettings(interface, "document-font-name", self.legible_font.then_some(sans.as_str())),
            gsettings(interface, "monospace-font-name", self.legible_font.then_some(mono.as_str())),
            gsettings(interface, "enable-animations", flag(!self.reduce_motion)),
            gsettings(
                "org.gnome.desktop.a11y.interface",
                "reduced-motion",
                self.reduce_motion.then_some("'reduce'"),
            ),
        ]
    }

    fn xfconf_commands(&self) -> Vec<Vec<String>> {
        let flag = |on: bool| Some(if on { "true" } else { "false" });
        let font = |family: &str| format!("{family} {XFCE_FONT_SIZE}");
        let (sans, mono) = (font(LEGIBLE_SANS), font(LEGIBLE_MONO));
        let title = format!("{LEGIBLE_SANS} Bold {}", XFCE_FONT_SIZE - 1);
        vec![
            // starts the AT-SPI bus Orca talks to apps over
            xfconf(
                "xfce4-session",
                "/general/StartAssistiveTechnologies",
                "bool",
                flag(self.screen_reader),
            ),
            xfconf("accessibility", "/StickyKeys", "bool", flag(self.sticky_keys)),
            xfconf("accessibility", "/SlowKeys", "bool", flag(self.slow_keys)),
            xfconf("accessibility", "/BounceKeys", "bool", flag(self.bounce_keys)),
            xfconf("xsettings", "/Xft/DPI", "int", self.larger_text.then_some(XFCE_LARGER_DPI)),
            xfconf(
                "xsettings",
                "/Gtk/FontName",
                "string",
                self.legible_font.then_some(sans.as_str()),
            ),
            xfconf(
                "xsettings",
                "/Gtk/MonospaceFontName",
                "string",
                self.legible_font.then_some(mono.as_str()),
            ),
            xfconf(
                "xfwm4",
                "/general/title_font",
                "string",
                self.legible_font.then_some(title.as_str()),
            ),
            xfconf("xsettings", "/Gtk/EnableAnimations", "bool", flag(!self.reduce_motion)),
        ]
    }

    /// whether the desktop needs [`ORCA_AUTOSTART`] to start the screen reader
    fn needs_orca_autostart(&self, edition: &str) -> bool {
        self.screen_reader && crate::cfg::desktop(edition) != "gnome"
    }
}

/// [`A11y::desktop_commands`] if there's a tool on this system to run them, all of them with
/// `reset` to undo options that were turned off
fn desktop_commands(
    settings: &crate::settings::Settings,
    edition: &str,
    reset: bool,
) -> Vec<Vec<String>> {
    let tool = if crate::theme::uses_xfconf(edition) { XFCONF } else { GSETTINGS };
    if !std::path::Path::new(tool).exists() {
        tracing::info!(tool, "not found; skipping accessibility settings");
        return vec![];
    }
    if reset {
        settings.a11y.all_desktop_commands(edition)
    } else {
        settings.a11y.desktop_commands(edition)
    }
}

/// writes the options into the new user's config, so their session starts with them
///
/// on systemd-homed the home isn't mounted after it's created, so there they go in the skeleton
/// it's created from instead, see [`write_to_skel`]
///
/// # Errors
/// - cannot find the user's home directory
pub async fn apply_to_user(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Res<()> {
    // off too, for a retry after an attempt with it on
    apply_to_greeter(settings.a11y.screen_reader).await;
    if cfg.taidan0.systemd_homed || settings.a11y == A11y::default() {
        return Ok(());
    }
    write_to_home(settings, cfg, None).await
}

/// [`apply_to_user`] again if the options changed since `applied` were, e.g. on the installing
/// page, undoing the ones turned off since
///
/// # Errors
/// - cannot find the user's home directory
pub async fn reapply_to_user(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    applied: A11y,
) -> Res<()> {
    if settings.a11y == applied {
        return Ok(());
    }
    if settings.a11y.screen_reader != applied.screen_reader {
        apply_to_greeter(settings.a11y.screen_reader).await;
    }
    write_to_home(settings, cfg, Some(applied)).await
}

/// starts the screen reader on the login screen too, or stops it again, on GDM only. Plasma Login
/// Manager and SDDM have no setting for it, and slick-greeter keeps its own in the `lightdm`
/// user's dconf
async fn apply_to_greeter(on: bool) {
    if !std::path::Path::new(crate::helper::GDM_DCONF_PROFILE).exists() {
        return;
    }
    if let Err(e) = crate::root(crate::Verb::GdmScreenReader { on }).await {
        tracing::warn!(?e, on, "cannot set the screen reader on the login screen");
    }
}

/// a command to run as the user in their home, with what to feed its stdin
pub(crate) type HomeCmd = (Vec<String>, Option<String>);

/// runs `cmds` as the user, each failure logged. On systemd-homed, the home isn't mounted once
/// it's created, so they all run in one `homectl with`, which opens it with the user's password.
/// That only works once the user is unlocked, see `steps::unlock_user`
pub(crate) async fn run_in_home(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    cmds: Vec<HomeCmd>,
) {
    use secrecy::ExposeSecret as _;
    let user = settings.username.as_str();
    if !cfg.taidan0.systemd_homed {
        for (cmd, input) in cmds {
            let res = crate::as_user(user, cmd.clone(), input.as_deref().map(str::as_bytes)).await;
            if let Err(e) = res {
                tracing::warn!(?e, ?cmd, "cannot write into the user's home");
            }
        }
        return;
    }
    if cmds.is_empty() {
        return;
    }
    let script = homed_script(&cmds);
    // homectl takes the password from `$PASSWORD`, which the helper sets from stdin since anyone
    // can read a command's arguments, and keeps from what runs in the home
    let argv = ["sh", "-c", &script].map(str::to_owned).to_vec();
    let res = crate::root_with_stdin(
        crate::Verb::AsUserHomed { user: user.to_owned(), argv },
        format!("{}\n", settings.passwd.expose_secret()).as_bytes(),
    )
    .await;
    if let Err(e) = res {
        tracing::warn!(?e, "cannot write into the systemd-homed home");
    }
}

/// `cmds` as one `sh` script that runs them all and fails if any did
fn homed_script(cmds: &[HomeCmd]) -> String {
    use crate::proxy::sh_quote;
    let mut script = "rc=0\n".to_owned();
    for (cmd, input) in cmds {
        let cmd = cmd.iter().map(|a| sh_quote(a)).join(" ");
        match input {
            Some(input) => script += &format!("printf %s {} | {cmd} || rc=1\n", sh_quote(input)),
            None => script += &format!("{cmd} || rc=1\n"),
        }
    }
    script + "exit $rc\n"
}

/// writes the options into the user's home, undoing the ones in `applied` that are off now
async fn write_to_home(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    applied: Option<A11y>,
) -> Res<()> {
    let home = home_of(&settings.username).await?;
    let mut cmds: Vec<HomeCmd> = vec![];
    if is_plasma() {
        let config = home.join(".config");
        let mut entries = if applied.is_some() { A11y::reset_entries() } else { vec![] };
        entries.extend(settings.a11y.user_entries());
        for entry in entries {
            let mut cmd = vec!["kwriteconfig6".to_owned()];
            cmd.extend(entry.kwriteconfig_args(&config.join(entry.file).to_string_lossy()));
            cmds.push((cmd, None));
        }
        run_in_home(settings, cfg, cmds).await;
        return Ok(());
    }
    if let Some(cmd) = one_session(desktop_commands(settings, &cfg.edition, applied.is_some())) {
        let mut env = vec!["env".to_owned()];
        env.extend(in_home(&home, cmd));
        cmds.push((env, None));
    }
    if settings.a11y.needs_orca_autostart(&cfg.edition) {
        let path = home.join(ORCA_AUTOSTART_PATH);
        let dir = path.parent().unwrap_or(&home).to_string_lossy().into_owned();
        let path = path.to_string_lossy().into_owned();
        cmds.push((vec!["mkdir".to_owned(), "-p".to_owned(), dir], None));
        // quiet, it would only echo the file into the log
        cmds.push((
            vec![
                "sh".to_owned(),
                "-c".to_owned(),
                r#"exec cat > "$1""#.to_owned(),
                "sh".to_owned(),
                path,
            ],
            Some(ORCA_AUTOSTART.to_owned()),
        ));
    } else if applied.is_some_and(|a| a.needs_orca_autostart(&cfg.edition)) {
        let path = home.join(ORCA_AUTOSTART_PATH).to_string_lossy().into_owned();
        cmds.push((vec!["rm".to_owned(), "-f".to_owned(), path], None));
    }
    run_in_home(settings, cfg, cmds).await;
    Ok(())
}

/// writes the options into `skel`, a copy of `/etc/skel` the new user's home is made from,
/// returns whether there was anything to write
///
/// # Errors
/// - cannot create `skel/.config`
pub async fn write_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &std::path::Path,
) -> Res<bool> {
    if settings.a11y == A11y::default() {
        return Ok(false);
    }
    let config = skel.join(".config");
    tokio::fs::create_dir_all(&config).await.map_err(Err::Io)?;
    if is_plasma() {
        for entry in settings.a11y.user_entries() {
            let args = entry.kwriteconfig_args(&config.join(entry.file).to_string_lossy());
            let args = args.iter().map(String::as_str).collect_vec();
            if let Err(e) = crate::steps::acmd("kwriteconfig6", &args).await {
                tracing::warn!(?e, ?entry, "cannot write accessibility setting");
            }
        }
        return Ok(true);
    }
    if let Some(cmd) = one_session(desktop_commands(settings, &cfg.edition, false)) {
        let env = in_home(skel, cmd);
        let env = env.iter().map(String::as_str).collect_vec();
        if let Err(e) = crate::steps::acmd("env", &env).await {
            tracing::warn!(?e, "cannot write accessibility setting");
        }
    }
    if settings.a11y.needs_orca_autostart(&cfg.edition) {
        let path = skel.join(ORCA_AUTOSTART_PATH);
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(Err::Io)?;
        }
        tokio::fs::write(&path, ORCA_AUTOSTART).await.map_err(Err::Io)?;
    }
    Ok(true)
}

/// the home directory of `user`, from the passwd database
pub(crate) async fn home_of(user: &str) -> Res<std::path::PathBuf> {
    let out = tokio::process::Command::new("getent")
        .args(["passwd", user])
        .output()
        .await
        .map_err(|e| Err::FailToRunProgram("getent", e))?;
    // name:password:uid:gid:gecos:home:shell
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(':')
        .nth(5)
        .filter(|h| !h.is_empty())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| Err::ProgramFail {
            name: "getent",
            output: format!("no home directory for {user}"),
            rc: out.status.code(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homed_script_quotes_and_feeds_stdin() {
        let cmds: Vec<HomeCmd> = vec![
            (vec!["echo".to_owned(), "it's; rm -rf /".to_owned()], None),
            (vec!["cat".to_owned()], Some("a'b\n".to_owned())),
            (vec!["false".to_owned()], None),
        ];
        let out =
            std::process::Command::new("sh").args(["-c", &homed_script(&cmds)]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout), "it's; rm -rf /\na'b\n");
        assert!(!out.status.success());
    }

    #[test]
    fn nothing_on_writes_nothing() {
        assert!(A11y::default().user_entries().is_empty());
    }

    #[test]
    fn reset_deletes_every_entry() {
        let reset = A11y::reset_entries();
        assert!(reset.iter().all(|e| e.value.is_none()));
        let fonts = A11y { larger_text: true, ..A11y::default() }.user_entries();
        for entry in fonts {
            assert!(
                reset
                    .iter()
                    .any(|r| (r.file, r.group, r.key) == (entry.file, entry.group, entry.key))
            );
        }
    }

    #[test]
    fn color_filter_mode() {
        let a11y = A11y { color_filter: 4, ..A11y::default() };
        let entries = a11y.session_entries();
        let mode = entries.iter().find(|e| e.key == "Mode").unwrap();
        assert_eq!(mode.value.as_deref(), Some("3"));
        let enabled = entries.iter().find(|e| e.key == "colorblindnesscorrectionEnabled").unwrap();
        assert_eq!(enabled.value.as_deref(), Some("true"));
    }

    #[test]
    fn fonts_only_when_asked() {
        let zoom = A11y { zoom: true, ..A11y::default() }.user_entries();
        assert!(!zoom.iter().any(|e| e.key == "font"));
        assert!(zoom.iter().any(|e| e.key == "InitialZoom" && e.value.as_deref() == Some("2")));

        let legible = A11y { legible_font: true, larger_text: true, ..A11y::default() }
            .user_entries_with(&SystemFonts::default());
        let font = legible.iter().find(|e| e.key == "font").unwrap();
        assert_eq!(
            font.value.as_deref(),
            Some("Atkinson Hyperlegible Next,12,-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
        );
        let fixed = legible.iter().find(|e| e.key == "fixed").unwrap();
        assert!(fixed.value.as_deref().unwrap().starts_with("Atkinson Hyperlegible Mono,12,"));
    }

    #[test]
    fn larger_text_keeps_the_system_fonts() {
        let etc = "[$i]\n[General]\nfont[$e]=Inter,11,-1,5,400,0,0,0,0,0,0,0,0,0,0,1\n".to_owned();
        let profile = "[General]\nfixed=Noto Sans Mono,10,-1,5,50,0,0,0,0,0\n\
                       font=Noto Sans,10,-1,5,50,0,0,0,0,0\n[WM]\nfixed=Wrong,10\n"
            .to_owned();
        let system = SystemFonts::from_kdeglobals(&[etc, profile]);
        assert_eq!(
            system,
            SystemFonts { sans: "Inter".to_owned(), mono: "Noto Sans Mono".to_owned(), size: 11 }
        );
        let larger = A11y { larger_text: true, ..A11y::default() }.user_entries_with(&system);
        let value = |key: &str| larger.iter().find(|e| e.key == key).unwrap().value.clone();
        assert!(value("font").unwrap().starts_with("Inter,13,"));
        assert!(value("fixed").unwrap().starts_with("Noto Sans Mono,13,"));
        assert!(value("smallestReadableFont").unwrap().starts_with("Inter,11,"));

        assert_eq!(SystemFonts::from_kdeglobals(&[]), SystemFonts::default());
        // a pixel size keeps the default point size
        let px = SystemFonts::from_kdeglobals(&["[General]\nfont=Cantarell,-1,14,5\n".to_owned()]);
        assert_eq!((px.sans.as_str(), px.size), ("Cantarell", FONT_SIZE));
    }

    #[test]
    fn one_session_runs_them_all() {
        let cmd = |args: &[&str]| args.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
        let dir = std::env::temp_dir().join(format!("taidan-one-session-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out");
        let out_s = out.to_string_lossy().into_owned();
        let all = one_session(vec![
            cmd(&[
                "dbus-run-session",
                "--",
                "sh",
                "-c",
                "printf '%s|' \"$@\" >> \"$0\"",
                &out_s,
                "a b",
                "$(c)",
                "'d'",
            ]),
            cmd(&["X=1", "sh", "-c", "printf '%s' \"$X\" >> \"$0\"", &out_s]),
        ])
        .unwrap();
        // without the D-Bus around it, which the test machine may not have
        assert_eq!(all[..3], ["dbus-run-session", "--", "env"]);
        let status = std::process::Command::new(&all[2]).args(&all[3..]).status().unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "a b|$(c)|'d'|1");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(one_session(vec![]).is_none());
    }

    #[test]
    fn desktop_commands() {
        assert!(A11y::default().desktop_commands("gnome").is_empty());

        let a11y = A11y { sticky_keys: true, legible_font: true, ..A11y::default() };
        let gnome = a11y.desktop_commands("atomic-gnome");
        assert!(gnome.iter().all(|c| c[2] == "gsettings"));
        assert!(gnome.iter().any(|c| c.ends_with(&["stickykeys-enable".into(), "true".into()])));
        assert!(gnome.iter().any(|c| c.last().unwrap() == "'Atkinson Hyperlegible Next 11'"));
        // off ones go back to the defaults, for a retry after an attempt with them on
        assert!(
            gnome.iter().any(|c| c[3] == "reset" && c.last().unwrap() == "text-scaling-factor")
        );

        let xfce = a11y.desktop_commands("xfce");
        assert!(xfce.iter().all(|c| c[2] == "xfconf-query"));
        let sticky = xfce.iter().find(|c| c.contains(&"/StickyKeys".to_owned())).unwrap();
        assert_eq!(sticky[7..], ["-n", "-t", "bool", "-s", "true"]);
        let dpi = xfce.iter().find(|c| c.contains(&"/Xft/DPI".to_owned())).unwrap();
        assert_eq!(dpi.last().unwrap(), "-r");
    }

    #[test]
    fn orca_autostart_outside_gnome() {
        let a11y = A11y { screen_reader: true, ..A11y::default() };
        assert!(!a11y.needs_orca_autostart("gnome"));
        assert!(a11y.needs_orca_autostart("budgie") && a11y.needs_orca_autostart("xfce"));
        assert!(!A11y::default().needs_orca_autostart("xfce"));
    }

    #[test]
    fn kwriteconfig_args() {
        let on = Entry::bool("kaccessrc", "Keyboard", "StickyKeys", true);
        assert_eq!(
            on.kwriteconfig_args("/home/u/.config/kaccessrc"),
            [
                "--file",
                "/home/u/.config/kaccessrc",
                "--group",
                "Keyboard",
                "--key",
                "StickyKeys",
                "--type",
                "bool",
                "true"
            ]
        );
        let off = Entry::value("kdeglobals", "KDE", "AnimationDurationFactor", None);
        assert_eq!(
            off.kwriteconfig_args("kdeglobals").last().map(String::as_str),
            Some("--delete")
        );
    }
}
