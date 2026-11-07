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

/// font sizes in points, for Plasma's default (Noto Sans 10) and with larger text
const FONT_SIZE: u8 = 10;
const LARGER_FONT_SIZE: u8 = 12;

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

    /// everything to write for the new user, nothing when no option is on so their config stays
    /// untouched
    #[must_use]
    pub fn user_entries(&self) -> Vec<Entry> {
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
            let size = if self.larger_text { LARGER_FONT_SIZE } else { FONT_SIZE };
            let (sans, mono) = if self.legible_font {
                (LEGIBLE_SANS, LEGIBLE_MONO)
            } else {
                ("Noto Sans", "Hack")
            };
            for key in ["font", "menuFont", "toolBarFont"] {
                entries.push(Entry::value("kdeglobals", "General", key, Some(font(sans, size))));
            }
            entries.push(Entry::value(
                "kdeglobals",
                "General",
                "smallestReadableFont",
                Some(font(sans, size - 2)),
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

/// [`A11y::desktop_commands`] if there's a tool on this system to run them
fn desktop_commands(settings: &crate::settings::Settings, edition: &str) -> Vec<Vec<String>> {
    let tool = if crate::theme::uses_xfconf(edition) { XFCONF } else { GSETTINGS };
    if !std::path::Path::new(tool).exists() {
        tracing::info!(tool, "not found; skipping accessibility settings");
        return vec![];
    }
    settings.a11y.desktop_commands(edition)
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
    if cfg.taidan0.systemd_homed || settings.a11y == A11y::default() {
        return Ok(());
    }
    let home = home_of(&settings.username).await?;
    // as the user, so the files they get are theirs
    let as_user = |cmd: &[String]| {
        let mut args = vec!["-u".to_owned(), settings.username.clone(), "--".to_owned()];
        args.extend_from_slice(cmd);
        args
    };
    if is_plasma() {
        let config = home.join(".config");
        for entry in settings.a11y.user_entries() {
            let mut cmd = vec!["kwriteconfig6".to_owned()];
            cmd.extend(entry.kwriteconfig_args(&config.join(entry.file).to_string_lossy()));
            if let Err(e) = crate::root("runuser", as_user(&cmd)).await {
                tracing::warn!(?e, ?entry, "cannot write accessibility setting");
            }
        }
        return Ok(());
    }
    for cmd in desktop_commands(settings, &cfg.edition) {
        let mut env = vec!["env".to_owned()];
        env.extend(in_home(&home, cmd));
        if let Err(e) = crate::root("runuser", as_user(&env)).await {
            tracing::warn!(?e, "cannot write accessibility setting");
        }
    }
    if settings.a11y.needs_orca_autostart(&cfg.edition) {
        let path = home.join(ORCA_AUTOSTART_PATH);
        let dir = path.parent().unwrap_or(&home).to_string_lossy().into_owned();
        let path = path.to_string_lossy().into_owned();
        let mkdir = as_user(&["mkdir".to_owned(), "-p".to_owned(), dir]);
        let res = match crate::root("runuser", mkdir).await {
            Ok(()) => {
                let tee = as_user(&["tee".to_owned(), path]);
                crate::root_with_stdin("runuser", tee, ORCA_AUTOSTART.as_bytes()).await
            }
            Err(e) => Err(e),
        };
        if let Err(e) = res {
            tracing::warn!(?e, "cannot make Orca start at login");
        }
    }
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
    for cmd in desktop_commands(settings, &cfg.edition) {
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
    fn nothing_on_writes_nothing() {
        assert!(A11y::default().user_entries().is_empty());
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

        let legible =
            A11y { legible_font: true, larger_text: true, ..A11y::default() }.user_entries();
        let font = legible.iter().find(|e| e.key == "font").unwrap();
        assert_eq!(
            font.value.as_deref(),
            Some("Atkinson Hyperlegible Next,12,-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
        );
        let fixed = legible.iter().find(|e| e.key == "fixed").unwrap();
        assert!(fixed.value.as_deref().unwrap().starts_with("Atkinson Hyperlegible Mono,12,"));
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
