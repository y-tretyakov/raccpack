//! Project card — compact recent-project panel for the overview dashboard.
//!
//! Visual hierarchy per b1.0 §5.2: **PRIMARY** name → **SECONDARY**
//! language · size → **STATE** git glyph → **METADATA** path. Empty fields
//! render as the `·` placeholder so a card never leaves blank cells. Cards are
//! read-only (no focus) and carry no secret data.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::sniff::ProjectRow;
use crate::theme;
use crate::ui::widgets::{format_bytes, language_accent};

/// Card height in rows: PRIMARY, SECONDARY, STATE, METADATA.
pub const CARD_HEIGHT: u16 = 4;

/// Render one project card into `area` (at most [`CARD_HEIGHT`] rows; extra
/// content is clipped by ratatui).
pub fn render(f: &mut Frame, area: Rect, project: &ProjectRow) {
    f.render_widget(
        Paragraph::new(card_lines(project))
            .style(Style::default().bg(theme::SURFACE_RAISED).fg(theme::FG)),
        area,
    );
}

fn card_lines(project: &ProjectRow) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            project.name.clone(),
            Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
        )),
        Line::from(secondary_spans(project)),
        Line::from(state_spans(project)),
        Line::from(Span::styled(
            path_or_placeholder(project),
            Style::default().fg(theme::MUTED),
        )),
    ]
}

/// `language · size` with an optional language accent dot in front.
fn secondary_spans(project: &ProjectRow) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(4);
    match &project.language {
        Some(lang) => {
            spans.push(Span::styled(
                "● ",
                Style::default().fg(language_accent(lang)),
            ));
            spans.push(Span::styled(lang.clone(), Style::default().fg(theme::FG)));
        }
        None => spans.push(Span::styled(
            theme::EMPTY_PLACEHOLDER,
            Style::default().fg(theme::MUTED),
        )),
    }
    spans.push(Span::styled(" · ", Style::default().fg(theme::MUTED)));
    let size = if project.size_bytes == 0 {
        theme::EMPTY_PLACEHOLDER.to_string()
    } else {
        format_bytes(project.size_bytes)
    };
    spans.push(Span::styled(size, Style::default().fg(theme::FG)));
    spans
}

/// STATE line: `● CLEAN` in success for a git repo, muted `·` otherwise.
fn state_spans(project: &ProjectRow) -> Vec<Span<'static>> {
    if project.is_git_repo {
        vec![
            Span::styled("● ", Style::default().fg(theme::GIT_CLEAN)),
            Span::styled(
                "CLEAN",
                Style::default()
                    .fg(theme::GIT_CLEAN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![Span::styled(
            theme::EMPTY_PLACEHOLDER,
            Style::default().fg(theme::MUTED),
        )]
    }
}

fn path_or_placeholder(project: &ProjectRow) -> String {
    let path = project.path.to_string_lossy();
    if path.is_empty() {
        theme::EMPTY_PLACEHOLDER.to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn row(
        name: &str,
        language: Option<&str>,
        size_bytes: u64,
        is_git_repo: bool,
        path: &str,
    ) -> ProjectRow {
        ProjectRow {
            name: name.to_string(),
            language: language.map(str::to_string),
            frameworks: vec![],
            size_bytes,
            is_git_repo,
            path: PathBuf::from(path),
        }
    }

    fn card_text(project: &ProjectRow) -> String {
        let lines: Vec<String> = card_lines(project)
            .into_iter()
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
            .collect();
        lines.join("\n")
    }

    #[test]
    fn full_card_follows_primary_secondary_state_metadata() {
        let project = row(
            "raccpack-core",
            Some("Rust"),
            11 * 1024 * 1024 + 512 * 1024,
            true,
            "/home/dev/projects/raccpack-core",
        );
        let text = card_text(&project);
        assert!(text.contains("raccpack-core"));
        assert!(text.contains("Rust"));
        assert!(text.contains("11.5 MB"));
        assert!(text.contains("● CLEAN"));
        assert!(text.contains("/home/dev/projects/raccpack-core"));
    }

    #[test]
    fn secondary_sorts_by_lines() {
        let project = row("web", Some("TypeScript"), 256 * 1024, false, "/w");
        let lines = card_lines(&project);
        let title = lines[0].to_string();
        let secondary = lines[1].to_string();
        let state = lines[2].to_string();
        let meta = lines[3].to_string();
        assert!(title.starts_with("web"));
        assert!(secondary.contains("TypeScript") && secondary.contains("256.0 KB"));
        assert!(state.contains(theme::EMPTY_PLACEHOLDER));
        assert!(meta.contains("/w"));
    }

    #[test]
    fn empty_fields_render_placeholders() {
        let project = row("empty", None, 0, false, "");
        let text = card_text(&project);
        assert!(text.contains(theme::EMPTY_PLACEHOLDER));
        assert!(!text.contains("CLEAN"), "no git repo → no clean badge");
    }

    #[test]
    fn git_repo_uses_clean_glyph_and_token() {
        let project = row("repo", None, 0, true, "/r");
        let state = card_lines(&project)[2].to_string();
        assert!(state.contains(theme::GIT_CLEAN_GLYPH));
        assert!(state.contains("CLEAN"));
    }

    #[test]
    fn card_height_is_stable() {
        assert_eq!(CARD_HEIGHT, 4);
    }
}
