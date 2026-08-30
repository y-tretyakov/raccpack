//! Projects — Cards rendering mode.
//!
//! Lays the scanned projects out as a column grid of [`project_card`] panels.
//! The single selection index (shared with Table/Tree) is highlighted with a
//! brand border; arrows scroll the grid so the selection stays visible.
//! Read-only: never mutates application state.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::sniff::{ProjectRow, SniffScreenState};
use crate::theme;
use crate::ui::widgets::project_card;

/// Approximate width of one card column; fewer columns on narrow terminals.
const CARD_WIDTH: u16 = 26;
/// One spacer row between card rows.
const CARD_GAP: u16 = 1;
/// Top + bottom border rows that each card reserves.
const CARD_BORDER: u16 = 2;

/// Render the project cards grid into `area` (the top project region).
pub fn render(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" Projects ({}) — cards ", state.projects.len(),),
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.projects.is_empty() || inner.height < project_card::CARD_HEIGHT + CARD_BORDER {
        return;
    }
    render_grid(f, inner, state);
}

/// Lay visible cards into the bordered inner area, scrolling to keep the
/// selected index on screen and clipping at the edges.
fn render_grid(f: &mut Frame, area: Rect, state: &SniffScreenState) {
    let slot_height = project_card::CARD_HEIGHT + CARD_BORDER + CARD_GAP;
    let cols = (area.width.saturating_sub(1) / CARD_WIDTH).max(1);
    let visible_rows = (area.height / slot_height).max(1);

    let sel = state.table_state.selected().unwrap_or(0);
    let sel_row = (sel as u16) / cols;
    let scroll = sel_row.saturating_sub(visible_rows - 1);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(std::iter::repeat_n(Constraint::Fill(1), usize::from(cols)))
        .split(area);

    for (i, project) in state.projects.iter().enumerate() {
        let row = (i / usize::from(cols)) as u16;
        if row < scroll || row >= scroll + visible_rows {
            continue;
        }
        let col = i % usize::from(cols);
        let y = area.y.saturating_add((row - scroll) * slot_height);
        render_card(f, columns[col], y, project, i == sel);
    }
}

/// Render one bordered card; the selected one gets a brand border.
fn render_card(f: &mut Frame, column: Rect, y: u16, project: &ProjectRow, selected: bool) {
    let border_fg = if selected {
        theme::BRAND_PRIMARY
    } else {
        theme::BORDER
    };
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_fg));
    let card_area = Rect::new(
        column.x,
        y,
        column.width,
        project_card::CARD_HEIGHT + CARD_BORDER,
    );
    let inner = card_block.inner(card_area);
    f.render_widget(card_block, card_area);
    project_card::render(f, inner, project);
}

#[cfg(test)]
mod tests {
    use crate::app::sniff::{ProjectRow, ProjectsMode, SniffScreenState};
    use std::path::PathBuf;

    fn row(name: &str) -> ProjectRow {
        ProjectRow {
            name: name.to_string(),
            language: Some("Rust".into()),
            frameworks: vec!["Axum".into()],
            size_bytes: 1024 * 1024,
            is_git_repo: true,
            path: PathBuf::from(format!("/tmp/{name}")),
        }
    }

    fn sample_state(n: usize) -> SniffScreenState {
        SniffScreenState {
            projects: (0..n).map(|i| row(&format!("p{i}"))).collect(),
            table_state: ratatui::widgets::TableState::default(),
            ..Default::default()
        }
    }

    fn render_text(state: &mut SniffScreenState, cols: u16, rows: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(cols, rows);
        let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
        terminal
            .draw(|f| crate::ui::screens::sniff::render(f, f.area(), state))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn cards_mode_renders_projects_no_panic() {
        let mut state = sample_state(6);
        state.table_state.select(Some(0));
        state.mode = ProjectsMode::Cards;
        let text = render_text(&mut state, 80, 24);
        assert!(text.contains("Projects"));
        assert!(
            text.contains("p0"),
            "first project card present, got: {text}"
        );
    }

    #[test]
    fn cards_mode_without_selection_does_not_panic() {
        let mut state = sample_state(10);
        state.mode = ProjectsMode::Cards;
        let text = render_text(&mut state, 80, 24);
        assert!(!text.is_empty());
    }

    #[test]
    fn cards_mode_renders_on_narrow_terminal_single_column() {
        let mut state = sample_state(4);
        state.table_state.select(Some(0));
        state.mode = ProjectsMode::Cards;
        let text = render_text(&mut state, 40, 24);
        assert!(
            text.contains("p0"),
            "single column still renders, got: {text}"
        );
    }
}
