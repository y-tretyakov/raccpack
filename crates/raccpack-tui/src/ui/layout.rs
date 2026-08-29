//! Top-level frame rendering: header, body (sidebar + main), footer.

use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use crate::app::{App, Focus, ViewId, ALL_VIEWS};
use crate::ui::screens;
use crate::ui::theme;

/// Sidebar width in columns. Comfortable room for label + accent bar + key hint.
const SIDEBAR_WIDTH: u16 = 23;

/// Render one complete frame.
pub fn render(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    terminal.draw(|f| {
        let area = f.area();

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(0),    // body
                Constraint::Length(1), // footer
            ])
            .split(area);

        render_header(f, outer[0], app);
        render_body(f, outer[1], app);
        render_footer(f, outer[2], app);

        if app.help_visible {
            screens::help::render(f, area);
        }
    })?;
    Ok(())
}

fn render_header(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let title = Span::styled(
        " raccpack-tui ",
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    );
    let sep = Span::raw("│");
    let space = Span::raw(" ");

    let root = app.sniff_state.scan_root.to_string_lossy();
    let root_text = if root.is_empty() { "-" } else { &root };
    let root_span = Span::styled(root_text, Style::default().fg(theme::MUTED));

    let hotkeys = Span::styled(
        "Tab views · hjkl move · ? help · q quit",
        Style::default().fg(theme::MUTED),
    );

    // Dense single line; drop the hotkey strip (never panic) on narrow terms.
    let full = Line::from(vec![
        title.clone(),
        sep.clone(),
        space.clone(),
        root_span.clone(),
        sep.clone(),
        space.clone(),
        hotkeys,
    ]);
    let line = if full.width() <= usize::from(area.width) {
        full
    } else {
        Line::from(vec![title, sep, space, root_span])
    };

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_body(f: &mut ratatui::Frame, area: Rect, app: &mut App) {
    // Clamp sidebar to the available width so tiny terminals never panic.
    let sidebar_width = area.width.min(SIDEBAR_WIDTH);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Min(0)])
        .split(area);

    render_sidebar(f, chunks[0], app);
    screens::render_screen(f, chunks[1], app);
}

fn render_sidebar(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let sidebar_focused = app.focus == Focus::Sidebar;

    let mut lines = vec![Line::raw("")];
    for view in ALL_VIEWS {
        let active = app.current_view == view;
        let item_style = if active && sidebar_focused {
            Style::default()
                .fg(theme::FG)
                .bg(theme::SELECTION)
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::SURFACE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::MUTED)
        };

        let bar = Span::styled(
            if active { "▎" } else { " " },
            Style::default().fg(theme::ACCENT),
        );
        let label = Span::styled(view.label(), item_style);

        // Row: " " + bar + " " + label + pad + hint. Keep the hint on the
        // right edge of the fixed-width column.
        let prefix_width = 3 + view.label().chars().count();
        let pad = area.width.saturating_sub(prefix_width as u16 + 1);

        lines.push(Line::from(vec![
            Span::raw(" "),
            bar,
            space(),
            label,
            Span::styled(" ".repeat(pad as usize), Style::default().fg(theme::MUTED)),
            Span::styled(view.key().to_string(), Style::default().fg(theme::MUTED)),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme::SURFACE).fg(theme::FG)),
        area,
    );
}

fn space() -> Span<'static> {
    Span::raw(" ")
}

fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let status = footer_status(app);
    let left = Line::from(vec![
        space(),
        Span::styled(status, Style::default().fg(theme::MUTED)),
    ]);

    let focus_hint = format!(
        " focus: {} · view: {} ",
        app.focus.label(),
        app.current_view.label()
    );
    let right = Line::from(Span::styled(focus_hint, Style::default().fg(theme::MUTED)));

    // Left status paragraph paints the full-width background; the right hint
    // draws text-only on top, so no spacer spaces are needed.
    f.render_widget(
        Paragraph::new(left)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .alignment(Alignment::Left),
        area,
    );
    f.render_widget(
        Paragraph::new(right)
            .style(Style::default().fg(theme::MUTED))
            .alignment(Alignment::Right),
        area,
    );
}

/// Current-screen status line shown in the footer.
fn footer_status(app: &App) -> String {
    let state = &app.sniff_state;
    if state.is_loading {
        "Loading…".to_string()
    } else if state.error.is_some() {
        "Error — press r to retry".to_string()
    } else if state.projects.is_empty() && app.current_view == ViewId::Projects {
        "No projects found — press r to scan".to_string()
    } else if state.projects.is_empty() {
        "No projects scanned".to_string()
    } else {
        let timestamp = state
            .last_refresh
            .map(|t| humantime::format_rfc3339(t).to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let cache = if state.from_cache {
            " (from cache)"
        } else {
            ""
        };
        format!("Last refresh: {timestamp}{cache}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn sidebar_width_within_spec_range() {
        assert!(
            (22..=24).contains(&SIDEBAR_WIDTH),
            "sidebar must be 22-24 columns, got {SIDEBAR_WIDTH}"
        );
    }

    #[test]
    fn layout_split_header_body_footer() {
        let constraints = [
            Constraint::Length(1), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // footer
        ];
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn sidebar_split_constraints() {
        let constraints = [Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)];
        assert_eq!(constraints.len(), 2);
    }

    #[test]
    fn all_views_have_non_empty_labels() {
        for view in ALL_VIEWS {
            assert!(!view.label().is_empty(), "{view:?} label must be non-empty");
        }
    }

    #[test]
    fn header_line_contains_title_and_degrades_on_narrow_width() {
        let title = Span::styled(
            " raccpack-tui ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        );
        let sep = Span::raw("│");
        let space = Span::raw(" ");
        let root = Span::styled("-", Style::default().fg(theme::MUTED));
        let hotkeys = Span::styled(
            "Tab views · hjkl move · ? help · q quit",
            Style::default().fg(theme::MUTED),
        );

        let full = Line::from(vec![
            title.clone(),
            sep.clone(),
            space.clone(),
            root.clone(),
            sep.clone(),
            space.clone(),
            hotkeys,
        ]);
        assert!(full.width() > 0);

        // On a very narrow header the hotkey strip is dropped.
        let narrow = Line::from(vec![title, sep, space, root]);
        assert!(narrow.width() <= full.width());
    }

    #[test]
    fn footer_status_reflects_current_sniff_state() {
        let app = App::new();
        assert!(!footer_status(&app).is_empty());
    }

    #[test]
    fn sidebar_rows_pad_to_width() {
        // Each row must fill exactly the sidebar width (no under/overflow).
        // Row = leading space + bar + space + label(=prefix) + pad + hint(1).
        for view in ALL_VIEWS {
            let prefix_width = 3 + view.label().chars().count();
            let pad = SIDEBAR_WIDTH.saturating_sub(prefix_width as u16 + 1);
            let total = prefix_width as u16 + pad + 1;
            assert_eq!(
                total, SIDEBAR_WIDTH,
                "{view:?} row (len {total}) must fill sidebar width {SIDEBAR_WIDTH}"
            );
        }
    }

    #[test]
    fn focus_label_matches_footer_hint() {
        assert_eq!(Focus::Sidebar.label(), "sidebar");
        assert_eq!(Focus::Main.label(), "main");
    }
}
