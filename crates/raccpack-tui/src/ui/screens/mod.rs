//! Screen registry — routes ViewId to its renderer.

pub mod dig;
pub mod help;
pub mod raid;
pub mod reveal;
pub mod sniff;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, ViewId};
use crate::theme;

/// Render the given screen into `area`.
pub fn render_screen(f: &mut Frame, area: Rect, app: &mut App) {
    match app.current_view {
        ViewId::Overview => render_stub(
            f,
            area,
            "Overview",
            "No projects scanned yet.\nRun `racc sniff` to get started.",
            theme::FOCUS,
        ),
        ViewId::Projects => crate::ui::screens::sniff::render(f, area, &mut app.sniff_state),
        ViewId::Findings => crate::ui::screens::dig::render(f, area, &mut app.dig_state),
        ViewId::Operations => render_stub(
            f,
            area,
            "Operations",
            "No operations in progress.\nHistory will appear here.",
            theme::DANGER,
        ),
    }
}

/// Shared placeholder for not-yet-implemented screens.
fn render_stub(
    f: &mut Frame,
    area: Rect,
    title: &str,
    subtitle: &str,
    accent: ratatui::style::Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let body = vec![
        Line::from(""),
        Line::from(Span::styled(subtitle, Style::default().fg(theme::MUTED))),
        Line::from(""),
        Line::from(Span::styled(
            "press 2 or Tab for Projects",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    f.render_widget(
        Paragraph::new(body)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .alignment(Alignment::Center),
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
        let accents = [theme::FOCUS, theme::SUCCESS, theme::WARNING, theme::DANGER];
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
            theme::FOCUS,
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

    #[test]
    fn stub_hints_point_to_projects() {
        // The stub copy must always offer a way forward to the sniff screen.
        let hint = "press 2 or Tab for Projects";
        assert!(hint.contains("Tab"));
        assert!(hint.contains("Projects"));
    }
}
