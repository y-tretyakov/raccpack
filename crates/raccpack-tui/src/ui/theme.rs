//! Nocturnal semantic colour palette.

use ratatui::style::Color;

/// Background — very dark grey, near-black.
pub const BG: Color = Color::Rgb(0x0b, 0x0c, 0x0e);

/// Foreground — warm off-white.
pub const FG: Color = Color::Rgb(0xe8, 0xe6, 0xe1);

/// Accent — muted teal for interactive elements.
pub const ACCENT: Color = Color::Rgb(0x56, 0xb6, 0xc2);

/// Danger — errors, critical findings.
pub const DANGER: Color = Color::Rgb(0xe0, 0x6c, 0x75);

/// Warning — non-critical alerts.
pub const WARNING: Color = Color::Rgb(0xe5, 0xc0, 0x7b);

/// Success — completed operations, clean state.
pub const SUCCESS: Color = Color::Rgb(0x98, 0xc3, 0x79);

/// Muted — secondary text, disabled items.
pub const MUTED: Color = Color::Rgb(0x5c, 0x63, 0x70);

/// Border — panel and separator lines.
pub const BORDER: Color = Color::Rgb(0x3e, 0x44, 0x51);

/// Surface — card and row background tint.
pub const SURFACE: Color = Color::Rgb(0x1a, 0x1c, 0x20);

/// Selection — highlighted / focused item.
pub const SELECTION: Color = Color::Rgb(0x3b, 0x40, 0x48);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bg_is_near_black() {
        assert_eq!(BG, Color::Rgb(0x0b, 0x0c, 0x0e));
    }

    #[test]
    fn fg_is_warm_offwhite() {
        assert_eq!(FG, Color::Rgb(0xe8, 0xe6, 0xe1));
    }

    #[test]
    fn accent_is_teal() {
        assert_eq!(ACCENT, Color::Rgb(0x56, 0xb6, 0xc2));
    }

    #[test]
    fn danger_is_red() {
        assert_eq!(DANGER, Color::Rgb(0xe0, 0x6c, 0x75));
    }

    #[test]
    fn warning_is_yellow() {
        assert_eq!(WARNING, Color::Rgb(0xe5, 0xc0, 0x7b));
    }

    #[test]
    fn success_is_green() {
        assert_eq!(SUCCESS, Color::Rgb(0x98, 0xc3, 0x79));
    }

    #[test]
    fn muted_is_grey() {
        assert_eq!(MUTED, Color::Rgb(0x5c, 0x63, 0x70));
    }

    #[test]
    fn border_is_dark_grey() {
        assert_eq!(BORDER, Color::Rgb(0x3e, 0x44, 0x51));
    }

    #[test]
    fn surface_is_dark() {
        assert_eq!(SURFACE, Color::Rgb(0x1a, 0x1c, 0x20));
    }

    #[test]
    fn selection_is_medium_grey() {
        assert_eq!(SELECTION, Color::Rgb(0x3b, 0x40, 0x48));
    }

    #[test]
    fn all_colours_are_rgb() {
        let colours = [
            BG, FG, ACCENT, DANGER, WARNING, SUCCESS, MUTED, BORDER, SURFACE, SELECTION,
        ];
        for c in &colours {
            assert!(matches!(c, Color::Rgb(_, _, _)), "expected RGB: {c:?}");
        }
    }
}
