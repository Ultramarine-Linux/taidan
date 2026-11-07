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

/// the GTK theme set through `gsettings` for GTK 3 apps, which don't read `color-scheme`, [`None`]
/// to leave it alone
#[must_use]
pub fn gtk_theme(edition: &str, dark: bool) -> Option<&'static str> {
    match edition.to_ascii_lowercase().as_str() {
        "gnome" => Some(if dark { "Adwaita-dark" } else { "Adwaita" }),
        // Ultramarine's Budgie ships `Fluent-round-Dark` in its gschema override
        "budgie" | "flagship" => {
            Some(if dark { "Fluent-round-Dark" } else { "Fluent-round-Light" })
        }
        _ => None,
    }
}

/// the GTK theme for Xfce, which reads it from xfconf rather than `gsettings`. Ultramarine's
/// xfce-config ships `Orchis-Dark`
#[must_use]
pub fn xfce_theme(edition: &str, dark: bool) -> Option<&'static str> {
    edition.eq_ignore_ascii_case("xfce").then_some(if dark {
        "Orchis-Dark"
    } else {
        "Orchis-Light"
    })
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
    fn gtk_themes() {
        assert_eq!(gtk_theme("flagship", true), Some("Fluent-round-Dark"));
        assert_eq!(gtk_theme("gnome", false), Some("Adwaita"));
        assert_eq!(gtk_theme("plasma", true), None);
        assert_eq!(xfce_theme("xfce", false), Some("Orchis-Light"));
        assert_eq!(xfce_theme("gnome", false), None);
    }
}
