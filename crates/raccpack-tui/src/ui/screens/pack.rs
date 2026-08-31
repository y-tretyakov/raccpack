//! Pack modal overlay — rendered on top of the current screen while a pack
//! flow is active. Render-only: all state lives in `app::pack::PackFlow`.

use std::borrow::Cow;
use std::path::Path;

use raccpack_core::app::PackResult;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::pack::{PackFlow, PackFlowPhase};
use crate::theme;
use crate::ui::widgets::centered_rect;

/// Render the pack flow modal centered over `area`.
pub fn render(f: &mut Frame, area: Rect, flow: &PackFlow) {
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
fn phase_banner(flow: &PackFlow) -> (&'static str, Color) {
    match &flow.phase {
        PackFlowPhase::Preparing => ("Pack — preparing", theme::FOCUS),
        PackFlowPhase::Preview(_) => ("Pack — preview (dry run)", theme::FOCUS),
        PackFlowPhase::Running => ("Pack — running", theme::FOCUS),
        PackFlowPhase::Done(_) => ("Pack — success", theme::SUCCESS),
        PackFlowPhase::Failed(_) => ("Pack — failed", theme::DANGER),
    }
}

/// Body lines for the current phase (never contains raw secret material).
fn phase_lines(flow: &PackFlow) -> Vec<Line<'static>> {
    match &flow.phase {
        PackFlowPhase::Preparing => vec![
            Line::from(Span::styled(
                "  Preparing pack…",
                Style::default().fg(theme::FG),
            )),
            Line::from(""),
            hint("  y confirm · n/Esc cancel"),
        ],
        PackFlowPhase::Preview(result) => preview_lines(flow, result),
        PackFlowPhase::Running => running_lines(flow),
        PackFlowPhase::Done(result) => done_lines(flow, result),
        PackFlowPhase::Failed(message) => vec![
            Line::from(Span::styled(
                format!("  {message}"),
                Style::default().fg(theme::DANGER),
            )),
            Line::from(""),
            hint("  Enter/Esc close"),
        ],
    }
}

