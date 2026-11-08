//! the theme page's accent colors, see [`libtaidan::theme::Accent`]
use slint::{Color, ModelRc, VecModel};

use crate::ui::{AccentItem, Appearance};

/// `#rrggbb` as a [`Color`], for the accents' colors, which are always valid
fn parse_hex(hex: &str) -> Color {
    crate::cfg::parse_color(hex).unwrap_or_default()
}

/// WCAG relative luminance
fn luminance(c: Color) -> f32 {
    let lin = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126f32.mul_add(lin(c.red()), 0.7152f32.mul_add(lin(c.green()), 0.0722 * lin(c.blue())))
}

/// WCAG contrast ratio of white text on `c`
fn white_contrast(c: Color) -> f32 {
    1.05 / (luminance(c) + 0.05)
}

/// `c` darkened just enough that white text on it reaches AA's 4.5:1. Most of libadwaita's accents
/// don't on their own (yellow is about 3:1), so buttons and other fills with text use this, and only
/// the swatches show the color itself
fn readable_fill(c: Color) -> Color {
    // scaling every channel keeps the hue, and 1% steps keep it as close to `c` as they can
    (0..=100u8)
        .rev()
        .map(|pct| {
            let scale =
                |v: u8| u8::try_from((u16::from(v) * u16::from(pct) + 50) / 100).unwrap_or(u8::MAX);
            Color::from_rgb_u8(scale(c.red()), scale(c.green()), scale(c.blue()))
        })
        .find(|&c| white_contrast(c) >= 4.5)
        .unwrap_or(Color::from_rgb_u8(0, 0, 0))
}

/// the swatch's check mark on `c`: white, as on the other swatches, unless it's under 3:1 there
/// (non-text contrast, with some room), then near black
fn check_color(c: Color) -> Color {
    if white_contrast(c) >= 3.5 {
        Color::from_rgb_u8(255, 255, 255)
    } else {
        Color::from_rgb_u8(0x1e, 0x1e, 0x1e)
    }
}

/// offers the accents if the edition's desktop follows them, otherwise there's no picker
pub fn initialize_ui(appearance: &Appearance<'_>) {
    if !libtaidan::theme::accent_supported(&crate::CFG.edition) {
        return;
    }
    let accents = (libtaidan::theme::Accent::ALL.iter())
        .map(|a| {
            let color = parse_hex(a.hex());
            AccentItem {
                id: a.id().into(),
                color,
                fill: readable_fill(color),
                check: check_color(color),
            }
        })
        .collect::<Vec<_>>();
    appearance.set_accents(ModelRc::new(VecModel::from(accents)));
}

/// the accent picked on the theme page, [`None`] for the distro's default
#[must_use]
pub fn picked(appearance: &Appearance<'_>) -> Option<libtaidan::theme::Accent> {
    let i = usize::try_from(appearance.get_accent()).ok()?;
    libtaidan::theme::Accent::ALL
        .get(i)
        .copied()
        .filter(|_| libtaidan::theme::accent_supported(&crate::CFG.edition))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex("#3584e4"), Color::from_rgb_u8(0x35, 0x84, 0xe4));
    }

    #[test]
    fn fills_take_white_text() {
        for accent in libtaidan::theme::Accent::ALL {
            let color = parse_hex(accent.hex());
            let fill = readable_fill(color);
            assert!(white_contrast(fill) >= 4.5, "{accent:?} is {}", white_contrast(fill));
            // only as dark as it has to be
            assert!(
                white_contrast(color) < 4.5 || fill == color,
                "{accent:?} was darkened though it passed"
            );
        }
        assert_eq!(readable_fill(parse_hex("#3584e4")), Color::from_rgb_u8(0x30, 0x77, 0xcd));
    }

    #[test]
    fn checks_stand_out() {
        let contrast = |a: Color, b: Color| {
            let (a, b) = (luminance(a) + 0.05, luminance(b) + 0.05);
            a.max(b) / a.min(b)
        };
        for accent in libtaidan::theme::Accent::ALL {
            let color = parse_hex(accent.hex());
            let c = contrast(check_color(color), color);
            assert!(c >= 3.5, "{accent:?}'s check is {c}");
        }
    }
}
