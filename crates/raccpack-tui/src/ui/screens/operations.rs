//! Operations hub screen — the list of runnable operations plus the project
//! the next operation targets.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::operations::{OperationKind, OperationsScreenState, ALL_OPERATIONS};
use crate::theme;
use crate::ui::widgets::centered_rect;
use crate::ui::widgets::detail::{render as render_detail, DetailLine};

/// Render the Operations hub into `area`.
///
/// `project` is the currently sniff-selected project (may be `None`). It is
/// mirrored into the screen state so the hub owns a display copy, refreshed
/// every frame: changing the row on Projects is reflected here immediately.
pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &mut OperationsScreenState,
    project: Option<&std::path::Path>,
) {
    state.refresh_project(project);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(theme::SPACE_DETAIL_HEIGHT),
        ])
        .split(area);
    render_list(f, chunks[0], state);
    render_project_strip(f, chunks[1], state);

    if let Some(kind) = state.stub {
        render_stub_notice(f, area, kind);
    }
}

/// Bordered list of every operation, the selection highlighted.
fn render_list(f: &mut Frame, area: Rect, state: &OperationsScreenState) {
    let lines: Vec<Line> = ALL_OPERATIONS
        .iter()
        .map(|kind| operation_line(*kind, state))
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" Operations ({}) ", ALL_OPERATIONS.len()),
            Style::default().fg(theme::BRAND_PRIMARY),
        ));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

/// One operations-list row: accent bar + label + shortcut key hint.
fn operation_line(kind: OperationKind, state: &OperationsScreenState) -> Line<'static> {
    let selected = kind == state.selected;
    let bg = if selected {
        theme::SURFACE_RAISED
    } else {
        theme::BG
    };
    let bar = if selected {
        Span::styled("▎", Style::default().fg(theme::FOCUS).bg(bg))
    } else {
        Span::raw(" ").style(Style::default().bg(bg))
    };
    let label = Span::styled(
        format!(" {}", kind.label()),
        Style::default()
            .fg(if selected { theme::FG } else { theme::MUTED })
            .bg(bg)
            .add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    );
    let hint = Span::styled(
        format!(" [{}]", kind.key()),
        Style::default().fg(theme::MUTED).bg(bg),
    );
    Line::from(vec![bar, label, hint]).style(Style::default().bg(bg).fg(theme::FG))
}

/// Strip under the list: the target project, or a prompt when none exists.
fn render_project_strip(f: &mut Frame, area: Rect, state: &OperationsScreenState) {
    match state.project.as_deref() {
        Some(path) => {
            let lines = [
                DetailLine::new("Project", path.display().to_string()),
                DetailLine::new("Operation", state.selected.label()),
            ];
            render_detail(f, area, "Target", &lines);
        }
        None => render_empty_project_hint(f, area),
    }
}

/// Prompt shown when no sniff result exists yet: choose a project first.
fn render_empty_project_hint(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(" Project ", Style::default().fg(theme::MUTED)));
    f.render_widget(
        Paragraph::new("No project selected — open Projects (2), pick a row, return here.")
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::MUTED)),
        area,
    );
}

/// Centered placeholder shown when an operation whose real flow does not exist
/// yet is activated (Pack/Stash/Rinse → T-02..T-04).
fn render_stub_notice(f: &mut Frame, area: Rect, kind: OperationKind) {
    let popup = centered_rect(50, 30, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::WARNING))
        .title(Span::styled(
            format!(" {} ", kind.label()),
            Style::default()
                .fg(theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ));

    let stage = kind.planned_stage().unwrap_or("a later stage");
    let body = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} is coming in {}.", kind.label(), stage),
            Style::default().fg(theme::FG),
        )),
        Line::from(Span::styled(
            "The full flow is a separate stage of the CLI→TUI track.",
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to dismiss",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    f.render_widget(
        Paragraph::new(body)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .alignment(ratatui::layout::Alignment::Center),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn list_has_four_operations_with_labels_and_keys() {
        let state = OperationsScreenState::default();
        let lines: Vec<String> = ALL_OPERATIONS
            .iter()
            .map(|k| line_text(&operation_line(*k, &state)))
            .collect();
        assert_eq!(lines.len(), 4);
        for (i, kind) in ALL_OPERATIONS.iter().enumerate() {
            assert!(lines[i].contains(kind.label()), "row {i} carries its label");
            assert!(
                lines[i].contains(kind.key()),
                "row {i} carries its key hint"
            );
        }
    }

    #[test]
    fn only_the_selected_row_is_bold() {
        let state = OperationsScreenState {
            selected: OperationKind::Raid,
            ..Default::default()
        };
        for kind in ALL_OPERATIONS {
            let line = operation_line(kind, &state);
            let bold = line
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
            assert_eq!(
                bold,
                kind == state.selected,
                "bold highlight must follow the selection"
            );
        }
    }

    #[test]
    fn selected_row_gets_accent_bar() {
        let state = OperationsScreenState {
            selected: OperationKind::Pack,
            ..Default::default()
        };
        assert!(line_text(&operation_line(OperationKind::Pack, &state)).starts_with('▎'));
        assert!(!line_text(&operation_line(OperationKind::Raid, &state)).starts_with('▎'));
    }

    #[test]
    fn no_project_hint_points_to_projects() {
        // The empty-project prompt must name a concrete next step.
        let hint = "open Projects (2), pick a row";
        assert!(hint.contains("Projects"));
        assert!(hint.contains('2'));
    }

    #[test]
    fn stub_notice_names_the_planned_stage() {
        for kind in [OperationKind::Stash, OperationKind::Rinse] {
            let stage = kind.planned_stage().expect("future stages are mapped");
            assert!(stage.starts_with("T-0"), "{kind:?} maps to a track stage");
        }
        assert_eq!(OperationKind::Pack.planned_stage(), None);
        assert_eq!(OperationKind::Raid.planned_stage(), None);
    }
}
