//! Raid modal overlay — rendered on top of the current screen while a raid
//! flow is active. Render-only: all state lives in `app::raid::RaidFlow`.

use std::borrow::Cow;
use std::path::Path;

use raccpack_core::app::{OrchestrationMode, RaidResult};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::raid::{FlowPhase, PassphraseInput, RaidFlow};
use crate::theme;
use crate::ui::widgets::centered_rect;

/// Render the raid flow modal centered over `area`.
pub fn render(f: &mut Frame, area: Rect, flow: &RaidFlow) {
    let popup = centered_rect(72, 78, area);
    f.render_widget(Clear, popup);

    let (title, accent) = phase_banner(flow);
    let lines = phase_lines(flow);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        popup,
    );
}

/// Title accent that maps the current phase to a status color.
fn phase_banner(flow: &RaidFlow) -> (&'static str, Color) {
    match &flow.phase {
        FlowPhase::Preparing => ("Raid — preparing", theme::FOCUS),
        FlowPhase::Preview(_) => ("Raid — preview (dry run)", theme::FOCUS),
        FlowPhase::Passphrase(_) => ("Raid — passphrase", theme::WARNING),
        FlowPhase::Running => ("Raid — running", theme::FOCUS),
        FlowPhase::Done(result) if result.success => ("Raid — success", theme::SUCCESS),
        FlowPhase::Done(result) if result.rolled_back => ("Raid — rolled back", theme::WARNING),
        FlowPhase::Done(_) => ("Raid — failed", theme::DANGER),
        FlowPhase::Failed(_) => ("Raid — failed", theme::DANGER),
    }
}

/// Body lines for the current phase (never contains raw secret material).
fn phase_lines(flow: &RaidFlow) -> Vec<Line<'static>> {
    match &flow.phase {
        FlowPhase::Preparing => vec![
            Line::from(Span::styled(
                "  Preparing raid…",
                Style::default().fg(theme::FG),
            )),
            Line::from(""),
            hint("  y confirm · n/Esc cancel"),
        ],
        FlowPhase::Preview(result) => preview_lines(flow, result),
        FlowPhase::Passphrase(input) => passphrase_lines(input),
        FlowPhase::Running => running_lines(flow),
        FlowPhase::Done(result) => done_lines(flow, result),
        FlowPhase::Failed(message) => vec![
            Line::from(Span::styled(
                format!("  {message}"),
                Style::default().fg(theme::DANGER),
            )),
            Line::from(""),
            hint("  Enter/Esc close"),
        ],
    }
}

/// Dry-run summary: project, mode badge, phases, keep/skip toggles.
fn preview_lines(flow: &RaidFlow, _result: &RaidResult) -> Vec<Line<'static>> {
    let project = flow
        .project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "-".to_string());

    vec![
        row("Project", &project, theme::FG),
        row("Mode", mode_badge(flow.options.mode), theme::FOCUS),
        phases_row(flow),
        row("min-risk", "High", theme::FG),
        toggled("keep-sources", flow.options.keep_sources),
        toggled("skip-stash", flow.options.skip_stash),
        Line::from(""),
        Line::from(Span::styled(
            "  dry-run: nothing will be written to the den",
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        hint("  y/Enter confirm · K keep-src · S skip-stash · m mode · n/Esc cancel"),
    ]
}

/// Phase plan row; a skipped stash is dimmed.
fn phases_row(flow: &RaidFlow) -> Line<'static> {
    let mut spans = vec![
        Span::styled("  phases".to_string(), Style::default().fg(theme::MUTED)),
        Span::raw("  "),
    ];
    let mut first = true;
    for name in flow.planned_names().into_iter().filter(|p| *p != "move") {
        let skipped = name == "stash" && flow.options.skip_stash;
        let label = if skipped { "stash (skip)" } else { name };
        let color = if skipped { theme::MUTED } else { theme::FOCUS };
        if !first {
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::styled(label.to_string(), Style::default().fg(color)));
        first = false;
    }
    Line::from(spans)
}

fn running_lines(flow: &RaidFlow) -> Vec<Line<'static>> {
    let mut lines = vec![progress_bar(flow.overall_percent), Line::from("")];
    if flow.pipeline.is_empty() {
        for (i, name) in flow.planned_names().into_iter().enumerate() {
            lines.push(pipeline_row(name, false, i == 0));
        }
    } else {
        for line in &flow.pipeline {
            lines.push(pipeline_row(&line.name, line.done, line.current));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        flow.message.clone(),
        Style::default().fg(theme::MUTED),
    )));
    lines.push(Line::from(""));
    lines.push(hint("  running… Esc does not cancel"));
    lines
}

fn passphrase_lines(input: &PassphraseInput) -> Vec<Line<'static>> {
    let active_len = if input.step().is_confirm() {
        input.confirm_len()
    } else {
        input.first_len()
    };
    let prompt = if input.step().is_confirm() {
        "  enter passphrase (repeat):"
    } else {
        "  enter passphrase (first):"
    };
    let dots = if active_len == 0 {
        " ".to_string()
    } else {
        "•".repeat(active_len)
    };
    let mut lines = vec![
        Line::from(Span::styled(
            prompt.to_string(),
            Style::default().fg(theme::FG),
        )),
        Line::from(vec![
            Span::styled("  ", Style::default().fg(theme::FG)),
            Span::styled(dots, Style::default().fg(theme::FOCUS)),
        ]),
        Line::from(""),
        hint("  Enter confirm · Backspace delete · Esc cancel"),
    ];
    if let Some(error) = input.error() {
        lines.push(Line::from(Span::styled(
            format!("  {error}"),
            Style::default().fg(theme::DANGER),
        )));
    }
    lines
}

