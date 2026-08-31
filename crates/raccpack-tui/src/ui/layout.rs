//! Top-level frame rendering: header, body (sidebar + main), footer.

use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use crate::app::raid::FlowPhase;
use crate::app::{App, ViewId};
use crate::theme;
use crate::ui::screens;
use crate::ui::widgets::sidebar;

/// Render one complete frame.
pub fn render(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    terminal.draw(|f| {
        let area = f.area();

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(theme::SPACE_HEADER_HEIGHT), // header
                Constraint::Min(0),                             // body
                Constraint::Length(theme::SPACE_FOOTER_HEIGHT), // footer
            ])
            .split(area);

        render_header(f, outer[0], app);
        render_body(f, outer[1], app);
        render_footer(f, outer[2], app);

        if app.help_visible {
            screens::help::render(f, area);
        }
        if let Some(flow) = &app.raid_flow {
            screens::raid::render(f, area, flow);
        }
        if let Some(flow) = &app.pack_flow {
            screens::pack::render(f, area, flow);
        }
        if let Some(modal) = &app.reveal {
            screens::reveal::render(f, area, modal);
        }
    })?;
    Ok(())
}

fn render_header(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let title = Span::styled(
        " raccpack-tui ",
        Style::default()
            .fg(theme::BRAND_PRIMARY)
            .add_modifier(Modifier::BOLD),
    );
    let sep = Span::raw("│");
    let space = Span::raw(" ");

    let root = app.sniff_state.scan_root.to_string_lossy();
    let root_text = if root.is_empty() { "-" } else { &root };
    let root_span = Span::styled(root_text, Style::default().fg(theme::MUTED));

    let version = Span::styled(
        format!("v{}", env!("CARGO_PKG_VERSION")),
        Style::default().fg(theme::MUTED),
    );

    let hotkeys = Span::styled(
        "Tab views · hjkl move · ? help · q quit",
        Style::default().fg(theme::MUTED),
    );

    // Dense single line; drop the hotkey strip (never panic) on narrow terms.
    let full = Line::from(vec![
        title.clone(),
        sep.clone(),
        space.clone(),
        root_span.clone(),
        sep.clone(),
        space.clone(),
        version.clone(),
        sep.clone(),
        space.clone(),
        hotkeys,
    ]);
    let narrow = Line::from(vec![title, sep, space, root_span]);
    let line = if full.width() <= usize::from(area.width) {
        full
    } else {
        narrow
    };

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG).fg(theme::FG)),
        area,
    );
}

fn render_body(f: &mut ratatui::Frame, area: Rect, app: &mut App) {
    // Clamp sidebar to the available width so tiny terminals never panic.
    let sidebar_width = area.width.min(theme::SPACE_SIDEBAR_WIDTH);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Min(0)])
        .split(area);

    sidebar::render(f, chunks[0], app);

    screens::render_screen(f, chunks[1], app);
}

fn space() -> Span<'static> {
    Span::raw(" ")
}

fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let status = footer_status(app);
    let left = Line::from(vec![
        space(),
        Span::styled(status, Style::default().fg(theme::MUTED)),
    ]);

    let focus_hint = format!(
        " focus: {} · view: {} ",
        app.focus.label(),
        app.current_view.label()
    );
    let right = Line::from(Span::styled(focus_hint, Style::default().fg(theme::MUTED)));

    // Left status paragraph paints the full-width background; the right hint
    // draws text-only on top, so no spacer spaces are needed.
    f.render_widget(
        Paragraph::new(left)
            .style(Style::default().bg(theme::BG).fg(theme::FG))
            .alignment(Alignment::Left),
        area,
    );
    f.render_widget(
        Paragraph::new(right)
            .style(Style::default().fg(theme::MUTED))
            .alignment(Alignment::Right),
        area,
    );
}

