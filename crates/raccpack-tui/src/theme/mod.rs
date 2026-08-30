//! Theme root: primitive hex → semantic meaning → widget intent.
//!
//! Source of truth: `docs/design-tokens/raccpack.tokens.json` (primitive →
//! semantic → component). Flat re-exports keep screen code terse
//! (`theme::FOCUS`, `theme::SURFACE_RAISED`, `theme::BG`, …).

pub mod intent;
pub mod primitive;
pub mod semantic;

pub use intent::*;
pub use semantic::*;

/// Clean name → hex registry of all primitives (token checks, docs, audits).
pub fn all_primitive_hex() -> &'static [(&'static str, &'static str)] {
    &primitive::HEX
}

/// Clean name → colour registry of all primitives.
pub fn all_primitive_rgb() -> &'static [(&'static str, ratatui::style::Color)] {
    &primitive::ALL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_14_primitives() {
        assert_eq!(all_primitive_hex().len(), 14);
        assert_eq!(all_primitive_rgb().len(), 14);
    }

    #[test]
    fn registry_names_agree_and_follow_hex_order() {
        let hex_names: Vec<_> = all_primitive_hex().iter().map(|(n, _)| *n).collect();
        let rgb_names: Vec<_> = all_primitive_rgb().iter().map(|(n, _)| *n).collect();
        assert_eq!(hex_names, rgb_names, "hex and rgb registries must agree");
    }

    #[test]
    fn registry_hexes_are_6_digit_rgb() {
        for (name, hex) in all_primitive_hex() {
            assert_eq!(hex.len(), 7, "{name} hex must be #RRGGBB");
            assert_eq!(
                hex.chars().next(),
                Some('#'),
                "{name} hex must start with #"
            );
        }
    }
}
