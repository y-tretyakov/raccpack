//! Primitive theme tokens — raw 24-bit hex colours (Visual System 2.0).

use ratatui::style::Color;

/// Canonical hex table (name → hex). Mirrors `docs/design-tokens/raccpack.tokens.json`
/// → `color.primitive`. The typed constants below stay in sync via the
/// `hex_table_matches_typed_constants` test.
pub const HEX: [(&str, &str); 14] = [
    ("bg", "#080A0D"),
    ("surface", "#0D1117"),
    ("surface-raised", "#131820"),
    ("border", "#242B34"),
    ("text", "#E6EDF3"),
    ("muted", "#6B7785"),
    ("brand-primary", "#FF8A3D"),
    ("brand-bright", "#FFB454"),
    ("brand-dim", "#A65329"),
    ("success", "#7CCB5E"),
    ("warning", "#E6B450"),
    ("danger", "#F06C75"),
    ("info", "#61AFEF"),
    ("analysis", "#C678DD"),
];

/// Background — deep graphite.
pub const BG: Color = Color::Rgb(0x08, 0x0a, 0x0d);

/// Surface — base panels, subtle zebra rows.
pub const SURFACE: Color = Color::Rgb(0x0d, 0x11, 0x17);

/// Raised surface — selection wash, active rows.
pub const SURFACE_RAISED: Color = Color::Rgb(0x13, 0x18, 0x20);

/// Border — panel edges and separator rules.
pub const BORDER: Color = Color::Rgb(0x24, 0x2b, 0x34);

/// Text — primary foreground.
pub const TEXT: Color = Color::Rgb(0xe6, 0xed, 0xf3);

/// Muted — secondary text, hints, empty placeholders.
pub const MUTED: Color = Color::Rgb(0x6b, 0x77, 0x85);

/// Brand primary — RaccPack orange: focus, primary action, identity.
pub const BRAND_PRIMARY: Color = Color::Rgb(0xff, 0x8a, 0x3d);

/// Brand bright — hover / strong emphasis.
pub const BRAND_BRIGHT: Color = Color::Rgb(0xff, 0xb4, 0x54);

/// Brand dim — quiet brand accents.
pub const BRAND_DIM: Color = Color::Rgb(0xa6, 0x53, 0x29);

/// Success — healthy / clean / ok.
pub const SUCCESS: Color = Color::Rgb(0x7c, 0xcb, 0x5e);

/// Warning — attention.
pub const WARNING: Color = Color::Rgb(0xe6, 0xb4, 0x50);

/// Danger — error / destructive.
pub const DANGER: Color = Color::Rgb(0xf0, 0x6c, 0x75);

/// Info — technology / information.
pub const INFO: Color = Color::Rgb(0x61, 0xaf, 0xef);

/// Analysis — detection / DAG.
pub const ANALYSIS: Color = Color::Rgb(0xc6, 0x78, 0xdd);

