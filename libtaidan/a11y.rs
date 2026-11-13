use crate::{
    cfg::Desktop,
    home::{Home, HomeCmd},
    kconfig::KConfigEntry as Entry,
    prelude::*,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct A11y {
    pub screen_reader: bool,
    pub zoom: bool,
    pub sticky_keys: bool,
    pub slow_keys: bool,
    pub bounce_keys: bool,
    pub dwell_click: bool,
    pub larger_text: bool,
    pub legible_font: bool,
    pub reduce_motion: bool,
    pub color_filter: u8,
}

const ZOOM: f64 = 2.0;

const PLASMA_DEFAULT_FONT_SIZE: u8 = 10;
const PLASMA_DEFAULT_SANS: &str = "Noto Sans";
const PLASMA_DEFAULT_MONO: &str = "Hack";

const LARGER_TEXT_EXTRA_POINTS: u8 = 2;

const SYSTEM_KDEGLOBALS_BY_PRIORITY: [&str; 2] =
    ["/etc/xdg/kdeglobals", "/usr/share/kde-settings/kde-profile/default/xdg/kdeglobals"];

const LEGIBLE_SANS: &str = "Atkinson Hyperlegible Next";
const LEGIBLE_MONO: &str = "Atkinson Hyperlegible Mono";

const GNOME_DEFAULT_FONT_SIZE: u8 = 11;
const GNOME_LARGE_TEXT_SCALE: &str = "1.25";

const XFCE_DEFAULT_FONT_SIZE: u8 = 10;
const XFCE_LARGER_TEXT_DPI: &str = "120";

fn qt_font_string(family: &str, size: u8) -> String {
    format!("{family},{size},-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SystemFonts {
    sans: String,
    mono: String,
    point_size: u8,
}

impl Default for SystemFonts {
    fn default() -> Self {
        Self {
            sans: PLASMA_DEFAULT_SANS.to_owned(),
            mono: PLASMA_DEFAULT_MONO.to_owned(),
            point_size: PLASMA_DEFAULT_FONT_SIZE,
        }
    }
}

impl SystemFonts {
    fn read() -> Self {
        let files =
            SYSTEM_KDEGLOBALS_BY_PRIORITY.iter().filter_map(|p| std::fs::read_to_string(p).ok());
        Self::from_kdeglobals(&files.collect_vec())
    }

    fn from_kdeglobals(files_by_priority: &[String]) -> Self {
        let get = |key: &str| {
            (files_by_priority.iter())
                .find_map(|f| kconfig_value(f, "General", key))
                .and_then(parse_qt_font)
        };
        let default = Self::default();
        let sans = get("font");
        let mono = get("fixed");
        Self {
            point_size: sans.as_ref().and_then(|f| f.1).unwrap_or(default.point_size),
            sans: sans.map_or(default.sans, |f| f.0),
            mono: mono.map_or(default.mono, |f| f.0),
        }
    }
}

fn kconfig_value<'a>(file: &'a str, group: &str, key: &str) -> Option<&'a str> {
    let mut in_group = false;
    for line in file.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let is_file_flag = name.starts_with('$');
            if !is_file_flag {
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

fn parse_qt_font(font: &str) -> Option<(String, Option<u8>)> {
    let mut fields = font.split(',');
    let family = fields.next()?.trim();
    if family.is_empty() {
        return None;
    }
    let point_size = fields.next().and_then(parse_whole_point_size);
    Some((family.to_owned(), point_size))
}

fn parse_whole_point_size(field: &str) -> Option<u8> {
    let field = field.trim();
    let whole = field.split_once('.').map_or(field, |(whole, _)| whole);
    whole.parse::<u8>().ok().filter(|&points| points > 0)
}

impl A11y {
    #[must_use]
    pub fn session_entries(&self) -> Vec<Entry> {
        let kwin_filter_mode = self.color_filter.checked_sub(1);
        vec![
            Entry::bool("kaccessrc", "Keyboard", "StickyKeys", self.sticky_keys),
            Entry::bool("kaccessrc", "Keyboard", "SlowKeys", self.slow_keys),
            Entry::bool("kaccessrc", "Keyboard", "BounceKeys", self.bounce_keys),
            Entry::value(
                "kdeglobals",
                "KDE",
                "AnimationDurationFactor",
                self.reduce_motion.then_some("0"),
            ),
            Entry::bool(
                "kwinrc",
                "Plugins",
                "colorblindnesscorrectionEnabled",
                kwin_filter_mode.is_some(),
            ),
            Entry::value(
                "kwinrc",
                "Effect-colorblindnesscorrection",
                "Mode",
                kwin_filter_mode.map(|mode| mode.to_string()),
            ),
            Entry::bool("kwinrc", "Plugins", "dwellclickerEnabled", self.dwell_click),
        ]
    }

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
        all.user_entries().into_iter().map(|e| Entry::delete(e.file, e.group, e.key)).collect()
    }

    fn user_entries(&self) -> Vec<Entry> {
        self.user_entries_with(&SystemFonts::read())
    }

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
                system.point_size.saturating_add(LARGER_TEXT_EXTRA_POINTS)
            } else {
                system.point_size
            };
            let (sans, mono) = if self.legible_font {
                (LEGIBLE_SANS, LEGIBLE_MONO)
            } else {
                (system.sans.as_str(), system.mono.as_str())
            };
            let font = qt_font_string(sans, size);
            for key in ["font", "menuFont", "toolBarFont"] {
                entries.push(Entry::value("kdeglobals", "General", key, Some(font.clone())));
            }
            entries.push(Entry::value(
                "kdeglobals",
                "General",
                "smallestReadableFont",
                Some(qt_font_string(sans, size.saturating_sub(2).max(1))),
            ));
            let mono = qt_font_string(mono, size);
            entries.push(Entry::value("kdeglobals", "General", "fixed", Some(mono)));
            entries.push(Entry::value("kdeglobals", "WM", "activeFont", Some(font)));
        }
        entries
    }
}

