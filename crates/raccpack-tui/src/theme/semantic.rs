//! Semantic theme tokens — meanings built from primitive hex, plus the layout
//! space tokens and component glyphs.

use ratatui::style::Color;

use crate::theme::primitive;

/// Background. ← color.semantic.bg
pub const BG: Color = primitive::BG;

/// Base surface. ← color.semantic.surface
pub const SURFACE: Color = primitive::SURFACE;

/// Raised surface — selection wash and active rows. ← color.semantic.surface-raised
pub const SURFACE_RAISED: Color = primitive::SURFACE_RAISED;

/// Borders and rules. ← color.semantic.border
pub const BORDER: Color = primitive::BORDER;

/// Primary text. ← color.semantic.text
pub const TEXT: Color = primitive::TEXT;

/// Secondary text, hints, empty placeholders. ← color.semantic.muted
pub const MUTED: Color = primitive::MUTED;

/// Brand orange — focus / primary action / identity. ← color.semantic.brand-primary
pub const BRAND_PRIMARY: Color = primitive::BRAND_PRIMARY;

/// Bright brand — hover / strong emphasis. ← color.semantic.brand-bright
pub const BRAND_BRIGHT: Color = primitive::BRAND_BRIGHT;

/// Dim brand — quiet brand accents. ← color.semantic.brand-dim
pub const BRAND_DIM: Color = primitive::BRAND_DIM;

/// Success / healthy. ← color.semantic.success
pub const SUCCESS: Color = primitive::SUCCESS;

/// Warning / attention. ← color.semantic.warning
pub const WARNING: Color = primitive::WARNING;

/// Danger / error / destructive. ← color.semantic.danger
pub const DANGER: Color = primitive::DANGER;

/// Info / technology. ← color.semantic.info
pub const INFO: Color = primitive::INFO;

/// Analysis / detection / DAG. ← color.semantic.analysis
pub const ANALYSIS: Color = primitive::ANALYSIS;

/// Primary foreground — legacy alias for `TEXT` used across screens.
pub const FG: Color = primitive::TEXT;

/// Git repo present. ← color.semantic.git-clean (equals SUCCESS)
pub const GIT_CLEAN: Color = primitive::SUCCESS;

/// Git dirty or absent — not a git repo or neutral absent mark. ← color.semantic.git-dirty-or-absent (equals MUTED)
pub const GIT_DIRTY_OR_ABSENT: Color = primitive::MUTED;

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
    fn git_clean_is_success() {
        assert_eq!(GIT_CLEAN, SUCCESS);
    }

    #[test]
    fn git_dirty_or_absent_is_muted() {
        assert_eq!(GIT_DIRTY_OR_ABSENT, MUTED);
    }

    #[test]
    fn fg_is_text() {
        assert_eq!(FG, TEXT);
    }

    #[test]
    fn all_colours_are_rgb() {
        let colours = [
            BG,
            SURFACE,
            SURFACE_RAISED,
            BORDER,
            TEXT,
            MUTED,
            BRAND_PRIMARY,
            BRAND_BRIGHT,
            BRAND_DIM,
            SUCCESS,
            WARNING,
            DANGER,
            INFO,
            ANALYSIS,
            FG,
            GIT_CLEAN,
            GIT_DIRTY_OR_ABSENT,
        ];
        for c in &colours {
            assert!(matches!(c, Color::Rgb(_, _, _)), "expected RGB: {c:?}");
        }
    }

    #[test]
    fn semantic_pairs_are_distinct() {
        assert_ne!(BG, TEXT);
        assert_ne!(BRAND_PRIMARY, MUTED);
        assert_ne!(DANGER, SUCCESS);
        assert_ne!(BRAND_PRIMARY, DANGER);
        assert_ne!(WARNING, SUCCESS);
        assert_ne!(SURFACE, SURFACE_RAISED);
        assert_ne!(BRAND_PRIMARY, BRAND_BRIGHT);
        assert_ne!(BRAND_PRIMARY, BRAND_DIM);
        assert_ne!(GIT_CLEAN, GIT_DIRTY_OR_ABSENT);
        assert_ne!(INFO, ANALYSIS);
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
    }
}
