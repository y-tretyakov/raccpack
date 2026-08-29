//! Dig screen — findings table with severity, path, kind, git status + detail strip.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use raccpack_core::domain::SensitiveRisk;

use crate::app::dig::{DigScreenState, FindingRow};
use crate::ui::theme;
use crate::ui::widgets::detail::{render as render_detail, DetailLine};

/// Render the dig screen. The findings table owns the top area; the detail
/// strip (selected finding metadata) sits below it.
pub fn render(f: &mut Frame, area: Rect, state: &mut DigScreenState) {
    if state.project.is_none() {
        render_no_scope(f, area);
    } else if state.is_loading {
        render_loading(f, area, state);
    } else if let Some(error) = &state.error {
        render_error(f, area, error);
    } else if state.findings.is_empty() {
        render_empty(f, area, state);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(theme::SPACE_DETAIL_HEIGHT),
            ])
            .split(area);
        render_table(f, chunks[0], state);
        render_finding_detail(f, chunks[1], state);
    }
}

fn render_no_scope(f: &mut Frame, area: Rect) {
    render_message(
        f,
        area,
        " Findings ",
        theme::ACCENT,
        "No project selected — press Enter on a project on the Projects screen.",
    );
}

fn render_loading(f: &mut Frame, area: Rect, state: &DigScreenState) {
    let text = state
        .progress
        .as_ref()
        .map(|p| format!("{}% — {}", p.percent, p.message))
        .unwrap_or_else(|| "Digging for secrets…".to_string());
    render_message(f, area, " Digging… ", theme::WARNING, &text);
}

fn render_error(f: &mut Frame, area: Rect, error: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DANGER))
        .title(Span::styled(" Error ", Style::default().fg(theme::DANGER)));

    f.render_widget(
        Paragraph::new(error)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::DANGER))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_empty(f: &mut Frame, area: Rect, state: &DigScreenState) {
    let (title, message) = if !state.all_findings.is_empty() {
        (
            format!(" Nothing at {} ", state.min_risk.label()),
            format!(
                "All {} findings fall below the {} filter — press f to widen.",
                state.all_findings.len(),
                state.min_risk.label()
            ),
        )
    } else {
        (
            " No Findings ".to_string(),
            "No sensitive files found in this project.".to_string(),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(title, Style::default().fg(theme::MUTED)));

    f.render_widget(
        Paragraph::new(message)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::MUTED))
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

/// Shared bordered status paragraph for the transient dig screen states.
fn render_message(
    f: &mut Frame,
    area: Rect,
    title: &str,
    color: ratatui::style::Color,
    text: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(title, Style::default().fg(color)));

    f.render_widget(
        Paragraph::new(text)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_table(f: &mut Frame, area: Rect, state: &mut DigScreenState) {
    let header = Row::new(vec![
        Cell::from(" "), // row accent bar
        Cell::from("Risk").style(header_style()),
        Cell::from("Path").style(header_style()),
        Cell::from("Kind").style(header_style()),
        Cell::from("Git").style(header_style()),
    ]);

    let rows: Vec<Row> = state
        .findings
        .iter()
        .enumerate()
        .map(|(i, finding)| {
            let selected = state.table_state.selected() == Some(i);
            let bg = if selected {
                theme::SELECTION
            } else if i % 2 == 0 {
                theme::SURFACE
            } else {
                theme::BG
            };
            finding_row(finding, selected, bg)
        })
        .collect();

    let widths = [
        Constraint::Length(theme::SPACE_ROW_ACCENT_BAR),
        Constraint::Percentage(16),
        Constraint::Percentage(39),
        Constraint::Percentage(30),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(Span::styled(
                    format!(
                        " Findings ({}) · masked · {} ",
                        state.findings.len(),
                        state.min_risk.label()
                    ),
                    Style::default().fg(theme::ACCENT),
                )),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme::SELECTION)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(table, area, &mut state.table_state);
}

/// Build one findings-table row. The selected row gets an accent bar and the
/// highlight background; the Risk cell carries the severity color.
fn finding_row(finding: &FindingRow, selected: bool, bg: ratatui::style::Color) -> Row<'_> {
    let accent_bar = if selected {
        Cell::from(Span::styled("▎", Style::default().fg(theme::ACCENT).bg(bg)))
    } else {
        Cell::from(Span::raw(" ").style(Style::default().bg(bg)))
    };

    let (git_glyph, git_fg) = git_cell_for(&finding.git_status);

    Row::new(vec![
        accent_bar,
        Cell::from(Span::styled(
            finding.risk.as_str(),
            Style::default().fg(risk_fg(finding.risk)),
        )),
        Cell::from(Span::styled(
            finding.path.display().to_string(),
            Style::default().fg(theme::FG),
        )),
        Cell::from(Span::styled(
            kind_or_placeholder(&finding.kind),
            Style::default().fg(theme::FG),
        )),
        Cell::from(git_glyph).style(Style::default().fg(git_fg)),
    ])
    .style(Style::default().bg(bg).fg(theme::FG))
}

/// Detail strip under the findings table: selected finding metadata.
fn render_finding_detail(f: &mut Frame, area: Rect, state: &DigScreenState) {
    let lines = match (state.selected_finding(), &state.project) {
        (Some(finding), Some(project)) => vec![
            DetailLine {
                label: "Risk",
                value: finding.risk.as_str().to_string(),
                fg: risk_fg(finding.risk),
            },
            DetailLine::muted("Path", finding.path.display().to_string()),
            DetailLine::new("Kind", kind_or_placeholder(&finding.kind).to_string()),
            DetailLine::new("Git", git_or_placeholder(&finding.git_status).to_string()),
            DetailLine::muted("Project", project.display().to_string()),
        ],
        _ => vec![DetailLine::new("Finding", "")],
    };
    render_detail(f, area, "Finding details", &lines);
}

fn header_style() -> Style {
    Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::BOLD)
}

