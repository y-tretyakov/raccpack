//! Sniff screen — project table with stack, size, git status.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::sniff::SniffScreenState;

#[cfg(test)]
use crate::app::sniff::ProjectRow;
use crate::ui::theme;

/// Render the sniff screen.
pub fn render(f: &mut Frame, area: Rect, state: &mut SniffScreenState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // table
            Constraint::Length(2), // status
        ])
        .split(area);

    render_header(f, chunks[0], state);
    render_table(f, chunks[1], state);
    render_status(f, chunks[2], state);
}

fn render_header(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let root_display = state.scan_root.display().to_string();
    let project_count = state.projects.len();

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "  Projects",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!("{} projects", project_count),
                Style::default().fg(theme::FG),
            ),
            Span::raw(" │ "),
            Span::styled(
                format_bytes(state.total_size).to_string(),
                Style::default().fg(theme::MUTED),
            ),
            Span::raw(" │ "),
            Span::styled(root_display, Style::default().fg(theme::MUTED)),
        ]),
        Line::from(vec![Span::styled(
            "  [r] refresh  [o] change root  [j/k] navigate  [Enter] dig  [Esc] back",
            Style::default().fg(theme::MUTED),
        )]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(" Sniff ", Style::default().fg(theme::ACCENT)));

    f.render_widget(
        ratatui::widgets::Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_table(f: &mut Frame, area: Rect, state: &mut SniffScreenState) {
    if state.is_loading {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .title(Span::styled(
                " Scanning… ",
                Style::default().fg(theme::WARNING),
            ));

        let text = if let Some(progress) = &state.progress {
            format!("{}% — {}", progress.percent, progress.message)
        } else {
            "Scanning projects…".to_string()
        };

        f.render_widget(
            ratatui::widgets::Paragraph::new(text)
                .block(block)
                .style(Style::default().bg(theme::BG).fg(theme::FG))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    if let Some(error) = &state.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::DANGER))
            .title(Span::styled(" Error ", Style::default().fg(theme::DANGER)));

        f.render_widget(
            ratatui::widgets::Paragraph::new(error.as_str())
                .block(block)
                .style(Style::default().bg(theme::BG).fg(theme::DANGER))
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true }),
            area,
        );
        return;
    }

    if state.projects.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .title(Span::styled(
                " No Projects ",
                Style::default().fg(theme::MUTED),
            ));

        f.render_widget(
            ratatui::widgets::Paragraph::new("Press [r] to scan for projects")
                .block(block)
                .style(Style::default().bg(theme::BG).fg(theme::MUTED))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("Name").style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Language").style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Frameworks").style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Size").style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Git").style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let rows: Vec<Row> = state
        .projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let style = if state.table_state.selected() == Some(i) {
                Style::default().bg(theme::SELECTION).fg(theme::FG)
            } else if i % 2 == 0 {
                Style::default().bg(theme::SURFACE).fg(theme::FG)
            } else {
                Style::default().bg(theme::BG).fg(theme::FG)
            };

            let language = project.language.as_deref().unwrap_or("-");
            let frameworks = if project.frameworks.is_empty() {
                "-".to_string()
            } else {
                project.frameworks.join(", ")
            };
            let size = format_bytes(project.size_bytes);
            let git = if project.is_git_repo { "✓" } else { "✗" };

            Row::new(vec![
                Cell::from(project.name.clone()),
                Cell::from(language),
                Cell::from(frameworks),
                Cell::from(size),
                Cell::from(git),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(Span::styled(
                    format!(" Projects ({}) ", state.projects.len()),
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

fn render_status(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let cache_indicator = if state.from_cache {
        " (from cache)"
    } else {
        ""
    };

    let text = if state.is_loading {
        "Loading…".to_string()
    } else if state.error.is_some() {
        "Error — press [r] to retry".to_string()
    } else if state.projects.is_empty() {
        "No projects found — press [r] to scan".to_string()
    } else {
        let timestamp = state
            .last_refresh
            .map(|t| humantime::format_rfc3339(t).to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!("Last refresh: {}{cache_indicator}", timestamp)
    };

    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .style(Style::default().bg(theme::BG).fg(theme::MUTED))
            .alignment(ratatui::layout::Alignment::Left),
        area,
    );
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn format_bytes_test() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn project_row_creation() {
        let row = ProjectRow {
            name: "test".into(),
            language: Some("Rust".into()),
            frameworks: vec!["Axum".into()],
            size_bytes: 1024 * 1024,
            is_git_repo: true,
            path: PathBuf::from("/tmp/test"),
        };
        assert_eq!(row.name, "test");
        assert_eq!(row.language, Some("Rust".into()));
        assert_eq!(row.frameworks, vec!["Axum"]);
        assert_eq!(row.size_bytes, 1024 * 1024);
        assert!(row.is_git_repo);
    }
}
