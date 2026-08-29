//! Screen registry — routes ViewId to its renderer.

pub mod help;
pub mod sniff;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, ViewId};
use crate::ui::theme;

/// Render the given screen into `area`.
pub fn render_screen(f: &mut Frame, area: Rect, app: &mut App) {
    match app.current_view {
        ViewId::Overview => render_overview(f, area),
        ViewId::Projects => crate::ui::screens::sniff::render(f, area, &mut app.sniff_state),
        ViewId::Findings => render_findings(f, area),
        ViewId::Operations => render_operations(f, area),
    }
}

fn render_overview(f: &mut Frame, area: Rect) {
    let (title, text, accent) = (
        "Overview",
        "No projects scanned yet.\nRun `racc sniff` to get started.",
        theme::ACCENT,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let lines: Vec<Line<'_>> = text.lines().map(Line::from).collect();

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_findings(f: &mut Frame, area: Rect) {
    let (title, text, accent) = (
        "Findings",
        "No findings yet.\nResults will appear after a scan.",
        theme::WARNING,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let lines: Vec<Line<'_>> = text.lines().map(Line::from).collect();

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_operations(f: &mut Frame, area: Rect) {
    let (title, text, accent) = (
        "Operations",
        "No operations in progress.\nHistory will appear here.",
        theme::DANGER,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let lines: Vec<Line<'_>> = text.lines().map(Line::from).collect();

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ViewId;

    #[test]
    fn all_views_have_labels() {
        let views = [
            ViewId::Overview,
            ViewId::Projects,
            ViewId::Findings,
            ViewId::Operations,
        ];
        for v in &views {
            assert!(!v.label().is_empty());
        }
    }

    #[test]
    fn view_accent_colours_are_distinct() {
        let accents = [theme::ACCENT, theme::SUCCESS, theme::WARNING, theme::DANGER];
        let unique: Vec<_> = accents.iter().collect();
        assert_eq!(
            unique.len(),
            4,
            "each view should have a unique accent colour"
        );
    }

    #[test]
    fn overview_text_mentions_sniff() {
        let (_, text, _) = (
            ViewId::Overview.label(),
            "Run `racc sniff` to get started.",
            theme::ACCENT,
        );
        assert!(text.contains("sniff"));
    }

    #[test]
    fn projects_text_mentions_detect() {
        let (_, text, _) = (
            ViewId::Projects.label(),
            "Run `racc sniff` to detect projects.",
            theme::SUCCESS,
        );
        assert!(text.contains("detect"));
    }
}
