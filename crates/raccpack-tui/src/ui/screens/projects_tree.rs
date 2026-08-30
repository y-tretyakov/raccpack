//! Projects — Tree rendering mode (stub until V2-T1).
//!
//! The real tree layout is a follow-up milestone; this view only signals
//! intent so the `v` cycle never renders a blank area and never panics.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;

const STUB_HINT: &str = "Tree view — follow-up (V2-T1)";

/// Render the tree placeholder into `area` (the top project region).
pub fn render(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " Projects — tree ",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::BOLD),
        ));

    let body = vec![
        Line::from(""),
        Line::from(Span::styled(STUB_HINT, Style::default().fg(theme::MUTED))),
        Line::from(Span::styled(
            "A hierarchy of projects lands in a later milestone.",
            Style::default().fg(theme::MUTED),
        )),
        Line::from(Span::styled(
            "Press v to cycle Cards → Table → Tree.",
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

    #[test]
    fn stub_hint_mentions_follow_up() {
        assert!(STUB_HINT.contains("V2-T1"));
    }

    #[test]
    fn tree_renders_hint_not_blank() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
        terminal.draw(|f| render(f, f.area())).expect("draw");
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            text.contains("V2-T1"),
            "tree stub carries its hint, got: {text}"
        );
    }
}
