//! Reveal modal overlay — the ONE place a raw secret value is rendered.
//!
//! Render-only: all state lives in `app::reveal::RevealModal`. The raw value
//! is shown verbatim only in the `Ready` phase and disappears (is zeroized)
//! the moment the modal closes. In every other phase only metadata and prompts
//! render — never any value.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::reveal::{RevealModal, RevealPhase};
use crate::theme;
use crate::ui::widgets::centered_rect;

/// Render the reveal modal centered over `area`.
pub fn render(f: &mut Frame, area: Rect, modal: &RevealModal) {
    let popup = centered_rect(70, 40, area);
    f.render_widget(Clear, popup);

    let (title, accent) = banner(modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(
        Paragraph::new(body_lines(modal))
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

/// Title + accent colour for the current phase.
fn banner(modal: &RevealModal) -> (&'static str, Color) {
    match &modal.phase {
        RevealPhase::Confirm => ("Reveal — confirm", theme::WARNING),
        RevealPhase::Revealing => ("Reveal — reading", theme::FOCUS),
        RevealPhase::Ready { .. } => ("Reveal — value", theme::DANGER),
        RevealPhase::Failed { .. } => ("Reveal — failed", theme::DANGER),
    }
}

/// Body lines for the current phase. The raw value renders ONLY in `Ready`.
fn body_lines(modal: &RevealModal) -> Vec<Line<'static>> {
    let file = modal
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "-".to_string());
    // Marker label is safe to show (it is a rule id, not a value).
    let marker = &modal.reference.marker_id;

    match &modal.phase {
        RevealPhase::Confirm => vec![
            Line::from(vec![
                Span::styled(
                    "  Reveal the secret value in ",
                    Style::default().fg(theme::FG),
                ),
                Span::styled(file, Style::default().fg(theme::WARNING)),
            ]),
            Line::from(Span::styled(
                format!("  marker: {marker}"),
                Style::default().fg(theme::MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  The value will be shown once and wiped on close.",
                Style::default().fg(theme::MUTED),
            )),
            Line::from(""),
            hint("y reveal · n/Esc cancel"),
        ],
        RevealPhase::Revealing => vec![
            Line::from(Span::styled(
                "  Reading and verifying the value…",
                Style::default().fg(theme::FG),
            )),
            Line::from(""),
            hint("verifying against the recorded hash…"),
        ],
        RevealPhase::Ready { secret } => {
            let value = secret.expose();
            vec![
                Line::from(Span::styled(
                    format!("  {file} · {marker}"),
                    Style::default().fg(theme::MUTED),
                )),
                Line::from(""),
                // THE single screen that shows the raw value verbatim.
                Line::from(Span::styled(
                    value.to_string(),
                    Style::default()
                        .fg(theme::DANGER)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  This value is shown once.",
                    Style::default().fg(theme::MUTED),
                )),
                Line::from(""),
                hint("any key closes and wipes this value"),
            ]
        }
        RevealPhase::Failed { message } => vec![
            Line::from(Span::styled(
                format!("  {message}"),
                Style::default().fg(theme::DANGER),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  The file may have changed since the scan.",
                Style::default().fg(theme::MUTED),
            )),
            Line::from(""),
            hint("Enter/Esc close"),
        ],
    }
}

fn hint(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            text.to_string(),
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::reveal::{RevealModal, RevealPhase};
    use crate::worker::WorkerRevealSecret;
    use raccpack_core::secrets::FindingRef;
    use std::path::PathBuf;

    fn reference() -> FindingRef {
        FindingRef {
            path: PathBuf::from("/repo/.env"),
            marker_id: "aws_access_key".to_string(),
            line: 1,
            value_hash: "abc".to_string(),
        }
    }

    #[test]
    fn ready_phase_renders_the_value() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_ready(WorkerRevealSecret::new("AKIASUPERSECRET123".to_string()));
        let text: String = body_lines(&modal).iter().map(|l| l.to_string()).collect();
        assert!(
            text.contains("AKIASUPERSECRET123"),
            "Ready phase must render the raw value: {text}"
        );
    }

    #[test]
    fn confirm_phase_never_renders_a_value() {
        let modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        let text: String = body_lines(&modal).iter().map(|l| l.to_string()).collect();
        assert!(
            !text.contains("AKIA"),
            "Confirm must not render any value: {text}"
        );
        assert!(text.contains("y reveal"));
    }

    #[test]
    fn failed_phase_renders_message_only() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_failed("file changed since dig".to_string());
        let text: String = body_lines(&modal).iter().map(|l| l.to_string()).collect();
        assert!(text.contains("file changed since dig"));
        assert!(
            !text.contains("AKIA"),
            "Failed must not render any value: {text}"
        );
    }

    #[test]
    fn revealing_phase_has_no_value() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.phase = RevealPhase::Revealing;
        let text: String = body_lines(&modal).iter().map(|l| l.to_string()).collect();
        assert!(!text.contains("AKIA"));
    }
}