const ORCA_AUTOSTART: &str = "[Desktop Entry]
Type=Application
Name=Orca Screen Reader
Exec=orca --replace
NoDisplay=true
NotShowIn=GNOME;KDE;
X-GNOME-AutoRestart=true
";

const ORCA_AUTOSTART_OVERRIDE_PATH: &str = ".config/autostart/orca-autostart.desktop";

const fn flag(on: bool) -> Option<&'static str> {
    Some(if on { "true" } else { "false" })
}

impl A11y {
    fn desktop_commands(&self, desktop: Desktop) -> Vec<Vec<String>> {
        use crate::home::gsettings;
        if desktop == Desktop::Xfce {
            return self.xfconf_commands();
        }
        let keyboard = "org.gnome.desktop.a11y.keyboard";
        let apps = "org.gnome.desktop.a11y.applications";
        let interface = "org.gnome.desktop.interface";
        let mag = ZOOM.to_string();
        let font = |family: &str| format!("'{family} {GNOME_DEFAULT_FONT_SIZE}'");
        let (sans, mono) = (font(LEGIBLE_SANS), font(LEGIBLE_MONO));
        vec![
            gsettings(apps, "screen-reader-enabled", flag(self.screen_reader)),
            // Budgie apps only talk to Orca with this on
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
                self.larger_text.then_some(GNOME_LARGE_TEXT_SCALE),
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
        use crate::home::xfconf;
        let font = |family: &str| format!("{family} {XFCE_DEFAULT_FONT_SIZE}");
        let (sans, mono) = (font(LEGIBLE_SANS), font(LEGIBLE_MONO));
        let title = format!("{LEGIBLE_SANS} Bold {}", XFCE_DEFAULT_FONT_SIZE - 1);
        vec![
            xfconf(
                "xfce4-session",
                "/general/StartAssistiveTechnologies",
                "bool",
                flag(self.screen_reader),
            ),
            xfconf("accessibility", "/StickyKeys", "bool", flag(self.sticky_keys)),
            xfconf("accessibility", "/SlowKeys", "bool", flag(self.slow_keys)),
            xfconf("accessibility", "/BounceKeys", "bool", flag(self.bounce_keys)),
            xfconf(
                "xsettings",
                "/Xft/DPI",
                "int",
                self.larger_text.then_some(XFCE_LARGER_TEXT_DPI),
            ),
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

    fn needs_orca_autostart(&self, desktop: Desktop) -> bool {
        self.screen_reader && desktop != Desktop::Gnome
    }

    fn desktop_commands_if_installed(&self, desktop: Desktop) -> Vec<Vec<String>> {
        let tool =
            if desktop == Desktop::Xfce { "/usr/bin/xfconf-query" } else { "/usr/bin/gsettings" };
        if !std::path::Path::new(tool).exists() {
            tracing::info!(tool, "not found; skipping accessibility settings");
            return vec![];
        }
        self.desktop_commands(desktop)
    }

    fn home_cmds(&self, home: &Home, desktop: Desktop, previous: Option<Self>) -> Vec<HomeCmd> {
        if desktop == Desktop::Plasma {
            let mut entries = if previous.is_some() { Self::reset_entries() } else { vec![] };
            entries.extend(self.user_entries());
            return home.kconfig(&entries).map(HomeCmd::best_effort).collect();
        }
        let session = home.session(self.desktop_commands_if_installed(desktop));
        let mut cmds = session.map(HomeCmd::best_effort).into_iter().collect_vec();
        if self.needs_orca_autostart(desktop) {
            cmds.push(home.write_file(ORCA_AUTOSTART_OVERRIDE_PATH, ORCA_AUTOSTART));
        } else if previous.is_some_and(|a| a.needs_orca_autostart(desktop)) {
            cmds.push(home.remove_file(ORCA_AUTOSTART_OVERRIDE_PATH).best_effort());
        }
        cmds
    }

    pub async fn apply(&self, home: &Home, desktop: Desktop, previous: Option<Self>) -> Res<()> {
        home.run(self.home_cmds(home, desktop, previous)).await
    }
}

pub async fn apply_to_user(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Res<()> {
    apply_to_gdm_greeter(settings.a11y.screen_reader).await;
    if cfg.taidan0.systemd_homed || settings.a11y == A11y::default() {
        return Ok(());
    }
    settings.a11y.apply(&Home::user(settings, cfg).await?, cfg.desktop(), None).await
}

pub async fn reapply_to_user(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    applied: A11y,
) -> Res<()> {
    if settings.a11y == applied {
        return Ok(());
    }
    if settings.a11y.screen_reader != applied.screen_reader {
        apply_to_gdm_greeter(settings.a11y.screen_reader).await;
    }
    settings.a11y.apply(&Home::user(settings, cfg).await?, cfg.desktop(), Some(applied)).await
}

async fn apply_to_gdm_greeter(on: bool) {
    if !std::path::Path::new(crate::helper::GDM_DCONF_PROFILE).exists() {
        return;
    }
    if let Err(e) = crate::root(crate::Verb::GdmScreenReader { on }).await {
        tracing::warn!(?e, on, "cannot set the screen reader on the login screen");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            SystemFonts {
                sans: "Inter".to_owned(),
                mono: "Noto Sans Mono".to_owned(),
                point_size: 11
            }
        );
        let larger = A11y { larger_text: true, ..A11y::default() }.user_entries_with(&system);
        let value = |key: &str| larger.iter().find(|e| e.key == key).unwrap().value.clone();
        assert!(value("font").unwrap().starts_with("Inter,13,"));
        assert!(value("fixed").unwrap().starts_with("Noto Sans Mono,13,"));
        assert!(value("smallestReadableFont").unwrap().starts_with("Inter,11,"));

        assert_eq!(SystemFonts::from_kdeglobals(&[]), SystemFonts::default());
    }

    #[test]
    fn pixel_sized_font_keeps_the_default_point_size() {
        let px = SystemFonts::from_kdeglobals(&["[General]\nfont=Cantarell,-1,14,5\n".to_owned()]);
        assert_eq!((px.sans.as_str(), px.point_size), ("Cantarell", PLASMA_DEFAULT_FONT_SIZE));
    }

    #[test]
    fn desktop_commands() {
        let a11y = A11y { sticky_keys: true, legible_font: true, ..A11y::default() };
        let gnome = a11y.desktop_commands(Desktop::Gnome);
        assert!(gnome.iter().all(|c| c[2] == "gsettings"));
        assert!(gnome.iter().any(|c| c.ends_with(&["stickykeys-enable".into(), "true".into()])));
        assert!(gnome.iter().any(|c| c.last().unwrap() == "'Atkinson Hyperlegible Next 11'"));
        let resets_turned_off_larger_text =
            gnome.iter().any(|c| c[3] == "reset" && c.last().unwrap() == "text-scaling-factor");
        assert!(resets_turned_off_larger_text);

        let xfce = a11y.desktop_commands(Desktop::Xfce);
        assert!(xfce.iter().all(|c| c[2] == "xfconf-query"));
        let sticky = xfce.iter().find(|c| c.contains(&"/StickyKeys".to_owned())).unwrap();
        assert_eq!(sticky[7..], ["-n", "-t", "bool", "-s", "true"]);
        let dpi = xfce.iter().find(|c| c.contains(&"/Xft/DPI".to_owned())).unwrap();
        assert_eq!(dpi.last().unwrap(), "-r");
    }

    #[test]
    fn home_cmds_per_desktop() {
        let skel = Home::Skel("/skel".into());
        let a11y = A11y { screen_reader: true, ..A11y::default() };
        let plasma = a11y.home_cmds(&skel, Desktop::Plasma, None);
        assert_eq!(
            plasma,
            skel.kconfig(&a11y.user_entries()).map(HomeCmd::best_effort).collect_vec()
        );
        let orca = skel.write_file(ORCA_AUTOSTART_OVERRIDE_PATH, ORCA_AUTOSTART);
        assert!(a11y.home_cmds(&skel, Desktop::Budgie, None).contains(&orca));
        assert!(!a11y.home_cmds(&skel, Desktop::Gnome, None).contains(&orca));
        let off = A11y::default().home_cmds(&skel, Desktop::Xfce, Some(a11y));
        assert!(off.contains(&skel.remove_file(ORCA_AUTOSTART_OVERRIDE_PATH).best_effort()));
    }

    #[test]
    fn orca_autostart_outside_gnome() {
        let a11y = A11y { screen_reader: true, ..A11y::default() };
        assert!(!a11y.needs_orca_autostart(Desktop::Gnome));
        assert!(
            a11y.needs_orca_autostart(Desktop::Budgie) && a11y.needs_orca_autostart(Desktop::Xfce)
        );
        assert!(!A11y::default().needs_orca_autostart(Desktop::Xfce));
    }
}