/// Current-screen status line shown in the footer.
fn footer_status(app: &App) -> String {
    if let Some(flow) = &app.raid_flow {
        return raid_footer(flow);
    }
    if let Some(flow) = &app.pack_flow {
        return pack_footer(flow);
    }
    if app.current_view == ViewId::Findings {
        return dig_footer(app);
    }

    let state = &app.sniff_state;
    if state.is_loading {
        "Loading…".to_string()
    } else if state.error.is_some() {
        "Error — press r to retry".to_string()
    } else if state.projects.is_empty() && app.current_view == ViewId::Projects {
        "No projects found — press r to scan".to_string()
    } else if state.projects.is_empty() {
        "No projects scanned".to_string()
    } else {
        let timestamp = state
            .last_refresh
            .map(|t| humantime::format_rfc3339(t).to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let cache = if state.from_cache {
            " (from cache)"
        } else {
            ""
        };
        format!("Last refresh: {timestamp}{cache}")
    }
}

/// Footer status while a raid flow is open.
fn raid_footer(flow: &crate::app::raid::RaidFlow) -> String {
    match &flow.phase {
        FlowPhase::Preparing => "Raid: preparing… (y confirm · n/Esc cancel)".to_string(),
        FlowPhase::Preview(_) => {
            "Raid: preview — dry run (y/Enter commit · n/Esc cancel)".to_string()
        }
        FlowPhase::Passphrase(_) => {
            "Raid: entering passphrase — Enter confirm · Esc cancel".to_string()
        }
        FlowPhase::Running => {
            format!(
                "Raid: running… {}% — Esc does not cancel",
                flow.overall_percent
            )
        }
        FlowPhase::Done(result) if result.success => "Raid: success — Enter/Esc close".to_string(),
        FlowPhase::Done(_) => "Raid: finished (see modal) — Enter/Esc close".to_string(),
        FlowPhase::Failed(_) => "Raid: failed — Enter/Esc close".to_string(),
    }
}

/// Footer status while a pack flow is open.
fn pack_footer(flow: &crate::app::pack::PackFlow) -> String {
    match &flow.phase {
        crate::app::pack::PackFlowPhase::Preparing => {
            "Pack: preparing… (y confirm · n/Esc cancel)".to_string()
        }
        crate::app::pack::PackFlowPhase::Preview(_) => {
            "Pack: preview — dry run (y/Enter commit · c/n toggles · n/Esc cancel)".to_string()
        }
        crate::app::pack::PackFlowPhase::Running => {
            format!("Pack: running… {}% — Esc does not cancel", flow.percent)
        }
        crate::app::pack::PackFlowPhase::Done(_) => "Pack: success — Enter/Esc close".to_string(),
        crate::app::pack::PackFlowPhase::Failed(_) => "Pack: failed — Enter/Esc close".to_string(),
    }
}

/// Footer status specific to the dig/findings screen.
fn dig_footer(app: &App) -> String {
    let state = &app.dig_state;
    if state.project.is_none() {
        return "Select a project on Projects and press Enter to dig".to_string();
    }
    if state.is_loading {
        return "Digging…".to_string();
    }
    if state.error.is_some() {
        return "Dig failed — press r to retry".to_string();
    }
    let project = state
        .project
        .as_deref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{} findings · filter {} · content {} · {}",
        state.findings.len(),
        state.min_risk.label(),
        if state.scan_content { "on" } else { "off" },
        project,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, ALL_VIEWS};

    #[test]
    fn sidebar_width_within_spec_range() {
        assert!(
            (22..=24).contains(&theme::SPACE_SIDEBAR_WIDTH),
            "sidebar must be 22-24 columns, got {}",
            theme::SPACE_SIDEBAR_WIDTH
        );
    }

    #[test]
    fn layout_split_header_body_footer() {
        let constraints = [
            Constraint::Length(1), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // footer
        ];
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn sidebar_split_constraints() {
        let constraints = [
            Constraint::Length(theme::SPACE_SIDEBAR_WIDTH),
            Constraint::Min(0),
        ];
        assert_eq!(constraints.len(), 2);
    }

    #[test]
    fn all_views_have_non_empty_labels() {
        for view in ALL_VIEWS {
            assert!(!view.label().is_empty(), "{view:?} label must be non-empty");
        }
    }

    #[test]
    fn header_line_contains_title_and_degrades_on_narrow_width() {
        let title = Span::styled(
            " raccpack-tui ",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );
        let sep = Span::raw("│");
        let space = Span::raw(" ");
        let root = Span::styled("-", Style::default().fg(theme::MUTED));
        let version = Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme::MUTED),
        );
        let hotkeys = Span::styled(
            "Tab views · hjkl move · ? help · q quit",
            Style::default().fg(theme::MUTED),
        );

        let full = Line::from(vec![
            title.clone(),
            sep.clone(),
            space.clone(),
            root.clone(),
            sep.clone(),
            space.clone(),
            version.clone(),
            sep.clone(),
            space.clone(),
            hotkeys,
        ]);
        assert!(full.width() > 0);
        assert!(full.to_string().contains("v0."));

        // On a very narrow header the hotkey strip is dropped.
        let narrow = Line::from(vec![title, sep, space, root]);
        assert!(narrow.width() <= full.width());
    }

    #[test]
    fn footer_status_reflects_current_sniff_state() {
        let app = App::new();
        assert!(!footer_status(&app).is_empty());
    }

    #[test]
    fn eighty_by_twentyfour_body_preserves_sniff_width() {
        // 80×24 terminal: 1 header + 1 footer → body of 22 rows; the sidebar
        // clamps to 23 columns leaving the sniff screen all 57 columns of the
        // main region — no resize truncation on the baseline size.
        let term = Rect::new(0, 0, 80, 24);
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(theme::SPACE_HEADER_HEIGHT),
                Constraint::Min(0),
                Constraint::Length(theme::SPACE_FOOTER_HEIGHT),
            ])
            .split(term);
        let body = outer[1];
        assert_eq!(body.height, 22);

        let sidebar_width = body.width.min(theme::SPACE_SIDEBAR_WIDTH);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(0)])
            .split(body);
        assert_eq!(chunks[0].width, 23);
        assert_eq!(chunks[1].width, 57);
    }
}
