//! Sniff screen — project table with stack, size, git status + detail strip.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::sniff::ProjectsMode;
use crate::app::sniff::SniffScreenState;
use crate::theme;
use crate::ui::screens::{projects_cards, projects_tree};
use crate::ui::widgets::detail::{render as render_detail, DetailLine};
use crate::ui::widgets::format_bytes;

/// Render the sniff screen. The project area (Cards/Table/Tree) owns the top;
/// the detail strip (selected project metadata) sits below it. The chrome
/// lives in the global header/footer.
pub fn render(f: &mut Frame, area: Rect, state: &mut SniffScreenState) {
    if state.is_loading {
        render_loading(f, area, state);
    } else if let Some(error) = &state.error {
        render_error(f, area, error);
    } else if state.projects.is_empty() {
        render_empty(f, area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(theme::SPACE_DETAIL_HEIGHT),
            ])
            .split(area);
        let mode = state.mode;
        match mode {
            ProjectsMode::Cards => projects_cards::render(f, chunks[0], state),
            ProjectsMode::Table => render_table(f, chunks[0], state),
            ProjectsMode::Tree => projects_tree::render(f, chunks[0]),
        }
        render_project_detail(f, chunks[1], state);
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
        Cell::from(" "), // row accent bar
        Cell::from("Name").style(header_style()),
        Cell::from("Language").style(header_style()),
        Cell::from("Frameworks").style(header_style()),
        Cell::from("Size").style(header_style()),
        Cell::from("Git").style(header_style()),
    ]);

    let rows: Vec<Row> = state
        .projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let selected = state.table_state.selected() == Some(i);
            let bg = if selected {
                theme::SURFACE_RAISED
            } else if i % 2 == 0 {
                theme::SURFACE
            } else {
                theme::BG
            };

            let accent_bar = if selected {
                Cell::from(Span::styled("▎", Style::default().fg(theme::FOCUS).bg(bg)))
            } else {
                Cell::from(Span::raw(" ").style(Style::default().bg(bg)))
            };

            let language = project
                .language
                .as_deref()
                .unwrap_or(theme::EMPTY_PLACEHOLDER);
            let frameworks = if project.frameworks.is_empty() {
                theme::EMPTY_PLACEHOLDER.to_string()
            } else {
                project.frameworks.join(", ")
            };
            let size = format_bytes(project.size_bytes);
            let (git_glyph, git_fg) = git_glyph_for(project.is_git_repo);

            let style = Style::default().bg(bg).fg(theme::FG);
            Row::new(vec![
                accent_bar,
                Cell::from(project.name.clone()),
                Cell::from(language),
                Cell::from(frameworks),
                Cell::from(size),
                Cell::from(git_glyph).style(Style::default().fg(git_fg)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(theme::SPACE_ROW_ACCENT_BAR),
        Constraint::Percentage(23),
        Constraint::Percentage(14),
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
                    Style::default().fg(theme::BRAND_PRIMARY),
                )),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme::SURFACE_RAISED)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(table, area, &mut state.table_state);
}

/// Detail strip under the project table: selected project metadata.
fn render_project_detail(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let lines = match state.selected_project() {
        Some(project) => {
            let frameworks = if project.frameworks.is_empty() {
                String::new()
            } else {
                project.frameworks.join(", ")
            };
            let (git_glyph, git_fg) = git_glyph_for(project.is_git_repo);
            vec![
                DetailLine::new("Name", project.name.clone()),
                DetailLine::new("Language", project.language.clone().unwrap_or_default()),
                DetailLine::new("Frameworks", frameworks),
                DetailLine::muted("Path", project.path.display().to_string()),
                DetailLine {
                    label: "Git",
                    value: git_glyph.to_string(),
                    fg: git_fg,
                },
            ]
        }
        None => vec![DetailLine::new("Project", "")],
    };
    render_detail(f, area, "Selected project", &lines);
}

fn header_style() -> Style {
    Style::default()
        .fg(theme::BRAND_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

/// Glyph + foreground for the git-repository cell/line.
fn git_glyph_for(is_repo: bool) -> (&'static str, ratatui::style::Color) {
    if is_repo {
        (theme::GIT_CLEAN_GLYPH, theme::GIT_CLEAN)
    } else {
        (theme::GIT_ABSENT_GLYPH, theme::GIT_DIRTY_OR_ABSENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn git_glyphs_use_tokens() {
        let (clean, _) = git_glyph_for(true);
        assert_eq!(clean, theme::GIT_CLEAN_GLYPH, "repo present → clean glyph");
        let (absent, _) = git_glyph_for(false);
        assert_eq!(absent, theme::GIT_ABSENT_GLYPH, "no repo → absent glyph");
    }

    #[test]
    fn empty_cells_use_placeholder() {
        assert_eq!(theme::EMPTY_PLACEHOLDER, "·");
        assert_ne!(theme::EMPTY_PLACEHOLDER, "-");
    }
}
