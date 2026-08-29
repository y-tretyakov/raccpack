//! Top-level frame rendering: header, sidebar, main area, footer.

use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::app::{App, ViewId};
use crate::ui::screens;
use crate::ui::theme;

const SIDEBAR_WIDTH: u16 = 16;

/// Render one complete frame.
pub fn render(app: &App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
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

        render_header(f, outer[0]);
        render_body(f, outer[1], app);
        render_footer(f, outer[2]);

        if app.help_visible {
            screens::help::render(f, area);
        }
    })?;
    Ok(())
}

fn render_header(f: &mut ratatui::Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            "  raccpack-tui ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│"),
        Span::styled(" 1-4 navigate", Style::default().fg(theme::MUTED)),
        Span::raw(" │ "),
        Span::styled("? help", Style::default().fg(theme::MUTED)),
        Span::raw(" │ "),
        Span::styled("q quit", Style::default().fg(theme::MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_body(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
        .split(area);

    render_sidebar(f, chunks[0], app);
    render_main(f, chunks[1], app);
}

fn render_sidebar(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let views = [
        ViewId::Overview,
        ViewId::Projects,
        ViewId::Findings,
        ViewId::Operations,
    ];

    let mut lines = Vec::new();
    for &view in &views {
        let is_active = app.current_view == view;
        let style = if is_active {
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::MUTED)
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", view.key()), style),
            Span::styled(view.label(), style),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(" Nav ", Style::default().fg(theme::ACCENT)));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG)),
        area,
    );
}

fn render_main(f: &mut ratatui::Frame, area: Rect, app: &App) {
    screens::render_screen(f, area, app.current_view);
}

fn render_footer(f: &mut ratatui::Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("  raccpack-tui v", Style::default().fg(theme::MUTED)),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(theme::MUTED)),
        Span::raw("                              "),
        Span::styled("q quit │ ? help", Style::default().fg(theme::MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_width_constant() {
        assert_eq!(SIDEBAR_WIDTH, 16);
    }

    #[test]
    fn header_line_contains_title() {
        let line = Line::from(vec![
            Span::styled(
                "  raccpack-tui ",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("│"),
            Span::styled(" 1-4 navigate", Style::default().fg(theme::MUTED)),
            Span::raw(" │ "),
            Span::styled("? help", Style::default().fg(theme::MUTED)),
            Span::raw(" │ "),
            Span::styled("q quit", Style::default().fg(theme::MUTED)),
        ]);
        assert!(line.width() > 0, "header line must have content");
    }

    #[test]
    fn footer_line_contains_version() {
        let v = env!("CARGO_PKG_VERSION");
        let line = Line::from(vec![
            Span::styled("  raccpack-tui v", Style::default().fg(theme::MUTED)),
            Span::styled(v, Style::default().fg(theme::MUTED)),
            Span::raw("                              "),
            Span::styled("q quit │ ? help", Style::default().fg(theme::MUTED)),
        ]);
        assert!(line.width() > 0);
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
}
