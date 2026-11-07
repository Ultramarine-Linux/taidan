//! what the new user's desktop looks like: light or dark, the accent color, and the GTK theme that
//! goes with them on desktops where `color-scheme` alone doesn't reach GTK 3 apps

/// an accent color, named as GNOME's `org.gnome.desktop.interface accent-color` names them
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Slate,
}

impl Accent {
    pub const ALL: [Self; 9] = [
        Self::Blue,
        Self::Teal,
        Self::Green,
        Self::Yellow,
        Self::Orange,
        Self::Red,
        Self::Pink,
        Self::Purple,
        Self::Slate,
    ];

    /// the `accent-color` value
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Teal => "teal",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Orange => "orange",
            Self::Red => "red",
            Self::Pink => "pink",
            Self::Purple => "purple",
            Self::Slate => "slate",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }

    /// libadwaita's color for it, which Plasma gets too so both look the same, as `#rrggbb`
    #[must_use]
    pub const fn hex(self) -> &'static str {
        match self {
            Self::Blue => "#3584e4",
            Self::Teal => "#2190a4",
            Self::Green => "#3a944a",
            Self::Yellow => "#c88800",
            Self::Orange => "#ed5b00",
            Self::Red => "#e62d42",
            Self::Pink => "#d56199",
            Self::Purple => "#9141ac",
            Self::Slate => "#6f8396",
        }
    }
}

/// whether the edition's desktop follows an accent color all the way through: GNOME 47+ (shell
/// and libadwaita apps) and Plasma. Budgie and Xfce use GTK 3 themes that ignore `accent-color`
#[must_use]
pub fn accent_supported(edition: &str) -> bool {
    ["gnome", "plasma", "kde"].iter().any(|e| edition.eq_ignore_ascii_case(e))
}

/// the GTK and icon themes that go with light or dark mode, and Xfce's window manager theme
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Themes {
    pub gtk: &'static str,
    /// [`None`] where the icons work in either mode
    pub icons: Option<&'static str>,
    /// only on Xfce, where xfwm4 draws the title bars from a theme of its own
    pub wm: Option<&'static str>,
}

/// the themes for the edition, [`None`] to leave them alone. GNOME's are for GTK 3 apps, which
/// don't read `color-scheme`. Budgie and Xfce keep their panel dark in either mode, so the light
/// icon themes are the ones with light panel icons: `Fluent` rather than `Fluent-light`, `Papirus`
/// rather than `Papirus-Light`
#[must_use]
pub fn themes(edition: &str, dark: bool) -> Option<Themes> {
    Some(match edition.to_ascii_lowercase().as_str() {
        "gnome" => {
            Themes { gtk: if dark { "Adwaita-dark" } else { "Adwaita" }, icons: None, wm: None }
        }
        // as Ultramarine's budgie-filesystem gschema override has them for dark
        "budgie" | "flagship" => Themes {
            gtk: if dark { "Fluent-round-Dark" } else { "Fluent-round-Light" },
            icons: Some(if dark { "Fluent-dark" } else { "Fluent" }),
            wm: None,
        },
        // as Ultramarine's xfce-config has them for dark
        "xfce" => Themes {
            gtk: if dark { "Orchis-Dark" } else { "Orchis-Light" },
            icons: Some(if dark { "Papirus-Dark" } else { "Papirus" }),
            wm: Some(if dark { "Orchis-Dark" } else { "Orchis-Light" }),
        },
        _ => return None,
    })
}

/// whether the edition reads its themes from xfconf rather than `gsettings`
#[must_use]
pub fn uses_xfconf(edition: &str) -> bool {
    edition.eq_ignore_ascii_case("xfce")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_ids_round_trip() {
        for accent in Accent::ALL {
            assert_eq!(Accent::from_id(accent.id()), Some(accent));
            let json = serde_json::to_value(accent).unwrap();
            assert_eq!(json, accent.id());
        }
        assert_eq!(Accent::from_id("bogus"), None);
    }

    #[test]
    fn accent_only_where_it_works() {
        assert!(accent_supported("gnome"));
        assert!(accent_supported("Plasma"));
        assert!(!accent_supported("flagship"));
        assert!(!accent_supported("xfce"));
        assert!(!accent_supported(""));
    }

    #[test]
    fn themes_per_edition() {
        let budgie = themes("flagship", true).unwrap();
        assert_eq!((budgie.gtk, budgie.icons), ("Fluent-round-Dark", Some("Fluent-dark")));
        let budgie = themes("budgie", false).unwrap();
        assert_eq!((budgie.gtk, budgie.icons), ("Fluent-round-Light", Some("Fluent")));
        let xfce = themes("xfce", false).unwrap();
        assert_eq!(
            (xfce.gtk, xfce.icons, xfce.wm),
            ("Orchis-Light", Some("Papirus"), Some("Orchis-Light"))
        );
        let gnome = themes("gnome", true).unwrap();
        assert_eq!((gnome.gtk, gnome.icons), ("Adwaita-dark", None));
        assert_eq!(themes("plasma", true), None);
        assert!(uses_xfconf("xfce") && !uses_xfconf("gnome"));
    }
}