/// Dry-run summary: project, archive name (den-relative), options, sizes.
fn preview_lines(flow: &PackFlow, result: &PackResult) -> Vec<Line<'static>> {
    let project = flow
        .project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "-".to_string());
    let archive = den_relative(&flow.den_dir, &result.output);

    if flow.editing_output_name {
        return output_name_editor_lines(flow);
    }

    let output_name = flow.options.output_name.as_deref().unwrap_or("(auto)");
    vec![
        row("project", &project, theme::FG),
        row("archive", &archive, theme::FOCUS),
        row("output-name", output_name, theme::FG),
        row(
            "zstd",
            &format!("level {}", flow.options.zstd_level),
            theme::FG,
        ),
        toggled("content-deny", flow.options.deny_content_secrets),
        row("files", "0 (dry run)", theme::FG),
        Line::from(""),
        Line::from(Span::styled(
            "  dry-run: nothing will be written to the den",
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        hint("  y/Enter confirm · o output-name · c content-deny · z zstd-level · n/Esc cancel"),
    ]
}

/// Inline editor for the custom `output_name`. The buffer holds the draft;
/// core validates on the actual run.
fn output_name_editor_lines(flow: &PackFlow) -> Vec<Line<'static>> {
    let draft = flow.output_name_buffer.as_str();
    let display = if draft.is_empty() { "(auto)" } else { draft };
    vec![
        bold("  Output archive name", theme::FOCUS),
        Line::from(""),
        row("name", display, theme::FG),
        Line::from(Span::styled(
            format!("  result: {}.tar.zst", display),
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        hint("  type name · Enter commit · Esc cancel · empty = auto name"),
    ]
}

fn running_lines(flow: &PackFlow) -> Vec<Line<'static>> {
    vec![
        progress_bar(flow.percent),
        Line::from(""),
        Line::from(Span::styled(
            flow.message.clone(),
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        hint("  running… Esc does not cancel"),
    ]
}

/// Honest result screen: path to archive, file count, size.
fn done_lines(flow: &PackFlow, result: &PackResult) -> Vec<Line<'static>> {
    let archive = den_relative(&flow.den_dir, &result.output);
    let size = human_bytes(result.size_bytes);

    vec![
        bold("  Pack complete", theme::SUCCESS),
        Line::from(""),
        row("archive", &archive, theme::FOCUS),
        row("files", &format!("{}", result.file_count), theme::FG),
        row("size", &size, theme::FG),
        row(
            "skipped secrets",
            &format!("{}", result.skipped_secret_files),
            theme::FG,
        ),
        Line::from(""),
        hint("  Enter/Esc close"),
    ]
}

/// Manual progress bar; the percent comes from core events, never invented.
fn progress_bar(percent: u8) -> Line<'static> {
    let width = 24usize;
    let filled = usize::from(percent).min(100) * width / 100;
    let bar = format!(
        "{}{}  {}%",
        "█".repeat(filled),
        "░".repeat(width - filled),
        percent
    );
    Line::from(vec![
        Span::raw("  "),
        Span::styled(bar, Style::default().fg(theme::FOCUS)),
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

fn den_relative<'a>(den_dir: &Path, path: &'a Path) -> Cow<'a, str> {
    path.strip_prefix(den_dir)
        .map(|p| p.to_string_lossy())
        .unwrap_or_else(|_| path.to_string_lossy())
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn human_bytes_scales_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(2048), "2.0 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(human_bytes(2 * 1024 * 1024 * 1024), "2.0 GiB");
    }

    #[test]
    fn progress_bar_renders_percent_and_fits_width() {
        let line = progress_bar(33);
        assert_eq!(line.width(), 2 + 24 + 2 + 3);
        assert!(line.to_string().contains("33%"));
    }

    #[test]
    fn den_relative_falls_back_to_absolute_when_outside() {
        let den = Path::new("/data/den");
        let inside = Path::new("/data/den/packs/2026/08/x.tar.zst");
        assert_eq!(den_relative(den, inside), "packs/2026/08/x.tar.zst");
        let outside = Path::new("/other/y");
        assert_eq!(den_relative(den, outside), "/other/y");
    }

    fn flow_in_preview(output_name: Option<String>) -> PackFlow {
        let mut opts = crate::app::pack::PackFlowOptions::default();
        opts.set_output_name(output_name);
        let mut flow = PackFlow::new(PathBuf::from("/proj"), PathBuf::from("/den"), opts);
        flow.phase = PackFlowPhase::Preview(PackResult {
            source: PathBuf::from("/proj"),
            output: PathBuf::from("/den/packs/2026/08/x.tar.zst"),
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        });
        flow
    }

    #[test]
    fn preview_shows_custom_output_name() {
        let flow = flow_in_preview(Some("my-artifact".to_string()));
        let text = preview_lines(
            &flow,
            &PackResult {
                source: PathBuf::from("/proj"),
                output: PathBuf::from("/den/packs/2026/08/my-artifact.tar.zst"),
                size_bytes: 0,
                file_count: 0,
                skipped_secret_files: 0,
                dry_run: true,
            },
        )
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        assert!(
            text.contains("my-artifact"),
            "expected custom name in preview: {text}"
        );
    }

    #[test]
    fn preview_falls_back_to_auto_output_name() {
        let flow = flow_in_preview(None);
        let text = preview_lines(
            &flow,
            &PackResult {
                source: PathBuf::from("/proj"),
                output: PathBuf::from("/den/packs/2026/08/slug__ts.tar.zst"),
                size_bytes: 0,
                file_count: 0,
                skipped_secret_files: 0,
                dry_run: true,
            },
        )
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        assert!(text.contains("(auto)"), "expected auto output-name: {text}");
    }

    #[test]
    fn output_name_editor_shows_draft_and_result() {
        let mut flow = flow_in_preview(None);
        flow.editing_output_name = true;
        flow.output_name_buffer = "my-artifact".to_string();
        let text = output_name_editor_lines(&flow)
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("my-artifact"), "draft missing: {text}");
        assert!(
            text.contains("my-artifact.tar.zst"),
            "result missing: {text}"
        );
    }
}
