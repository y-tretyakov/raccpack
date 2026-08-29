//! Sniff screen — project table with stack, size, git status.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::sniff::SniffScreenState;
use crate::ui::theme;

/// Render the sniff screen. The table (or a loading/error/empty placeholder)
/// owns the whole area; the chrome lives in the global header/footer.
pub fn render(f: &mut Frame, area: Rect, state: &mut SniffScreenState) {
    if state.is_loading {
        render_loading(f, area, state);
    } else if let Some(error) = &state.error {
        render_error(f, area, error);
    } else if state.projects.is_empty() {
        render_empty(f, area);
    } else {
        render_table(f, area, state);
    }
}

fn render_loading(f: &mut Frame, area: Rect, state: &SniffScreenState) {
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
}

fn render_error(f: &mut Frame, area: Rect, error: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DANGER))
        .title(Span::styled(" Error ", Style::default().fg(theme::DANGER)));

    f.render_widget(
        ratatui::widgets::Paragraph::new(error)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::DANGER))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true }),
        area,
    );
}

fn render_empty(f: &mut Frame, area: Rect) {
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
}

fn render_table(f: &mut Frame, area: Rect, state: &mut SniffScreenState) {
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
        let row = crate::app::sniff::ProjectRow {
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
