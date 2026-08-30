//! KPI strip — a row of workspace-metric tiles for the overview dashboard.
//!
//! Five horizontal tiles: projects, Rust, JS/TS, total size, git repos. Each
//! tile shows its value in front (bold) with a muted label underneath, on a
//! `surface_raised` background. The Rust and JS/TS tiles carry language accents
//! (b1.0 §3) so the brand and technology intents read at a glance.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::{format_bytes, language_accent};

/// Snapshot of the five workspace metrics the strip renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KpiMetrics {
    /// Detected projects count.
    pub projects: usize,
    /// Rust projects (brand-accented tile).
    pub rust: usize,
    /// JavaScript + TypeScript projects (technology-accented tile).
    pub js_ts: usize,
    /// Total scanned size in bytes (rendered human-readable).
    pub total_size_bytes: u64,
    /// Projects that are git repositories.
    pub git_repos: usize,
}

/// One metric tile: value in front, muted label underneath.
struct Tile<'a> {
    value: String,
    label: &'a str,
    accent: ratatui::style::Color,
}

/// Render the KPI strip into `area`: a raised background band with one
/// centered tile per column.
pub fn render(f: &mut Frame, area: Rect, metrics: &KpiMetrics) {
    f.render_widget(
        Paragraph::new(Span::raw("")).style(Style::default().bg(theme::SURFACE_RAISED)),
        area,
    );

    let tiles = build_tiles(metrics);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(std::iter::repeat_n(Constraint::Fill(1), tiles.len()))
        .split(area);

    for (tile, column) in tiles.iter().zip(columns.iter()) {
        render_tile(f, *column, tile);
    }
}

fn render_tile(f: &mut Frame, area: Rect, tile: &Tile<'_>) {
    let value = Span::styled(
        tile.value.clone(),
        Style::default()
            .fg(tile.accent)
            .add_modifier(Modifier::BOLD),
    );
    let label = Span::styled(tile.label, Style::default().fg(theme::MUTED));

    f.render_widget(
        Paragraph::new(Line::from(vec![value, label]))
            .style(Style::default().bg(theme::SURFACE_RAISED))
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

fn build_tiles(metrics: &KpiMetrics) -> Vec<Tile<'static>> {
    vec![
        Tile {
            value: metrics.projects.to_string(),
            label: "projects",
            accent: theme::FG,
        },
        Tile {
            value: metrics.rust.to_string(),
            label: "Rust",
            accent: language_accent("Rust"),
        },
        Tile {
            value: metrics.js_ts.to_string(),
            label: "JS/TS",
            // Combined JS/TS tile uses the technology/information intent.
            accent: theme::INFO,
        },
        Tile {
            value: format_bytes(metrics.total_size_bytes),
            label: "total size",
            accent: theme::FG,
        },
        Tile {
            value: metrics.git_repos.to_string(),
            label: "git repos",
            accent: theme::FG,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_the_five_metrics_in_order() {
        let metrics = KpiMetrics {
            projects: 6,
            rust: 3,
            js_ts: 2,
            total_size_bytes: 1024 * 1024,
            git_repos: 5,
        };
        let tiles = build_tiles(&metrics);
        assert_eq!(tiles.len(), 5);
        assert_eq!(tiles[0].value, "6");
        assert_eq!(tiles[0].label, "projects");
        assert_eq!(tiles[1].value, "3");
        assert_eq!(tiles[1].label, "Rust");
        assert_eq!(tiles[2].value, "2");
        assert_eq!(tiles[2].label, "JS/TS");
        assert_eq!(tiles[3].value, "1.0 MB");
        assert_eq!(tiles[3].label, "total size");
        assert_eq!(tiles[4].value, "5");
        assert_eq!(tiles[4].label, "git repos");
    }

    #[test]
    fn rust_and_js_ts_tiles_carry_language_accents() {
        let metrics = KpiMetrics {
            rust: 1,
            js_ts: 1,
            ..Default::default()
        };
        let tiles = build_tiles(&metrics);
        assert_eq!(tiles[1].accent, theme::BRAND_PRIMARY, "Rust → brand");
        assert_eq!(tiles[2].accent, theme::INFO, "JS/TS → technology");
        assert_eq!(tiles[0].accent, theme::FG, "plain tiles use default fg");
        assert_eq!(tiles[3].accent, theme::FG, "total size uses default fg");
    }

    #[test]
    fn zero_metrics_render_zero_values() {
        let metrics = KpiMetrics::default();
        let tiles = build_tiles(&metrics);
        for tile in &tiles[0..3] {
            assert_eq!(
                tile.value, "0",
                "counts must render 0, got {:?}",
                tile.value
            );
        }
        assert_eq!(tiles[3].value, "0 B", "empty total size must stay readable");
        assert_eq!(tiles[4].value, "0");
    }
}
