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

/// Accent dim — softer accent when a region is not focused. ← color.semantic.accent-dim
pub const ACCENT_DIM: Color = Color::Rgb(0x7e, 0xc8, 0xd1);

/// Git clean — git repo present. ← color.semantic.git-clean (equals SUCCESS)
pub const GIT_CLEAN: Color = Color::Rgb(0x98, 0xc3, 0x79);

/// Git dirty or absent — not a git repo or neutral absent mark. ← color.semantic.git-dirty-or-absent (equals MUTED)
pub const GIT_DIRTY_OR_ABSENT: Color = Color::Rgb(0x5c, 0x63, 0x70);

// ── Space tokens (terminal cells/columns) ─────────────────────────────────────
// Source of truth: `docs/design-tokens/raccpack.tokens.json` → `space.semantic.*`.
// The TUI shares these only by name with Desktop (which maps them to px/rem in
// its own transform); keep the numeric values here in sync with the JSON.

/// Sidebar width in character columns. ← space.semantic.sidebar-width (23).
pub const SPACE_SIDEBAR_WIDTH: u16 = 23;

/// Header height in rows. ← space.semantic.header-height (1).
pub const SPACE_HEADER_HEIGHT: u16 = 1;

/// Footer height in rows. ← space.semantic.footer-height (1).
pub const SPACE_FOOTER_HEIGHT: u16 = 1;

/// Detail strip height in rows. ← space.semantic.detail-height (7).
pub const SPACE_DETAIL_HEIGHT: u16 = 7;

/// Accent rail width for the active sidebar item. ← space.semantic.sidebar-accent-bar (2).
pub const SPACE_SIDEBAR_ACCENT_BAR: u16 = 2;

/// Accent bar width for the selected table row. ← space.semantic.row-accent-bar (1).
pub const SPACE_ROW_ACCENT_BAR: u16 = 1;

// ── Component glyphs ──────────────────────────────────────────────────────────

/// Git repo present. ← component.git.clean-glyph (●).
pub const GIT_CLEAN_GLYPH: &str = "●";

/// Git repo absent / neutral mark. ← component.git.absent-glyph (·).
pub const GIT_ABSENT_GLYPH: &str = "·";

/// Empty-cell placeholder. ← component.table.empty-placeholder (use `·`, not `-`).
pub const EMPTY_PLACEHOLDER: &str = "·";

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
    fn accent_dim_is_softer_teal() {
        assert_eq!(ACCENT_DIM, Color::Rgb(0x7e, 0xc8, 0xd1));
    }

    #[test]
    fn git_clean_is_success() {
        assert_eq!(GIT_CLEAN, SUCCESS);
    }

    #[test]
    fn git_dirty_or_absent_is_muted() {
        assert_eq!(GIT_DIRTY_OR_ABSENT, MUTED);
    }

    #[test]
    fn accent_dim_differs_from_accent() {
        assert_ne!(ACCENT_DIM, ACCENT);
    }

    #[test]
    fn all_colours_are_rgb() {
        let colours = [
            BG,
            FG,
            ACCENT,
            DANGER,
            WARNING,
            SUCCESS,
            MUTED,
            BORDER,
            SURFACE,
            SELECTION,
            ACCENT_DIM,
            GIT_CLEAN,
            GIT_DIRTY_OR_ABSENT,
        ];
        for c in &colours {
            assert!(matches!(c, Color::Rgb(_, _, _)), "expected RGB: {c:?}");
        }
    }

    #[test]
    fn semantic_pairs_are_distinct() {
        assert_ne!(BG, FG);
        assert_ne!(ACCENT, MUTED);
        assert_ne!(DANGER, SUCCESS);
        assert_ne!(ACCENT, DANGER);
        assert_ne!(WARNING, SUCCESS);
        assert_ne!(SURFACE, SELECTION);
        assert_ne!(ACCENT, ACCENT_DIM);
        assert_ne!(GIT_CLEAN, GIT_DIRTY_OR_ABSENT);
    }

    // ── space tokens ─────────────────────────────────────────────────────────

    #[test]
    fn space_tokens_match_design_tokens_json() {
        // Docs/design-tokens/raccpack.tokens.json → space.semantic.*. Keeping
        // the numeric values here in sync is a hard contract (UI ↔ Desktop).
        assert_eq!(SPACE_SIDEBAR_WIDTH, 23, "sidebar-width");
        assert_eq!(SPACE_HEADER_HEIGHT, 1, "header-height");
        assert_eq!(SPACE_FOOTER_HEIGHT, 1, "footer-height");
        assert_eq!(SPACE_DETAIL_HEIGHT, 7, "detail-height");
        assert_eq!(SPACE_SIDEBAR_ACCENT_BAR, 2, "sidebar-accent-bar");
        assert_eq!(SPACE_ROW_ACCENT_BAR, 1, "row-accent-bar");
    }

    #[test]
    fn space_tokens_are_positive() {
        for value in [
            SPACE_SIDEBAR_WIDTH,
            SPACE_HEADER_HEIGHT,
            SPACE_FOOTER_HEIGHT,
            SPACE_DETAIL_HEIGHT,
            SPACE_SIDEBAR_ACCENT_BAR,
            SPACE_ROW_ACCENT_BAR,
        ] {
            assert!(value > 0, "space token must be positive, got {value}");
        }
    }

    #[test]
    fn detail_height_fits_labels_and_values() {
        // The strip must comfortably hold project/finding metadata lines
        // (title row + label/value pairs) inside the bordered panel.
        assert!(SPACE_DETAIL_HEIGHT >= 5, "strip must not be impractically thin");
    }

    // ── glyphs / placeholder ──────────────────────────────────────────────────

    #[test]
    fn git_glyphs_match_token_values() {
        assert_eq!(GIT_CLEAN_GLYPH, "●", "component.git.clean-glyph");
        assert_eq!(GIT_ABSENT_GLYPH, "·", "component.git.absent-glyph");
        assert_ne!(GIT_CLEAN_GLYPH, GIT_ABSENT_GLYPH);
    }

    #[test]
    fn empty_placeholder_is_middle_dot_not_hyphen() {
        // component.table.empty-placeholder mandates `·` (U+00B7), not `-`.
        assert_eq!(EMPTY_PLACEHOLDER, "·");
        assert_ne!(EMPTY_PLACEHOLDER, "-");
        assert_eq!(EMPTY_PLACEHOLDER.chars().count(), 1);
    }
}
