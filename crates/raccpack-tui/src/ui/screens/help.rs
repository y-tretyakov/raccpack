//! Help overlay — toggled by `?`, dismissed by `Esc`.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme;

struct HelpItem {
    key: &'static str,
    description: &'static str,
}

fn help_items() -> Vec<HelpItem> {
    vec![
        HelpItem {
            key: "1-4",
            description: "Switch view",
        },
        HelpItem {
            key: "?",
            description: "Toggle this help",
        },
        HelpItem {
            key: "Esc",
            description: "Close help",
        },
        HelpItem {
            key: "q",
            description: "Quit",
        },
        HelpItem {
            key: "Ctrl-C",
            description: "Force quit",
        },
    ]
}

/// Render the help dialog centered in `area`.
pub fn render(f: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 40, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));

    let header = Line::from(vec![
        Span::styled(
            "  Key",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "    Description",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut lines = vec![header, Line::from("")];
    for item in help_items() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:6}", item.key), Style::default().fg(theme::FG)),
            Span::styled(item.description, Style::default().fg(theme::MUTED)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Press Esc to close",
        Style::default()
            .fg(theme::MUTED)
            .add_modifier(Modifier::ITALIC),
    )]));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        popup,
    );
}

/// Compute a centered rectangle as a percentage of the parent.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn help_items_non_empty() {
        assert!(!help_items().is_empty());
    }

    #[test]
    fn help_items_have_keys_and_descriptions() {
        for item in help_items() {
            assert!(!item.key.is_empty(), "key must not be empty");
            assert!(
                !item.description.is_empty(),
                "description must not be empty"
            );
        }
    }

    #[test]
    fn help_items_count() {
        assert_eq!(help_items().len(), 5);
    }

    #[test]
    fn centered_rect_is_smaller_than_parent() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        assert!(child.width < parent.width);
        assert!(child.height < parent.height);
    }

    #[test]
    fn centered_rect_has_positive_dimensions() {
        let parent = Rect::new(0, 0, 80, 24);
        let child = centered_rect(60, 40, parent);
        assert!(child.width > 0);
        assert!(child.height > 0);
    }

    #[test]
    fn centered_rect_is_centered() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        let left_margin = child.x.saturating_sub(parent.x);
        let right_margin = (parent.x + parent.width).saturating_sub(child.x + child.width);
        // Margins should be equal (±1 for rounding)
        assert!((left_margin as i16 - right_margin as i16).unsigned_abs() <= 1);
    }
}
