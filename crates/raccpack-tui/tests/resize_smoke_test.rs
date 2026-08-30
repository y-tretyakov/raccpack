//! Resize/small-size smoke test: renders every screen and mode on a range of
//! terminal sizes with a `TestBackend`, asserting the renderer never panics on
//! zero-area/very-narrow/very-short layouts.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;

use raccpack_core::app::RaidResult;

use raccpack_tui::app::raid::{FlowPhase, RaidFlow, RaidFlowOptions};
use raccpack_tui::app::sniff::{ProjectRow, ProjectsMode, SniffScreenState};
use raccpack_tui::app::{App, ViewId};
use raccpack_tui::ui::layout::{ACTIVITY_MIN_WIDTH, ACTIVITY_WIDTH};
use raccpack_tui::ui::screens;
use raccpack_tui::ui::widgets::{activity, sidebar};

/// Every terminal size the smoke test walks: 80×24 baseline, wide terminals
/// that unlock the activity panel, and a tiny 40×12 that stresses clamps.
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
        is_loading: false,
        scan_root: PathBuf::from("/workspace"),
        error: None,
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
        },
        raccpack_tui::app::dig::FindingRow {
            path: PathBuf::from("/workspace/project-0/key"),
            risk: raccpack_core::domain::SensitiveRisk::High,
            kind: "aws_access_key_id".into(),
            git_status: "untracked".into(),
        },
    ];
    app.dig_state.reapply_filter();
    app.activity.push(
        raccpack_tui::app::activity::ActivityKind::Ok,
        "scan complete",
    );
    app.activity.push(
        raccpack_tui::app::activity::ActivityKind::Warn,
        "dig project-0 · 2 findings",
    );
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
/// rows, a clamped sidebar, and an optional activity slot on wide terminals.
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

        let (content, activity_slot) = if chunks[1].width >= ACTIVITY_MIN_WIDTH {
            let inner = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(ACTIVITY_WIDTH)])
                .split(chunks[1]);
            (inner[0], Some(inner[1]))
        } else {
            (chunks[1], None)
        };

        screens::render_screen(f, content, app);
        if let Some(slot) = activity_slot {
            activity::render(f, slot, &app.activity);
        }

        if help {
            screens::help::render(f, area);
        }
        if let Some(flow) = &app.raid_flow {
            screens::raid::render(f, area, flow);
        }
    })
    .expect("frame must draw");
}

/// Render every view/mode at every size; a panic would fail the test.
#[test]
fn renders_every_view_at_every_size_without_panic() {
    let mut app = sample_app();
    for &(w, h) in SIZES {
        let sizes: [(ViewId, &str); 4] = [
            (ViewId::Overview, "overview"),
            (ViewId::Projects, "projects"),
            (ViewId::Findings, "findings"),
            (ViewId::Operations, "operations"),
        ];
        for (view, _name) in sizes {
            app.current_view = view;
            for mode in [ProjectsMode::Cards, ProjectsMode::Table, ProjectsMode::Tree] {
                app.sniff_state.mode = mode;
                let backend = TestBackend::new(w, h);
                let mut term = Terminal::new(backend).expect("test backend");
                draw_frame(&mut term, &mut app, false);
            }
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

/// Empty/no-scan state (overview empty hints, sniff empty) must not panic.
#[test]
fn empty_states_render_at_every_size_without_panic() {
    let mut app = App::new();
    for &(w, h) in SIZES {
        for &(view, _name) in &[
            (ViewId::Overview, "overview"),
            (ViewId::Projects, "projects"),
            (ViewId::Findings, "findings"),
            (ViewId::Operations, "operations"),
        ] {
            app.current_view = view;
            let backend = TestBackend::new(w, h);
            let mut term = Terminal::new(backend).expect("test backend");
            draw_frame(&mut term, &mut app, false);
        }
    }
}
