//! Sidebar widget — brand block, nav with live badges, version footer.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Focus, ViewId, ALL_VIEWS};
use crate::theme;

/// Render the sidebar into `area`.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let lines = build_lines(app, area.width);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme::SURFACE).fg(theme::FG)),
        area,
    );
}

/// Build every sidebar line: brand block, nav with badges, version footer.
fn build_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let sidebar_focused = app.focus == Focus::Sidebar;
    let mut lines = Vec::new();

    // Brand block: identity line over a muted workspace caption.
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "◈ RACCPACK",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("workspace", Style::default().fg(theme::MUTED)),
    ]));

    // Blank spacer between the brand block and the nav.
    lines.push(Line::raw(""));

    for view in ALL_VIEWS {
        let active = app.current_view == view;
        lines.push(nav_line(
            view,
            active,
            sidebar_focused,
            nav_badge(app, view),
            width,
        ));
    }

    // Divider, then the version footer.
    lines.push(Line::from(vec![Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(theme::BORDER),
    )]));
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme::MUTED),
        ),
    ]));

    lines
}

/// Optional tally shown on the right of a nav row. `None` = no badge.
fn nav_badge(app: &App, view: ViewId) -> Option<(String, Style)> {
    match view {
        ViewId::Projects => {
            let n = app.sniff_state.projects.len();
            Some((n.to_string(), Style::default().fg(theme::BRAND_PRIMARY)))
        }
        ViewId::Findings => {
            // No dig run yet → placeholder; a real run of zero → muted `0`;
            // any findings → warning amber so the count stands out. The number
            // itself is always readable (NO_COLOR-safe), colour is a hint.
            if app.dig_state.last_run.is_none() {
                Some((
                    theme::EMPTY_PLACEHOLDER.to_string(),
                    Style::default().fg(theme::MUTED),
                ))
            } else if app.dig_state.findings.is_empty() {
                Some(("0".to_string(), Style::default().fg(theme::MUTED)))
            } else {
                Some((
                    app.dig_state.findings.len().to_string(),
                    Style::default().fg(theme::WARNING),
                ))
            }
        }
        ViewId::Overview | ViewId::Operations => None,
    }
}

/// One nav row: rail + label + stretch pad + key hint + optional badge.
///
/// `width` is the sidebar column count; the row pads so the hint and badge sit
/// on the right edge. Rows with no badge flush the hint to the edge as before.
fn nav_line(
    view: ViewId,
    active: bool,
    sidebar_focused: bool,
    badge: Option<(String, Style)>,
    width: u16,
) -> Line<'static> {
    let item_style = if active && sidebar_focused {
        Style::default()
            .fg(theme::FG)
            .bg(theme::SURFACE_RAISED)
            .add_modifier(Modifier::BOLD)
    } else if active {
        Style::default()
            .fg(theme::FOCUS)
            .bg(theme::SURFACE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::MUTED)
    };

    let bar = Span::styled(
        if active { "▎" } else { " " },
        Style::default().fg(theme::FOCUS),
    );
    let label = Span::styled(view.label(), item_style);

    let prefix_width = 3 + view.label().chars().count();
    let badge_len = badge.as_ref().map(|(s, _)| s.chars().count()).unwrap_or(0);
    let trailing = 1 + badge_len + usize::from(badge.is_some());
    let pad = width.saturating_sub(prefix_width as u16 + trailing as u16);

    let mut spans = vec![
        Span::raw(" "),
        bar,
        space(),
        label,
        Span::raw(" ".repeat(pad as usize)),
        Span::styled(view.key().to_string(), Style::default().fg(theme::MUTED)),
    ];
    if let Some((text, badge_style)) = badge {
        spans.push(space());
        spans.push(Span::styled(text, badge_style));
    }
    Line::from(spans)
}

