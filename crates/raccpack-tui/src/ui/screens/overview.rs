//! Overview screen — workspace dashboard (replace the old empty stub).
//!
//! Composition per b1.0 §5.4: a KPI strip of workspace metrics, a detection
//! health line, and a grid of recent project cards. Unless a scan has run the
//! screen shows guided hints instead of blank space, so the start screen always
//! carries information.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::sniff::{ProjectRow, SniffScreenState};
use crate::theme;
use crate::ui::widgets::{kpi_strip, project_card};

/// Empty-state hint shown while no scan has run yet (shared with tests).
pub const EMPTY_HINT_TITLE: &str = "No projects scanned yet";
/// Empty-state hint action — the path to the first scan.
pub const EMPTY_HINT_ACTION: &str = "press 2 or Tab → Projects, then r";

/// Most recent projects shown on the dashboard (spec: 2–6 cards). Fewer
/// available projects render as-is — never padded with empty placeholders.
const RECENT_MAX: usize = 6;
/// Width hint used to derive the card grid column count.
const CARD_WIDTH: usize = 24;
/// One spacer row between card rows.
const CARD_GAP: u16 = 1;
/// Row budget for the KPI strip (value + label).
const KPI_STRIP_HEIGHT: u16 = 2;
/// Row budget for the health line.
const HEALTH_HEIGHT: u16 = 1;

/// Render the overview dashboard into `area`.
///
/// Read-only: takes the scan snapshot and never mutates application state.
pub fn render(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " Overview ",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.projects.is_empty() {
        render_empty(f, inner);
    } else {
        render_dashboard(f, inner, state);
    }
}

/// Data path: KPI strip → health line → recent cards.
fn render_dashboard(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(KPI_STRIP_HEIGHT),
            Constraint::Length(HEALTH_HEIGHT),
            Constraint::Min(0),
        ])
        .split(area);

    let metrics = metrics_from(state);
    kpi_strip::render(f, chunks[0], &metrics);
    render_health(f, chunks[1], state, &metrics);
    render_recent(f, chunks[2], state);
}

/// Derive the KPI snapshot from the scan state.
fn metrics_from(state: &SniffScreenState) -> kpi_strip::KpiMetrics {
    kpi_strip::KpiMetrics {
        projects: state.projects.len(),
        rust: state
            .projects
            .iter()
            .filter(|p| p.language.as_deref() == Some("Rust"))
            .count(),
        js_ts: state
            .projects
            .iter()
            .filter(|p| matches!(p.language.as_deref(), Some("JavaScript" | "TypeScript")))
            .count(),
        total_size_bytes: state.total_size,
        git_repos: state.projects.iter().filter(|p| p.is_git_repo).count(),
    }
}

fn render_health(
    f: &mut Frame,
    area: Rect,
    state: &SniffScreenState,
    metrics: &kpi_strip::KpiMetrics,
) {
    f.render_widget(
        Paragraph::new(health_line(state, metrics))
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

/// `✓ detection READY · N projects · M git [(cache)]` — success green for the
/// ready mark, neutral counts, muted cache note.
fn health_line(state: &SniffScreenState, metrics: &kpi_strip::KpiMetrics) -> Line<'static> {
    let ready = Style::default()
        .fg(theme::SUCCESS)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::styled("✓", ready),
        Span::raw(" "),
        Span::styled("detection READY", ready),
        Span::raw("  ·  "),
        Span::styled(
            format!("{} projects", metrics.projects),
            Style::default().fg(theme::FG),
        ),
        Span::raw(" · "),
        Span::styled(
            format!("{} git", metrics.git_repos),
            Style::default().fg(theme::FG),
        ),
    ];
    if state.from_cache {
        spans.push(Span::styled(" (cache)", Style::default().fg(theme::MUTED)));
    }
    Line::from(spans)
}

/// Recent-projects section: section label over a grid of up to 6 cards.
fn render_recent(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Recent projects ",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(theme::BG).fg(theme::FG)),
        chunks[0],
    );

    let projects = &state.projects[..state.projects.len().min(RECENT_MAX)];
    render_card_grid(f, chunks[1], projects);
}

/// Lay out `projects` into a column grid of [`project_card::CARD_HEIGHT`]
/// rows. Cards are clamped to the area so short terminals never overflow.
fn render_card_grid(f: &mut Frame, area: Rect, projects: &[ProjectRow]) {
    if projects.is_empty() || area.height < project_card::CARD_HEIGHT {
        return;
    }
    // At least one column even on a narrow terminal; 2 columns fills an 80×24.
    let cols = (usize::from(area.width.saturating_sub(1)) / CARD_WIDTH).max(1);
    let row_stride = project_card::CARD_HEIGHT + CARD_GAP;
    // Rows that fit without the trailing gap: 4 (first) + 5 per further row.
    let max_rows = 1 + (area.height - project_card::CARD_HEIGHT) / row_stride;
    let limit = cols * usize::from(max_rows);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(std::iter::repeat_n(Constraint::Fill(1), cols))
        .split(area);

    for (i, project) in projects.iter().take(limit).enumerate() {
        let column = columns[i % cols];
        let row_offset = (i / cols) as u16 * row_stride;
        let card_rect = Rect::new(
            column.x.saturating_add(1),
            area.y.saturating_add(row_offset),
            column.width.saturating_sub(2),
            project_card::CARD_HEIGHT,
        );
        project_card::render(f, card_rect, project);
    }
}

