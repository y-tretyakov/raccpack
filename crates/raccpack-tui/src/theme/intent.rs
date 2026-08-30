//! Widget intent layer — what a colour *means*, not its hex.
//!
//! Intents that map 1:1 onto a same-named semantic token (Info, Analysis,
//! Success, Danger, Warning) reuse that token directly instead of duplicating
//! it; only intent tokens carrying new meaning live here.

use ratatui::style::Color;

use crate::theme::semantic;

/// Keyboard focus / current selection — carries the RaccPack brand orange.
pub const FOCUS: Color = semantic::BRAND_PRIMARY;

/// Primary action (confirm, commit, primary CTA) — the same brand orange.
pub const PRIMARY_ACTION: Color = semantic::BRAND_PRIMARY;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_is_brand_primary() {
        assert_eq!(FOCUS, semantic::BRAND_PRIMARY);
    }

    #[test]
    fn focus_is_raccpack_orange() {
        assert_eq!(FOCUS, Color::Rgb(0xff, 0x8a, 0x3d));
    }

    #[test]
    fn primary_action_is_brand_primary() {
        assert_eq!(PRIMARY_ACTION, semantic::BRAND_PRIMARY);
    }

    #[test]
    fn primary_action_is_raccpack_orange() {
        assert_eq!(PRIMARY_ACTION, Color::Rgb(0xff, 0x8a, 0x3d));
    }
}