fn space() -> Span<'static> {
    Span::raw(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn width_within_spec_range() {
        assert!(
            (22..=24).contains(&theme::SPACE_SIDEBAR_WIDTH),
            "sidebar must be 22-24 columns, got {}",
            theme::SPACE_SIDEBAR_WIDTH
        );
    }

    #[test]
    fn brand_block_present() {
        let app = App::new();
        let lines = build_lines(&app, theme::SPACE_SIDEBAR_WIDTH);
        let joined: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(joined.iter().any(|l| l.contains("RACCPACK")));
        assert!(joined.iter().any(|l| l.contains("workspace")));
    }

    #[test]
    fn version_footer_present() {
        let app = App::new();
        let lines = build_lines(&app, theme::SPACE_SIDEBAR_WIDTH);
        assert!(lines.iter().any(|l| l.to_string().contains("v0.")));
    }

    #[test]
    fn all_views_have_non_empty_labels() {
        for view in ALL_VIEWS {
            assert!(!view.label().is_empty(), "{view:?} label must be non-empty");
        }
    }

    fn project(name: &str) -> crate::app::sniff::ProjectRow {
        crate::app::sniff::ProjectRow {
            name: name.into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::from("/").join(name),
        }
    }

    #[test]
    fn brand_line_is_exact_mark() {
        let app = App::new();
        let lines = build_lines(&app, theme::SPACE_SIDEBAR_WIDTH);
        let joined: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(
            joined.iter().any(|l| l.trim().ends_with("◈ RACCPACK")),
            "brand block must render the `◈ RACCPACK` mark, got {joined:?}"
        );
    }

    #[test]
    fn version_footer_matches_cargo_pkg_version() {
        let app = App::new();
        let lines = build_lines(&app, theme::SPACE_SIDEBAR_WIDTH);
        let expected = format!("v{}", env!("CARGO_PKG_VERSION"));
        let joined: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(
            joined.iter().any(|l| l.trim() == expected),
            "footer must exactly read {expected:?}, got {joined:?}"
        );
        // V2 phase is «no bump»: workspace is still at 0.4.4.
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.4.4");
    }

    #[test]
    fn nav_rows_fill_every_sidebar_width_22_to_24() {
        let app = App::new();
        for width in 22u16..=24 {
            for view in ALL_VIEWS {
                let line = nav_line(view, true, false, nav_badge(&app, view), width);
                let displayed: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
                assert_eq!(
                    displayed, width as usize,
                    "{view:?} at width {width} must exactly fill the column"
                );
            }
        }
    }

    #[test]
    fn badge_count_is_text_not_just_color() {
        // NO_COLOR-safe: the tally is real glyph text on the row, independent
        // of the badge style — dropping colour must not lose the number.
        let mut app = App::new();
        app.sniff_state.projects.push(project("a"));
        let line = nav_line(
            ViewId::Projects,
            false,
            false,
            nav_badge(&app, ViewId::Projects),
            theme::SPACE_SIDEBAR_WIDTH,
        );
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains('1'),
            "projects count must survive without colour: {text:?}"
        );

        let mut app = App::new();
        app.dig_state.last_run = Some(std::time::SystemTime::now());
        app.dig_state.findings.push(crate::app::dig::FindingRow {
            path: std::path::PathBuf::from("/a"),
            risk: raccpack_core::domain::SensitiveRisk::High,
            kind: ".env".into(),
            git_status: "tracked".into(),
        });
        let line = nav_line(
            ViewId::Findings,
            false,
            false,
            nav_badge(&app, ViewId::Findings),
            theme::SPACE_SIDEBAR_WIDTH,
        );
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains('1'),
            "findings count must survive without colour: {text:?}"
        );
    }

    #[test]
    fn projects_badge_counts_projects() {
        let mut app = App::new();
        assert_eq!(nav_badge(&app, ViewId::Projects).unwrap().0, "0");
        app.sniff_state
            .projects
            .push(crate::app::sniff::ProjectRow {
                name: "a".into(),
                language: None,
                frameworks: vec![],
                size_bytes: 0,
                is_git_repo: false,
                path: std::path::PathBuf::from("/a"),
            });
        assert_eq!(nav_badge(&app, ViewId::Projects).unwrap().0, "1");
    }

    #[test]
    fn findings_badge_hidden_until_dig_runs() {
        let app = App::new();
        assert_eq!(
            nav_badge(&app, ViewId::Findings).unwrap().0,
            theme::EMPTY_PLACEHOLDER
        );
    }

    #[test]
    fn findings_badge_zero_when_run_clean() {
        let mut app = App::new();
        app.dig_state.last_run = Some(std::time::SystemTime::now());
        let (text, style) = nav_badge(&app, ViewId::Findings).unwrap();
        assert_eq!(text, "0");
        assert_eq!(style.fg, Some(theme::MUTED));
    }

    #[test]
    fn findings_badge_warns_when_present() {
        let mut app = App::new();
        app.dig_state.last_run = Some(std::time::SystemTime::now());
        app.dig_state.findings.push(crate::app::dig::FindingRow {
            path: std::path::PathBuf::from("/a"),
            risk: raccpack_core::domain::SensitiveRisk::High,
            kind: ".env".into(),
            git_status: "tracked".into(),
        });
        let (text, style) = nav_badge(&app, ViewId::Findings).unwrap();
        assert_eq!(text, "1");
        assert_eq!(style.fg, Some(theme::WARNING));
    }

    #[test]
    fn badge_rows_pad_to_width() {
        // Every nav row must fill the sidebar width exactly: leading space +
        // bar + space + label + pad + key hint (+ space + badge when present).
        let app = App::new();
        let width = theme::SPACE_SIDEBAR_WIDTH;
        for view in ALL_VIEWS {
            let badge = nav_badge(&app, view);
            let badge_len = badge.as_ref().map(|(s, _)| s.chars().count()).unwrap_or(0);
            let prefix = 3 + view.label().chars().count();
            let trailing = 1 + badge_len + usize::from(badge.is_some());
            let pad = width.saturating_sub(prefix as u16 + trailing as u16);
            let total = prefix + pad as usize + trailing;
            assert_eq!(
                total, width as usize,
                "{view:?} row (len {total}) must fill sidebar width {width}"
            );
        }
    }

    #[test]
    fn nav_lines_never_overflow_sidebar() {
        let app = App::new();
        let width = theme::SPACE_SIDEBAR_WIDTH;
        for view in ALL_VIEWS {
            let line = nav_line(view, false, false, nav_badge(&app, view), width);
            // Count display width: non-styled glyphs are 1 cell each.
            let displayed: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            assert!(
                displayed <= width as usize,
                "{view:?} nav row (len {displayed}) must stay within {width}"
            );
        }
    }

    #[test]
    fn focus_label_matches_footer_hint() {
        assert_eq!(Focus::Sidebar.label(), "sidebar");
        assert_eq!(Focus::Main.label(), "main");
    }
}
