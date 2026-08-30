//! Reusable widgets shared across screens (detail strip, sidebar, KPI tiles,
//! project cards, …) plus the widget-level formatting helpers.

pub mod activity;
pub mod detail;
pub mod kpi_strip;
pub mod project_card;
pub mod sidebar;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;

use crate::theme;

/// Compute a centered rectangle as a percentage of the parent.
///
/// Shared by overlays (help, raid modal) so every popup centers the same way.
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Format bytes as a human-readable string (`B`/`KB`/`MB`/`GB`/`TB`).
///
/// Single source of truth for size rendering — the sniff table, the overview
/// KPI strip and the project cards all use this helper so sizes never drift.
/// Values below 1 KiB print as whole bytes; from KB up one decimal is kept
/// (`1.0 KB`, `11.5 MB`).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

/// Accent colour for a detected language (b1.0 §3 language→colour map).
///
/// Optional accent dots — the label text always carries the meaning, colour is
/// an enhancement. Unknown languages map to muted so only known ecosystems get
/// a colour accent.
pub fn language_accent(language: &str) -> Color {
    match language {
        "Rust" => theme::BRAND_PRIMARY,
        "JavaScript" => theme::WARNING,
        "TypeScript" => theme::INFO,
        "Python" => theme::INFO,
        "Go" => theme::INFO,
        "C#" => theme::ANALYSIS,
        "Java" => theme::DANGER,
        _ => theme::MUTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_is_smaller_than_parent() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        assert!(child.width < parent.width);
        assert!(child.height < parent.height);
    }

    #[test]
    fn centered_rect_has_positive_dimensions() {
        let parent = Rect::new(0, 0, 80, 24);
        let child = centered_rect(60, 85, parent);
        assert!(child.width > 0);
        assert!(child.height > 0);
    }

    #[test]
    fn centered_rect_is_centered() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        let left_margin = child.x.saturating_sub(parent.x);
        let right_margin = (parent.x + parent.width).saturating_sub(child.x + child.width);
        // Margins should be equal (±1 for rounding)
        assert!((left_margin as i16 - right_margin as i16).unsigned_abs() <= 1);
    }

    #[test]
    fn format_bytes_renders_units_and_decimals() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024u64.pow(4)), "1.0 TB");
    }

    #[test]
    fn language_accent_maps_known_ecosystems_only() {
        assert_eq!(language_accent("Rust"), theme::BRAND_PRIMARY);
        assert_eq!(language_accent("JavaScript"), theme::WARNING);
        assert_eq!(language_accent("TypeScript"), theme::INFO);
        assert_eq!(language_accent("Python"), theme::INFO);
        assert_eq!(language_accent("Go"), theme::INFO);
        assert_eq!(language_accent("C#"), theme::ANALYSIS);
        assert_eq!(language_accent("Java"), theme::DANGER);
        assert_eq!(language_accent("Brainfuck"), theme::MUTED);
        assert_eq!(language_accent(""), theme::MUTED);
    }
}
