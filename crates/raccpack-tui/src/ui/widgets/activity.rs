//! Activity panel — the visual wrapper over [`ActivityLog`].
//!
//! Rendered only on wide terminals (see `layout.rs::main_split`): a bordered
//! panel with a muted `Activity` title like the other panels. Glyphs carry the
//! semantics (NO_COLOR-safe); colour only reinforces them.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::activity::{ActivityEntry, ActivityKind, ActivityLog};
use crate::theme;

/// Render the activity panel into `area`.
pub fn render(f: &mut Frame, area: Rect, log: &ActivityLog) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " Activity ",
            Style::default().fg(theme::MUTED),
        ));

    f.render_widget(
        Paragraph::new(build_lines(log))
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

/// Build the panel body: newest entry bright, older rows muted, a `(n)` trim
/// marker at the bottom, and an empty-state hint when nothing happened yet.
fn build_lines(log: &ActivityLog) -> Vec<Line<'static>> {
    if log.is_empty() {
        return vec![Line::from(Span::styled(
            " No activity yet",
            Style::default().fg(theme::MUTED),
        ))];
    }

    let mut lines: Vec<Line<'static>> = log
        .entries()
        .iter()
        .enumerate()
        .map(|(i, entry)| entry_line(entry, i == 0))
        .collect();

    if log.trimmed() > 0 {
        lines.push(Line::from(Span::styled(
            format!(" ({})", log.trimmed()),
            Style::default().fg(theme::MUTED),
        )));
    }
    lines
}

/// One activity row: glyph (kind colour) + message. The newest entry is drawn
/// bright (bold), older entries fade to muted.
fn entry_line(entry: &ActivityEntry, newest: bool) -> Line<'static> {
    let message_style = if newest {
        Style::default().fg(theme::FG).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::MUTED)
    };
    Line::from(vec![
        Span::styled(
            format!(" {} ", entry.kind.glyph()),
            Style::default().fg(kind_color(entry.kind)),
        ),
        Span::styled(entry.message.clone(), message_style),
    ])
}

/// Semantic colour reinforcing the glyph (NO_COLOR-safe: glyph carries meaning).
fn kind_color(kind: ActivityKind) -> Color {
    match kind {
        ActivityKind::Ok => theme::SUCCESS,
        ActivityKind::Warn => theme::WARNING,
        ActivityKind::Error => theme::DANGER,
        ActivityKind::Info => theme::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::activity::{ActivityLog, ACTIVITY_CAP};

    fn render_text(log: &ActivityLog, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("test backend");
        terminal.draw(|f| render(f, f.area(), log)).expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn empty_log_hints_to_user() {
        let text = render_text(&ActivityLog::default(), 28, 12);
        assert!(text.contains("No activity yet"));
    }

    #[test]
    fn renders_at_panel_widths_without_panic() {
        let mut log = ActivityLog::default();
        log.push(ActivityKind::Ok, "Scan complete · 2 projects · 1.5 MB");
        log.push(ActivityKind::Warn, "dig srv · 3 findings");
        for width in [28u16, 120] {
            let text = render_text(&log, width, 12);
            assert!(text.contains('✔'), "ok glyph must render: {text:?}");
            assert!(text.contains('!'), "warn glyph must render: {text:?}");
        }
    }

    #[test]
    fn glyphs_carry_semantics_per_kind() {
        for (kind, glyph) in [
            (ActivityKind::Ok, "✔"),
            (ActivityKind::Warn, "!"),
            (ActivityKind::Error, "✖"),
            (ActivityKind::Info, "·"),
        ] {
            let mut log = ActivityLog::default();
            log.push(kind, "message");
            let line = &build_lines(&log)[0];
            let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
            assert!(
                text.contains(glyph),
                "{kind:?} must render {glyph:?}: {text:?}"
            );
        }
    }

    #[test]
    fn newest_first_order() {
        let mut log = ActivityLog::default();
        log.push(ActivityKind::Info, "old");
        log.push(ActivityKind::Info, "new");
        let lines = build_lines(&log);
        let old_y = lines
            .iter()
            .position(|l| l.to_string().contains("old"))
            .expect("old row must be present");
        let new_y = lines
            .iter()
            .position(|l| l.to_string().contains("new"))
            .expect("new row must be present");
        assert!(new_y < old_y, "newest entry must be on top");
    }

    #[test]
    fn newest_row_brighter_than_older() {
        let mut log = ActivityLog::default();
        log.push(ActivityKind::Info, "old");
        log.push(ActivityKind::Info, "new");
        let lines = build_lines(&log);
        let bold = lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
        assert!(bold, "newest message must be bold/brighter");
    }

    #[test]
    fn trim_marker_lists_lost_entries() {
        let mut log = ActivityLog::default();
        for i in 0..=ACTIVITY_CAP {
            log.push(ActivityKind::Info, format!("entry {i}"));
        }
        let text: String = build_lines(&log).iter().map(|l| l.to_string()).collect();
        assert!(
            text.contains("(1)"),
            "one trimmed entry must be marked: {text:?}"
        );
    }

    #[test]
    fn title_is_present_in_render() {
        let text = render_text(&ActivityLog::default(), 28, 12);
        assert!(text.contains("Activity"));
    }
}
