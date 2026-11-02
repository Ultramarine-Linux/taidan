//! accessibility options, as the KDE config entries that turn them on in KWin and Plasma
//!
//! the OOBE writes these for its own KWin session as they change, and for the new user during
//! setup, see [`apply_to_user`]
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
                ("Atkinson Hyperlegible Next", "Atkinson Hyperlegible Mono")
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

/// whether `kwriteconfig6` is there, which it is on Plasma
fn has_kwriteconfig() -> bool {
    std::path::Path::new("/usr/bin/kwriteconfig6").exists()
}

/// writes the options into the new user's config, so their Plasma session starts with them
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
    if cfg.taidan0.systemd_homed {
        return Ok(());
    }
    let entries = settings.a11y.user_entries();
    if entries.is_empty() {
        return Ok(());
    }
    if !has_kwriteconfig() {
        tracing::info!("no kwriteconfig6, so not a Plasma system; skipping accessibility settings");
        return Ok(());
    }
    let home = home_of(&settings.username).await?;
    let config = home.join(".config");
    for entry in entries {
        let file = config.join(entry.file);
        let mut args = vec![
            "-u".to_owned(),
            settings.username.clone(),
            "--".to_owned(),
            "kwriteconfig6".to_owned(),
        ];
        args.extend(entry.kwriteconfig_args(&file.to_string_lossy()));
        // as the user, so the files they get are theirs
        if let Err(e) = crate::root("runuser", &args).await {
            tracing::warn!(?e, ?entry, "cannot write accessibility setting");
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
    skel: &std::path::Path,
) -> Res<bool> {
    let entries = settings.a11y.user_entries();
    if entries.is_empty() || !has_kwriteconfig() {
        return Ok(false);
    }
    let config = skel.join(".config");
    tokio::fs::create_dir_all(&config).await.map_err(Err::Io)?;
    for entry in entries {
        let args = entry.kwriteconfig_args(&config.join(entry.file).to_string_lossy());
        let args = args.iter().map(String::as_str).collect_vec();
        if let Err(e) = crate::steps::acmd("kwriteconfig6", &args).await {
            tracing::warn!(?e, ?entry, "cannot write accessibility setting");
        }
    }
    Ok(true)
}

/// the home directory of `user`, from the passwd database
async fn home_of(user: &str) -> Res<std::path::PathBuf> {
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