/// Typed registry: token name → colour, in `HEX` order.
pub const ALL: [(&str, Color); 14] = [
    ("bg", BG),
    ("surface", SURFACE),
    ("surface-raised", SURFACE_RAISED),
    ("border", BORDER),
    ("text", TEXT),
    ("muted", MUTED),
    ("brand-primary", BRAND_PRIMARY),
    ("brand-bright", BRAND_BRIGHT),
    ("brand-dim", BRAND_DIM),
    ("success", SUCCESS),
    ("warning", WARNING),
    ("danger", DANGER),
    ("info", INFO),
    ("analysis", ANALYSIS),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let digits = hex.strip_prefix('#')?;
        if digits.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
        let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
        let b = u8::from_str_radix(&digits[4..6], 16).ok()?;
        Some((r, g, b))
    }

    fn name_to_const(name: &str) -> Color {
        match name {
            "bg" => BG,
            "surface" => SURFACE,
            "surface-raised" => SURFACE_RAISED,
            "border" => BORDER,
            "text" => TEXT,
            "muted" => MUTED,
            "brand-primary" => BRAND_PRIMARY,
            "brand-bright" => BRAND_BRIGHT,
            "brand-dim" => BRAND_DIM,
            "success" => SUCCESS,
            "warning" => WARNING,
            "danger" => DANGER,
            "info" => INFO,
            "analysis" => ANALYSIS,
            other => panic!("unexpected primitive token {other:?}"),
        }
    }

    #[test]
    fn bg_is_deep_graphite() {
        assert_eq!(BG, Color::Rgb(0x08, 0x0a, 0x0d));
    }

    #[test]
    fn surface_is_elevated() {
        assert_eq!(SURFACE, Color::Rgb(0x0d, 0x11, 0x17));
    }

    #[test]
    fn surface_raised_is_brighter_than_surface() {
        assert_eq!(SURFACE_RAISED, Color::Rgb(0x13, 0x18, 0x20));
    }

    #[test]
    fn border_is_mid_graphite() {
        assert_eq!(BORDER, Color::Rgb(0x24, 0x2b, 0x34));
    }

    #[test]
    fn text_is_light() {
        assert_eq!(TEXT, Color::Rgb(0xe6, 0xed, 0xf3));
    }

    #[test]
    fn muted_is_grey() {
        assert_eq!(MUTED, Color::Rgb(0x6b, 0x77, 0x85));
    }

    #[test]
    fn brand_primary_is_raccpack_orange() {
        assert_eq!(BRAND_PRIMARY, Color::Rgb(0xff, 0x8a, 0x3d));
    }

    #[test]
    fn brand_bright_is_lighter_orange() {
        assert_eq!(BRAND_BRIGHT, Color::Rgb(0xff, 0xb4, 0x54));
    }

    #[test]
    fn brand_dim_is_muted_orange() {
        assert_eq!(BRAND_DIM, Color::Rgb(0xa6, 0x53, 0x29));
    }

    #[test]
    fn success_is_green() {
        assert_eq!(SUCCESS, Color::Rgb(0x7c, 0xcb, 0x5e));
    }

    #[test]
    fn warning_is_yellow() {
        assert_eq!(WARNING, Color::Rgb(0xe6, 0xb4, 0x50));
    }

    #[test]
    fn danger_is_red() {
        assert_eq!(DANGER, Color::Rgb(0xf0, 0x6c, 0x75));
    }

    #[test]
    fn info_is_blue() {
        assert_eq!(INFO, Color::Rgb(0x61, 0xaf, 0xef));
    }

    #[test]
    fn analysis_is_purple() {
        assert_eq!(ANALYSIS, Color::Rgb(0xc6, 0x78, 0xdd));
    }

    #[test]
    fn all_colours_are_rgb() {
        for (name, colour) in ALL {
            assert!(
                matches!(colour, Color::Rgb(_, _, _)),
                "primitive {name} must be RGB, got {colour:?}"
            );
        }
    }

    #[test]
    fn primitives_are_pairwise_distinct() {
        for (i, (name_i, colour_i)) in ALL.iter().enumerate() {
            for (name_j, colour_j) in ALL.iter().skip(i + 1) {
                assert_ne!(
                    colour_i, colour_j,
                    "primitives {name_i} and {name_j} must differ"
                );
            }
        }
    }

    #[test]
    fn hex_table_matches_typed_constants() {
        for (name, hex) in HEX {
            let (r, g, b) = parse_hex(hex).expect("primitive hex must be #RRGGBB");
            assert_eq!(
                name_to_const(name),
                Color::Rgb(r, g, b),
                "constant {name} must match its hex {hex}"
            );
        }
    }

    #[test]
    fn ban_teal() {
        for (name, hex) in HEX {
            assert!(
                !hex.eq_ignore_ascii_case("#56B6C2") && !hex.eq_ignore_ascii_case("#7EC8D1"),
                "primitive {name} reintroduces a teal accent: {hex}"
            );
        }
    }
}