/// Foreground for a risk level (severity ramp).
pub fn risk_fg(risk: SensitiveRisk) -> ratatui::style::Color {
    match risk {
        SensitiveRisk::Critical => theme::DANGER,
        SensitiveRisk::High => theme::WARNING,
        SensitiveRisk::Medium => theme::FG,
        SensitiveRisk::Low => theme::MUTED,
    }
}

/// Git glyph + color for a git-status string; unknown status is neutral.
fn git_cell_for(status: &str) -> (&'static str, ratatui::style::Color) {
    match status {
        "tracked" => (theme::GIT_CLEAN_GLYPH, theme::GIT_CLEAN),
        _ => (theme::GIT_ABSENT_GLYPH, theme::GIT_DIRTY_OR_ABSENT),
    }
}

fn kind_or_placeholder(kind: &str) -> &str {
    if kind.is_empty() {
        theme::EMPTY_PLACEHOLDER
    } else {
        kind
    }
}

fn git_or_placeholder(status: &str) -> &str {
    if status.is_empty() {
        theme::EMPTY_PLACEHOLDER
    } else {
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_foregrounds_are_distinct_per_level() {
        let colors = [
            SensitiveRisk::Low,
            SensitiveRisk::Medium,
            SensitiveRisk::High,
            SensitiveRisk::Critical,
        ]
        .map(risk_fg);
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "risk {i} vs {j} must differ");
            }
        }
    }

    #[test]
    fn tracked_git_uses_clean_glyph() {
        assert_eq!(
            git_cell_for("tracked"),
            (theme::GIT_CLEAN_GLYPH, theme::GIT_CLEAN)
        );
    }

    #[test]
    fn other_git_statuses_are_neutral() {
        for status in ["untracked", "ignored", "modified", ""] {
            assert_eq!(
                git_cell_for(status),
                (theme::GIT_ABSENT_GLYPH, theme::GIT_DIRTY_OR_ABSENT),
                "status {status:?} must render neutral"
            );
        }
    }

    #[test]
    fn empty_kind_and_git_become_placeholder() {
        assert_eq!(kind_or_placeholder(""), theme::EMPTY_PLACEHOLDER);
        assert_eq!(kind_or_placeholder(".env"), ".env");
        assert_eq!(git_or_placeholder(""), theme::EMPTY_PLACEHOLDER);
        assert_eq!(git_or_placeholder("tracked"), "tracked");
    }
}
