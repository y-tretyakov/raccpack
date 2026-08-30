//! Resize/small-size smoke test: renders every screen and overlay on a range of
//! terminal sizes with a `TestBackend`, asserting the renderer never panics on
//! zero-area/very-narrow/very-short layouts.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;

use raccpack_core::app::RaidResult;

use raccpack_tui::app::raid::{FlowPhase, RaidFlow, RaidFlowOptions};
use raccpack_tui::app::sniff::{ProjectRow, SniffScreenState};
use raccpack_tui::app::{App, ViewId};
use raccpack_tui::ui::screens;
use raccpack_tui::ui::widgets::sidebar;

/// Every terminal size the smoke test walks: an 80×24 baseline, wide terminals
/// and a tiny 40×12 that stresses the clamps.
const SIZES: &[(u16, u16)] = &[(80, 24), (120, 30), (160, 40), (40, 12)];
const SIDEBAR_WIDTH: u16 = 23;

fn sample_state(n: usize) -> SniffScreenState {
    SniffScreenState {
        projects: (0..n)
            .map(|i| ProjectRow {
                name: format!("project-{i}"),
                language: Some("Rust".into()),
                frameworks: vec!["Axum".into()],
                size_bytes: (i as u64 + 1) * 1024 * 1024,
                is_git_repo: i % 2 == 0,
                path: PathBuf::from(format!("/workspace/project-{i}")),
            })
            .collect(),
        total_size: 1234,
        scan_root: PathBuf::from("/workspace"),
        ..Default::default()
    }
}

fn sample_app() -> App {
    let mut app = App::new();
    app.sniff_state = sample_state(10);
    app.sniff_state.table_state.select(Some(3));
    app.dig_state.project = Some(PathBuf::from("/workspace/project-0"));
    app.dig_state.all_findings = vec![
        raccpack_tui::app::dig::FindingRow {
            path: PathBuf::from("/workspace/project-0/.env"),
            risk: raccpack_core::domain::SensitiveRisk::Critical,
            kind: ".env".into(),
            git_status: "tracked".into(),
            content_ref: None,
        },
        raccpack_tui::app::dig::FindingRow {
            path: PathBuf::from("/workspace/project-0/key"),
            risk: raccpack_core::domain::SensitiveRisk::High,
            kind: "aws_access_key_id".into(),
            git_status: "untracked".into(),
            content_ref: None,
        },
    ];
    app.dig_state.reapply_filter();
    app
}

fn preview_result() -> RaidResult {
    RaidResult {
        project_path: PathBuf::from("/workspace/project-0"),
        stages: Vec::new(),
        stash: None,
        rinse: None,
        pack: None,
        den_artifacts: vec![
            PathBuf::from("/workspace/den/packs/2026/08/x.tar.zst"),
            PathBuf::from("/workspace/den/secrets/2026/08/y.age"),
        ],
        success: true,
        dry_run: true,
        rolled_back: false,
        rollback_warnings: Vec::new(),
    }
}

fn sample_flow() -> RaidFlow {
    let mut flow = RaidFlow::new(
        PathBuf::from("/workspace/project-0"),
        PathBuf::from("/workspace/den"),
        RaidFlowOptions::default(),
    );
    flow.phase = FlowPhase::Preview(preview_result());
    flow
}

/// Draw one full frame exactly as `ui::layout::render` would: header+footer
/// rows and a clamped sidebar on the left of the body.
fn draw_frame(term: &mut Terminal<TestBackend>, app: &mut App, help: bool) {
    term.draw(|f| {
        let area = f.area();
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let sidebar_width = outer[1].width.min(SIDEBAR_WIDTH);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(0)])
            .split(outer[1]);
        sidebar::render(f, chunks[0], app);

        screens::render_screen(f, chunks[1], app);

        if help {
            screens::help::render(f, area);
        }
        if let Some(flow) = &app.raid_flow {
            screens::raid::render(f, area, flow);
        }
        if let Some(modal) = &app.reveal {
            screens::reveal::render(f, area, modal);
        }
    })
    .expect("frame must draw");
}

/// Render every view at every size; a panic would fail the test.
#[test]
fn renders_every_view_at_every_size_without_panic() {
    let mut app = sample_app();
    for &(w, h) in SIZES {
        for view in [
            ViewId::Overview,
            ViewId::Projects,
            ViewId::Findings,
            ViewId::Operations,
        ] {
            app.current_view = view;
            let backend = TestBackend::new(w, h);
            let mut term = Terminal::new(backend).expect("test backend");
            draw_frame(&mut term, &mut app, false);
        }
    }
}

/// The help overlay and the raid modal must not panic on narrow/short terms.
#[test]
fn overlays_render_at_every_size_without_panic() {
    let mut app = sample_app();
    for &(w, h) in SIZES {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).expect("test backend");
        draw_frame(&mut term, &mut app, true);
    }

    let mut app = sample_app();
    app.raid_flow = Some(sample_flow());
    for &(w, h) in SIZES {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).expect("test backend");
        draw_frame(&mut term, &mut app, false);
    }
}

/// The reveal modal (both confirm and ready-with-value phases) must not panic on
/// narrow/short terms. The value is only parsed for the render, never asserted.
#[test]
fn reveal_modal_renders_at_every_size_without_panic() {
    use raccpack_core::secrets::FindingRef;
    use raccpack_tui::app::reveal::RevealModal;
    use raccpack_tui::worker::WorkerRevealSecret;

    for &(w, h) in SIZES {
        let mut app = sample_app();
        let reference = FindingRef {
            path: PathBuf::from("/workspace/project-0/.env"),
            marker_id: "aws_access_key".to_string(),
            line: 1,
            value_hash: "deadbeef".to_string(),
        };
        app.reveal = Some(RevealModal::new(
            PathBuf::from("/workspace/project-0/.env"),
            reference,
        ));
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).expect("test backend");
        draw_frame(&mut term, &mut app, false);
    }

    for &(w, h) in SIZES {
        let mut app = sample_app();
        let mut modal = RevealModal::new(
            PathBuf::from("/workspace/project-0/.env"),
            FindingRef {
                path: PathBuf::from("/workspace/project-0/.env"),
                marker_id: "aws_access_key".to_string(),
                line: 1,
                value_hash: "deadbeef".to_string(),
            },
        );
        modal.set_ready(WorkerRevealSecret::new(
            "AKIASUPERSECRETVALUE123".to_string(),
        ));
        app.reveal = Some(modal);
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).expect("test backend");
        draw_frame(&mut term, &mut app, false);
    }
}

/// Empty/no-scan state (overview hints, sniff empty, dig empty) must not panic.
#[test]
fn empty_states_render_at_every_size_without_panic() {
    let mut app = App::new();
    for &(w, h) in SIZES {
        for &view in &[
            ViewId::Overview,
            ViewId::Projects,
            ViewId::Findings,
            ViewId::Operations,
        ] {
            app.current_view = view;
            let backend = TestBackend::new(w, h);
            let mut term = Terminal::new(backend).expect("test backend");
            draw_frame(&mut term, &mut app, false);
        }
    }
}
