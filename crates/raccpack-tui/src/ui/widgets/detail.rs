//! Detail strip — a bordered metadata panel under a table.
//!
//! Shared by the sniff (Projects) and dig (Findings) screens so both use the
//! same visual language: token colors, fixed height (`space.semantic.detail-height`
//! → [`crate::ui::theme::SPACE_DETAIL_HEIGHT`]), and the middle-dot `·`
//! placeholder for empty values. It never renders secret payloads — callers
//! pass only label/value pairs.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::ui::theme;

/// One label/value row inside the detail strip.
#[derive(Debug, Clone)]
pub struct DetailLine {
    /// Short field label, e.g. `"Risk"` or `"Path"`.
    pub label: &'static str,
    /// Display value; an empty string renders as the `·` placeholder.
    pub value: String,
    /// Foreground for the value. Pass [`crate::ui::theme::MUTED`] for paths
    /// and other secondary metadata, [`crate::ui::theme::FG`] otherwise.
    pub fg: Color,
}

impl DetailLine {
    /// Convenience constructor with the default foreground.
    pub fn new(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
            fg: theme::FG,
        }
    }

    /// Convenience constructor for secondary/muted values (e.g. paths).
    pub fn muted(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
            fg: theme::MUTED,
        }
    }
}

/// Render a bordered detail strip filling exactly `area`.
///
/// The panel height should be [`crate::ui::theme::SPACE_DETAIL_HEIGHT`] (borders
/// included); rows beyond the visible area are clipped by the paragraph.
pub fn render(f: &mut Frame, area: Rect, title: &str, lines: &[DetailLine]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));

    let body: Vec<Line> = lines
        .iter()
        .map(detail_line_to_line)
        .collect();

    f.render_widget(
        Paragraph::new(body)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn detail_line_to_line(line: &DetailLine) -> Line<'_> {
    let value = if line.value.is_empty() {
        theme::EMPTY_PLACEHOLDER.to_string()
    } else {
        line.value.clone()
    };
    Line::from(vec![
        Span::styled(
            format!("  {}: ", line.label),
            Style::default().fg(theme::MUTED),
        ),
        Span::styled(value, Style::default().fg(line.fg)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_value_becomes_placeholder() {
        let line = DetailLine::new("Risk", "");
        let rendered = detail_line_to_line(&line);
        let text: String = rendered.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains(theme::EMPTY_PLACEHOLDER),
            "empty value must render the · placeholder, got {text:?}"
        );
        assert!(!text.contains("  :  "), "value slot must not stay blank");
    }

    #[test]
    fn full_value_is_kept() {
        let line = DetailLine::new("Path", "/a/b/c");
        let rendered = detail_line_to_line(&line);
        let text: String = rendered.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains("/a/b/c"));
        assert!(text.contains("Path"));
    }

    #[test]
    fn muted_uses_muted_fg() {
        let line = DetailLine::muted("Path", "/x");
        assert_eq!(line.fg, theme::MUTED);
        let normal = DetailLine::new("Git", "tracked");
        assert_eq!(normal.fg, theme::FG);
    }

    #[test]
    fn mixed_lines_render_deterministically() {
        let lines = [
            DetailLine::new("Risk", "Critical"),
            DetailLine::new("Kind", ""),
        ];
        let rendered: Vec<String> = lines.iter().map(detail_line_to_line)
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
            .collect();
        assert!(rendered[0].contains("Critical"));
        assert!(rendered[1].contains(theme::EMPTY_PLACEHOLDER));
    }
}