/// Honest result screen: Success / Rolled back / Failed, stage outcomes,
/// fail-fast warning, and den-relative artifact paths.
fn done_lines(flow: &RaidFlow, result: &RaidResult) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if result.success {
        lines.push(bold(
            &format!("  {}", result_verdict(result)),
            theme::SUCCESS,
        ));
    } else if result.rolled_back {
        lines.push(bold("  Rolled back to the pre-raid state", theme::WARNING));
        if !result.rollback_warnings.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {} warnings while rolling back",
                    result.rollback_warnings.len()
                ),
                Style::default().fg(theme::WARNING),
            )));
        }
    } else {
        lines.push(bold("  Failed", theme::DANGER));
        for stage in result.stages.iter().filter(|s| !s.success && !s.skipped) {
            lines.push(Line::from(Span::styled(
                format!("  {} — {}", stage.name, stage.message),
                Style::default().fg(theme::DANGER),
            )));
        }
    }

    lines.push(Line::from(""));
    for stage in &result.stages {
        let (glyph, color) = if stage.skipped {
            ("skip", theme::MUTED)
        } else if stage.success {
            ("ok", theme::SUCCESS)
        } else {
            ("fail", theme::DANGER)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {glyph:>4}"), Style::default().fg(color)),
            Span::raw("  "),
            Span::styled(stage.name.to_uppercase(), Style::default().fg(theme::FG)),
        ]));
    }

    if flow.options.mode == OrchestrationMode::FailFast {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Fail-Fast: artifacts already placed may remain in the den",
            Style::default().fg(theme::WARNING),
        )));
    }
    if !result.den_artifacts.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  artifacts:",
            Style::default().fg(theme::FOCUS),
        )));
        for artifact in result.den_artifacts.iter().take(5) {
            lines.push(Line::from(Span::styled(
                format!("    {}", den_relative(&flow.den_dir, artifact)),
                Style::default().fg(theme::FG),
            )));
        }
        if result.den_artifacts.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("    +{} more", result.den_artifacts.len() - 5),
                Style::default().fg(theme::MUTED),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(hint("  Enter/Esc close"));
    lines
}

fn result_verdict(result: &RaidResult) -> &str {
    if result.success {
        "Success"
    } else if result.rolled_back {
        "Rolled back"
    } else {
        "Failed"
    }
}

/// Manual progress bar (no gauge widget in the theme yet); the percent comes
/// from core events, never invented here.
fn progress_bar(overall: u8) -> Line<'static> {
    let width = 24usize;
    let filled = usize::from(overall).min(100) * width / 100;
    let bar = format!(
        "{}{}  {}%",
        "█".repeat(filled),
        "░".repeat(width - filled),
        overall
    );
    Line::from(vec![
        Span::raw("  "),
        Span::styled(bar, Style::default().fg(theme::FOCUS)),
    ])
}

fn pipeline_row(name: &str, done: bool, current: bool) -> Line<'static> {
    let (glyph, color) = if done {
        ("✓", theme::SUCCESS)
    } else if current {
        ("→", theme::FOCUS)
    } else {
        ("○", theme::MUTED)
    };
    Line::from(vec![
        Span::raw("  "),
        Span::styled(glyph.to_string(), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(name.to_uppercase(), Style::default().fg(color)),
    ])
}

fn row(label: &str, value: &str, value_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<12}"), Style::default().fg(theme::MUTED)),
        Span::styled(value.to_string(), Style::default().fg(value_color)),
    ])
}

fn toggled(label: &str, on: bool) -> Line<'static> {
    let (value, color) = if on {
        ("on", theme::FOCUS)
    } else {
        ("off", theme::MUTED)
    };
    row(label, value, color)
}

fn bold(text: &str, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

fn hint(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(theme::MUTED)
            .add_modifier(Modifier::ITALIC),
    ))
}

fn mode_badge(mode: OrchestrationMode) -> &'static str {
    match mode {
        OrchestrationMode::Atomic => "ATOMIC",
        OrchestrationMode::FailFast => "FAIL-FAST",
    }
}

fn den_relative<'a>(den_dir: &Path, path: &'a Path) -> Cow<'a, str> {
    path.strip_prefix(den_dir)
        .map(|p| p.to_string_lossy())
        .unwrap_or_else(|_| path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::raid::PassphraseInput;

    #[test]
    fn mode_badge_matches_modes() {
        assert_eq!(mode_badge(OrchestrationMode::Atomic), "ATOMIC");
        assert_eq!(mode_badge(OrchestrationMode::FailFast), "FAIL-FAST");
    }

    #[test]
    fn progress_bar_renders_percent_and_fits_width() {
        let line = progress_bar(33);
        let width = line.width();
        assert_eq!(width, 2 + 24 + 2 + 3, "bar = pad + track + pad + '33%'",);
        let text = line.to_string();
        assert!(text.contains("33%"));
    }

    #[test]
    fn passphrase_lines_show_dots_never_the_value() {
        let mut input = PassphraseInput::new();
        for c in "hunter2-secret".chars() {
            input.push_char(c);
        }
        let lines = passphrase_lines(&input);
        let joined: String = lines.iter().map(|l| l.to_string()).collect();
        assert!(
            !joined.contains("hunter2"),
            "passphrase modal must never print the typed value: {joined}"
        );
        assert!(joined.contains("•"), "dots must stand in for the input");
    }

    #[test]
    fn den_relative_falls_back_to_absolute_when_outside() {
        let den = Path::new("/data/den");
        let inside = Path::new("/data/den/packs/2026/08/x.tar.zst");
        assert_eq!(den_relative(den, inside), "packs/2026/08/x.tar.zst");
        let outside = Path::new("/other/y");
        assert_eq!(den_relative(den, outside), "/other/y");
    }
}
