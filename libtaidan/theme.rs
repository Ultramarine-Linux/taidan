use crate::cfg::Desktop;

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

#[must_use]
pub fn accent_supported(edition: &str) -> bool {
    Desktop::current(edition).supports_accent()
}

impl Desktop {
    #[must_use]
    pub fn supports_accent(self) -> bool {
        matches!(self, Self::Plasma | Self::Gnome)
    }

    #[must_use]
    pub const fn themes(self, dark: bool) -> Option<Themes> {
        Some(match self {
            Self::Gnome => {
                Themes { gtk: if dark { "Adwaita-dark" } else { "Adwaita" }, icons: None, wm: None }
            }
            // budgie and xfce panels stay dark in either mode, so light mode keeps the icon themes
            // with light panel icons
            Self::Budgie => Themes {
                gtk: if dark { "Fluent-round-Dark" } else { "Fluent-round-Light" },
                icons: Some(if dark { "Fluent-dark" } else { "Fluent" }),
                wm: None,
            },
            Self::Xfce => Themes {
                gtk: if dark { "Orchis-Dark" } else { "Orchis-Light" },
                icons: Some(if dark { "Papirus-Dark" } else { "Papirus" }),
                wm: Some(if dark { "Orchis-Dark" } else { "Orchis-Light" }),
            },
            Self::Plasma | Self::Other => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Themes {
    pub gtk: &'static str,
    pub icons: Option<&'static str>,
    pub wm: Option<&'static str>,
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
        assert!(Desktop::Gnome.supports_accent());
        assert!(Desktop::from_edition("atomic-gnome").supports_accent());
        assert!(Desktop::Plasma.supports_accent());
        assert!(!Desktop::from_edition("kde").supports_accent());
        assert!(!Desktop::Budgie.supports_accent());
        assert!(!Desktop::Xfce.supports_accent());
        assert!(!Desktop::from_edition("").supports_accent());
    }

    #[test]
    fn themes_per_edition() {
        let themes = |edition, dark| Desktop::from_edition(edition).themes(dark);
        let budgie = themes("atomic-budgie", true).unwrap();
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
        assert_eq!(Desktop::Plasma.themes(true), None);
    }
}
