use slint::{Color, ModelRc, VecModel};

use crate::ui::{AccentItem, Appearance};

const AA_TEXT_CONTRAST: f32 = 4.5;
const CHECK_MARK_CONTRAST: f32 = 3.5;

fn relative_luminance(c: Color) -> f32 {
    let lin = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126f32.mul_add(lin(c.red()), 0.7152f32.mul_add(lin(c.green()), 0.0722 * lin(c.blue())))
}

fn white_text_contrast(c: Color) -> f32 {
    1.05 / (relative_luminance(c) + 0.05)
}

fn readable_fill(c: Color) -> Color {
    (0..=100u8)
        .rev()
        .map(|pct| {
            let scale =
                |v: u8| u8::try_from((u16::from(v) * u16::from(pct) + 50) / 100).unwrap_or(u8::MAX);
            Color::from_rgb_u8(scale(c.red()), scale(c.green()), scale(c.blue()))
        })
        .find(|&c| white_text_contrast(c) >= AA_TEXT_CONTRAST)
        .unwrap_or(Color::from_rgb_u8(0, 0, 0))
}

fn check_mark_color(c: Color) -> Color {
    if white_text_contrast(c) >= CHECK_MARK_CONTRAST {
        Color::from_rgb_u8(255, 255, 255)
    } else {
        Color::from_rgb_u8(0x1e, 0x1e, 0x1e)
    }
}

pub fn initialize_ui(appearance: &Appearance<'_>) {
    if !libtaidan::theme::accent_supported(&crate::CFG.edition) {
        return;
    }
    let accents = (libtaidan::theme::Accent::ALL.iter())
        .map(|a| {
            let color = crate::cfg::parse_color(a.hex()).unwrap_or_default();
            AccentItem {
                id: a.id().into(),
                color,
                fill: readable_fill(color),
                check: check_mark_color(color),
            }
        })
        .collect::<Vec<_>>();
    appearance.set_accents(ModelRc::new(VecModel::from(accents)));
}

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
    use crate::cfg::parse_color;

    #[test]
    fn fills_take_white_text() {
        for accent in libtaidan::theme::Accent::ALL {
            let color = parse_color(accent.hex()).unwrap();
            let fill = readable_fill(color);
            assert!(
                white_text_contrast(fill) >= AA_TEXT_CONTRAST,
                "{accent:?} is {}",
                white_text_contrast(fill)
            );
            assert!(
                white_text_contrast(color) < AA_TEXT_CONTRAST || fill == color,
                "{accent:?} was darkened though it passed"
            );
        }
        assert_eq!(
            readable_fill(parse_color("#3584e4").unwrap()),
            Color::from_rgb_u8(0x30, 0x77, 0xcd)
        );
    }

    #[test]
    fn checks_stand_out() {
        let contrast = |a: Color, b: Color| {
            let (a, b) = (relative_luminance(a) + 0.05, relative_luminance(b) + 0.05);
            a.max(b) / a.min(b)
        };
        for accent in libtaidan::theme::Accent::ALL {
            let color = parse_color(accent.hex()).unwrap();
            let c = contrast(check_mark_color(color), color);
            assert!(c >= CHECK_MARK_CONTRAST, "{accent:?}'s check is {c}");
        }
    }
}
