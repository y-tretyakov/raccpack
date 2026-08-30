//! Integration test for the B1.3 dig screen: real core dig via the worker
//! bridge, findings mapped into the screen state, and the no-secret-leak
//! invariant (masked content preview never enters TUI state).

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use raccpack_tui::app::dig::DigScreenState;
use raccpack_tui::event::AppEvent;
use raccpack_tui::worker::{spawn_worker, WorkerEvent, WorkerMsg};

use raccpack_core::app::DigResult;

/// Mirror `run_event_loop`'s worker wiring: spawn the worker and forward every
/// `WorkerEvent` into the UI channel as `AppEvent::Worker`.
fn spawn_bridged_worker() -> (mpsc::Sender<WorkerMsg>, mpsc::Receiver<AppEvent>) {
    let (worker_tx, worker_rx) = spawn_worker();
    let (ui_tx, ui_rx) = mpsc::channel::<AppEvent>();

    std::thread::spawn(move || {
        for event in worker_rx {
            if ui_tx.send(AppEvent::Worker(event)).is_err() {
                break;
            }
        }
    });

    (worker_tx, ui_rx)
}

/// Wait for a `DigDone` event, draining progress events along the way.
fn wait_for_dig(
    ui_rx: &mpsc::Receiver<AppEvent>,
) -> Result<DigResult, raccpack_core::domain::Error> {
    for _ in 0..30 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(WorkerEvent::DigDone(result))) => return result,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    panic!("timeout waiting for WorkerEvent::DigDone");
}

/// Build a tiny project that dig can scan: a bare dir with a `.env` that also
/// carries a detectable AWS secret in its content.
fn env_project(tmp: &tempfile::TempDir, content: &str) -> PathBuf {
    let project = tmp.path().join("srv");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join(".env"), content).unwrap();
    project
}

#[test]
fn dig_screen_loads_findings_from_fixture_and_never_leaks_masked_value() {
    let tmp = tempfile::tempdir().unwrap();
    let project = env_project(&tmp, "AWS_SECRET_ACCESS_KEY = \"SECRET-RAW-VALUE-123456\"");

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::Dig {
            project: project.clone(),
            den_dir: PathBuf::from("/tmp/b1.3-den"),
            scan_content: true,
        })
        .unwrap();

    let result = wait_for_dig(&ui_rx).expect("dig run should succeed");
    let file = result
        .files
        .iter()
        .find(|f| f.path.ends_with(".env"))
        .expect(".env fixture must be reported");

    // The core run fed the app a masked preview only.
    let masked = file
        .content_match
        .as_ref()
        .map(|m| m.masked.clone())
        .expect("content scan must surface a masked content match");

    // Feed the result into the actual screen state (what DigDone does).
    let mut state = DigScreenState::default();
    state.set_dig_result(result);
    assert_eq!(state.all_findings.len(), 1);
    assert_eq!(state.findings.len(), 1);
    assert_eq!(state.selected_finding().unwrap().path, project.join(".env"));

    // No-secret-leak invariant: the masked preview (nor the raw value) may not
    // survive inside the TUI state — rows carry metadata only.
    let debug = format!("{:?}", state);
    assert!(
        !debug.contains(&masked),
        "masked value leaked into screen state: {debug}"
    );
    assert!(
        !debug.contains("SECRET-RAW-VALUE"),
        "raw value leaked: {debug}"
    );
}

#[test]
fn dig_without_content_scan_keeps_filename_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let project = env_project(&tmp, "AWS_SECRET_ACCESS_KEY = \"SECRET-RAW-VALUE-123456\"");

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::Dig {
            project,
            den_dir: PathBuf::from("/tmp/b1.3-den"),
            scan_content: false,
        })
        .unwrap();

    let result = wait_for_dig(&ui_rx).expect("dig run should succeed");
    let file = result
        .files
        .iter()
        .find(|f| f.path.ends_with(".env"))
        .expect(".env filename must still be reported without content scan");
    assert!(
        file.content_match.is_none(),
        "no content match may be produced when scan_content is false"
    );
}

#[test]
fn dig_missing_project_reports_error() {
    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::Dig {
            project: PathBuf::from("/nonexistent-b1.3-root"),
            den_dir: PathBuf::from("/tmp/b1.3-den"),
            scan_content: true,
        })
        .unwrap();

    let result = wait_for_dig(&ui_rx);
    assert!(
        result.is_err(),
        "digging a nonexistent project must surface an error"
    );
}

/// B1.5 case 10 — the raw secret literal that lives in the scanned file must
/// never appear in `DigScreenState`/`FindingRow` Debug after a full content dig,
/// even though the file itself is re-readable at reveal time. This holds whether
/// or not a content ref is present on the row; the TUI state carries metadata
/// only.
#[test]
fn dig_state_never_contains_raw_secret_literal() {
    let tmp = tempfile::tempdir().unwrap();
    // A distinctive AWS access key value; the raw literal must never enter the
    // TUI screen state (only the masked preview does).
    let raw = "AKIARAWVALUE1234567890AB";
    let project = tmp.path().join("srv");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("creds.txt"), format!("{raw}\n")).unwrap();

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::Dig {
            project: project.clone(),
            den_dir: PathBuf::from("/tmp/b1.5-den"),
            scan_content: true,
        })
        .unwrap();

    let result = wait_for_dig(&ui_rx).expect("dig run should succeed");
    assert!(
        result.files.iter().any(|f| f.path.ends_with("creds.txt")),
        "creds.txt must be reported as a finding"
    );

    let mut state = DigScreenState::default();
    state.set_dig_result(result);

    let debug = format!("{state:?}");
    assert!(
        !debug.contains(raw),
        "raw secret literal leaked into DigScreenState Debug: {debug}"
    );
    for row in &state.findings {
        let row_debug = format!("{row:?}");
        assert!(
            !row_debug.contains(raw),
            "raw secret literal leaked into FindingRow Debug: {row_debug}"
        );
    }
}
