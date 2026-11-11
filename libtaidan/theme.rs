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
    accent_supported_with(edition, crate::a11y::is_plasma())
}

fn accent_supported_with(edition: &str, is_plasma: bool) -> bool {
    is_plasma || crate::cfg::desktop(edition) == "gnome"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Themes {
    pub gtk: &'static str,
    pub icons: Option<&'static str>,
    pub wm: Option<&'static str>,
}

#[must_use]
pub fn themes(edition: &str, dark: bool) -> Option<Themes> {
    Some(match crate::cfg::desktop(edition).as_str() {
        "gnome" => {
            Themes { gtk: if dark { "Adwaita-dark" } else { "Adwaita" }, icons: None, wm: None }
        }
        // budgie and xfce panels stay dark in either mode, so light mode keeps the icon themes
        // with light panel icons
        "budgie" => Themes {
            gtk: if dark { "Fluent-round-Dark" } else { "Fluent-round-Light" },
            icons: Some(if dark { "Fluent-dark" } else { "Fluent" }),
            wm: None,
        },
        "xfce" => Themes {
            gtk: if dark { "Orchis-Dark" } else { "Orchis-Light" },
            icons: Some(if dark { "Papirus-Dark" } else { "Papirus" }),
            wm: Some(if dark { "Orchis-Dark" } else { "Orchis-Light" }),
        },
        _ => return None,
    })
}

#[must_use]
pub fn uses_xfconf(edition: &str) -> bool {
    crate::cfg::desktop(edition) == "xfce"
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
        assert!(accent_supported_with("gnome", false));
        assert!(accent_supported_with("atomic-gnome", false));
        assert!(accent_supported_with("kde", true));
        assert!(accent_supported_with("", true));
        assert!(!accent_supported_with("kde", false));
        assert!(!accent_supported_with("budgie", false));
        assert!(!accent_supported_with("xfce", false));
        assert!(!accent_supported_with("", false));
    }

    #[test]
    fn themes_per_edition() {
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
        assert!(uses_xfconf("xfce") && !uses_xfconf("gnome"));
    }
}