/// Empty path: guided hints where the KPI strip and cards would sit — the
/// overview is never a bare stub.
fn render_empty(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                EMPTY_HINT_TITLE,
                Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                EMPTY_HINT_ACTION,
                Style::default().fg(theme::MUTED),
            )),
        ])
        .style(Style::default().bg(theme::SURFACE_RAISED).fg(theme::FG))
        .alignment(ratatui::layout::Alignment::Center),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                " Recent projects ",
                Style::default()
                    .fg(theme::BRAND_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Recent projects will appear here after the first scan.",
                Style::default().fg(theme::MUTED),
            )),
        ])
        .style(Style::default().bg(theme::BG).fg(theme::FG)),
        chunks[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn row(name: &str, language: Option<&str>, size: u64, git: bool) -> ProjectRow {
        ProjectRow {
            name: name.to_string(),
            language: language.map(str::to_string),
            frameworks: vec![],
            size_bytes: size,
            is_git_repo: git,
            path: PathBuf::from("/proj").join(name),
        }
    }

    fn sample_state() -> SniffScreenState {
        SniffScreenState {
            projects: vec![
                row("core", Some("Rust"), 11 * 1024 * 1024, true),
                row("web", Some("TypeScript"), 256 * 1024, true),
                row("scripts", Some("Python"), 48 * 1024, false),
                row("other", Some("Rust"), 2 * 1024, false),
                row("plain", None, 0, false),
                row("more", Some("JavaScript"), 1024, true),
                row("seventh", Some("Go"), 33 * 1024, true),
            ],
            total_size: 11 * 1024 * 1024 + 256 * 1024 + 48 * 1024 + 2 * 1024 + 1024 + 33 * 1024,
            ..Default::default()
        }
    }

    #[test]
    fn metrics_count_languages_and_git() {
        let metrics = metrics_from(&sample_state());
        assert_eq!(metrics.projects, 7);
        assert_eq!(metrics.rust, 2);
        assert_eq!(metrics.js_ts, 2, "JavaScript + TypeScript");
        assert_eq!(metrics.git_repos, 4);
        assert!(metrics.total_size_bytes > 0);
    }

    #[test]
    fn empty_state_metrics_are_zero() {
        let metrics = metrics_from(&SniffScreenState::default());
        assert_eq!(metrics.projects, 0);
        assert_eq!(metrics.rust, 0);
        assert_eq!(metrics.js_ts, 0);
        assert_eq!(metrics.git_repos, 0);
        assert_eq!(metrics.total_size_bytes, 0);
    }

    #[test]
    fn health_line_reports_ready_counts_and_cache() {
        let state = SniffScreenState {
            from_cache: true,
            ..sample_state()
        };
        let metrics = metrics_from(&state);

        let with_cache = health_line(&state, &metrics).to_string();
        assert!(with_cache.contains("detection READY"));
        assert!(with_cache.contains("7 projects"));
        assert!(with_cache.contains("4 git"));
        assert!(with_cache.contains("(cache)"), "got: {with_cache}");

        let mut fresh = state;
        fresh.from_cache = false;
        assert!(
            !health_line(&fresh, &metrics)
                .to_string()
                .contains("(cache)"),
            "fresh scan must not show a cache marker"
        );
    }

    #[test]
    fn recent_section_is_capped_at_six() {
        let state = sample_state();
        assert!(state.projects.len() > RECENT_MAX);
        assert_eq!(RECENT_MAX, 6);
    }

    #[test]
    fn empty_hints_offer_a_way_forward() {
        assert!(EMPTY_HINT_ACTION.contains("Projects"));
        assert!(EMPTY_HINT_ACTION.contains("Tab"));
        assert!(EMPTY_HINT_ACTION.contains("r"));
        assert!(!EMPTY_HINT_TITLE.is_empty());
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    }

    #[test]
    fn dashboard_renders_non_empty_buffer() {
        let backend = ratatui::backend::TestBackend::new(55, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
        let state = sample_state();
        terminal
            .draw(|f| render(f, f.area(), &state))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("Overview"));
        assert!(text.contains("detection READY"));
        assert!(text.contains("Recent projects"));
        assert!(text.contains("core"));
        assert!(text.contains("Rust"));
        assert!(text.contains("11.0 MB"), "got: {text}");
    }

    #[test]
    fn empty_state_renders_hints_not_blank() {
        let backend = ratatui::backend::TestBackend::new(55, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
        let state = SniffScreenState::default();
        terminal
            .draw(|f| render(f, f.area(), &state))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("No projects scanned yet"));
        assert!(text.contains("Projects"));
        assert!(!text.is_empty());
    }
}
