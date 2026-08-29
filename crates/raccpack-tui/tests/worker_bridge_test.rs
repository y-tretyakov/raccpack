//! Regression tests for the B1.2 sniff-screen bridge + loading-state fixes.
//!
//! Case 1 (bridge fix): a `WorkerEvent` emitted by the worker must surface as
//! `AppEvent::Worker(...)` on the UI channel. `run_event_loop` needs a real
//! `Terminal`, so here we mirror its exact wiring (spawn_worker + a bridge
//! thread forwarding `WorkerEvent -> AppEvent`) and drive a deterministic
//! `Sniff` that fails fast, then assert delivery.
//!
//! Case 2 (loading fix): the loading state is set *before* the Sniff message is
//! sent. `handle_app_command` is private, so we verify the observable behavior
//! it relies on at the state level: `SniffScreenState::set_loading(true)` sets
//! `is_loading` and clears `error`. The "loading-set-before-send" wiring itself
//! is introduced by the Dev edit and must be re-confirmed by the Orchestrator on
//! the merged tree.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use raccpack_tui::app::sniff::SniffScreenState;
use raccpack_tui::app::App;
use raccpack_tui::event::AppEvent;
use raccpack_tui::worker::{spawn_worker, WorkerEvent, WorkerMsg};

/// Mirror `run_event_loop`'s worker wiring exactly as the fixed branch sets it
/// up: spawn the worker and forward every `WorkerEvent` into the UI channel as
/// an `AppEvent::Worker`. Without the Dev bridge this receiver would be
/// discarded and `AppEvent::Worker` would never be produced.
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

/// Case 1 — a `WorkerEvent` sent by the worker reaches the UI loop as
/// `AppEvent::Worker`. Uses a nonexistent scan root so the worker emits
/// `SniffDone(Err(_))` quickly and deterministically.
#[test]
fn worker_event_reaches_ui_loop() {
    let (worker_tx, ui_rx) = spawn_bridged_worker();

    worker_tx
        .send(WorkerMsg::Sniff {
            scan_root: PathBuf::from("/nonexistent/b1.2-no-such-root"),
            den_dir: PathBuf::from("/tmp/b1.2-den"),
            force_refresh: true,
            detect_mode: None,
            max_depth: None,
        })
        .unwrap();

    let mut sniff_done = false;
    for _ in 0..20 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(WorkerEvent::SniffDone(result))) => {
                assert!(result.is_err(), "nonexistent root must yield Err");
                sniff_done = true;
                break;
            }
            Ok(_other) => continue,
            Err(_) => break,
        }
    }

    assert!(
        sniff_done,
        "AppEvent::Worker(SniffDone(_)) must be delivered to the UI loop"
    );
}

/// Case 1 (bonus) — the same bridge also forwards `Progress` events as
/// `AppEvent::Worker` while a successful sniff is in flight.
#[test]
fn worker_progress_event_reaches_ui_loop() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"proj\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(project.join("src/main.rs"), "fn main() {}").unwrap();

    let (worker_tx, ui_rx) = spawn_bridged_worker();

    worker_tx
        .send(WorkerMsg::Sniff {
            scan_root: tmp.path().to_path_buf(),
            den_dir: PathBuf::from("/tmp/b1.2-den"),
            force_refresh: true,
            detect_mode: None,
            max_depth: None,
        })
        .unwrap();

    // At minimum a SniffDone must arrive; Progress is optional but exercised.
    let mut done = false;
    for _ in 0..20 {
        match ui_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) => done = true,
            Err(_) => break,
        }
    }
    assert!(
        done,
        "at least one AppEvent::Worker (Progress and/or SniffDone) must arrive"
    );
}

/// Case 2 — `set_loading(true)` flips `is_loading` and clears `error`.
/// This is the observable state change the loading-set-before-send wiring
/// relies on.
#[test]
fn set_loading_true_flips_flag_and_clears_error() {
    let mut state = SniffScreenState {
        error: Some("stale error".into()),
        ..Default::default()
    };

    assert!(!state.is_loading);

    state.set_loading(true);

    assert!(
        state.is_loading,
        "is_loading must be true after set_loading(true)"
    );
    assert!(
        state.error.is_none(),
        "set_loading(true) must clear any previous error"
    );
}

/// Case 2 — `set_loading(false)` clears `is_loading` but leaves `error` alone
/// (error is cleared only when entering the loading state, and set by the
/// SniffDone handler on failure).
#[test]
fn set_loading_false_clears_flag_but_keeps_error() {
    let mut state = SniffScreenState {
        error: Some("reported failure".into()),
        ..Default::default()
    };

    state.set_loading(true);
    state.error = Some("reported failure".into());
    state.set_loading(false);

    assert!(
        !state.is_loading,
        "is_loading must be false after set_loading(false)"
    );
    assert_eq!(state.error.as_deref(), Some("reported failure"));
}

/// Case 2 — the App exposes the SniffScreenState and a fresh App starts not
/// loading, so the sniff command path starts from a known baseline before the
/// loading flag is set.
#[test]
fn new_app_starts_not_loading_with_no_error() {
    let app = App::new();
    assert!(!app.sniff_state.is_loading);
    assert!(app.sniff_state.error.is_none());
}
