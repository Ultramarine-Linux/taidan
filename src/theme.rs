//! the theme page's accent colors, see [`libtaidan::theme::Accent`]
use slint::{Color, ModelRc, VecModel};

use crate::ui::{AccentItem, Appearance};

/// `#rrggbb` as a [`Color`]
fn parse_hex(hex: &str) -> Color {
    let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or_default();
    Color::from_argb_encoded(0xff00_0000 | rgb)
}

/// offers the accents if the edition's desktop follows them, otherwise there's no picker
pub fn initialize_ui(appearance: &Appearance<'_>) {
    if !libtaidan::theme::accent_supported(&crate::CFG.edition) {
        return;
    }
    let accents = (libtaidan::theme::Accent::ALL.iter())
        .map(|a| AccentItem { id: a.id().into(), color: parse_hex(a.hex()) })
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
}
