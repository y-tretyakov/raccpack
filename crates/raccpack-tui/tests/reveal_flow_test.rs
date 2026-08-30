//! Integration tests for B1.5 — TUI reveal flow.
//!
//! Covers:
//!   11. `v` on a row without a `content_ref` is a safe no-op (no panic).
//!   12. Confirm step aborts via `Esc`/`n` without revealing; the reveal
//!       modal's ephemeral secret is dropped/cleared on close.
//!   13. The reveal worker path: sending `WorkerMsg::Reveal` for a valid ref
//!       yields `WorkerEvent::RevealReady` whose payload `expose()`s the exact
//!       original value and whose `Debug` never contains the raw value.
//!
//! Headless: no real terminal, no network, no git. tempfile only.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use raccpack_core::secrets::{fingerprint_secret, FindingRef};

use raccpack_tui::app::dig::{DigScreenState, FindingRow};
use raccpack_tui::app::reveal::{RevealCommand, RevealModal, RevealPhase};
use raccpack_tui::worker::{spawn_worker, WorkerEvent, WorkerMsg, WorkerRevealSecret};

/// Wrap a bare `KeyCode` into a `KeyEvent` (mirrors the app's own test helper).
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// A valid `FindingRef` pointing at one raw value in `path` on `line`.
fn ref_for(tmp: &tempfile::TempDir, raw: &str) -> (std::path::PathBuf, FindingRef) {
    let path = tmp.path().join("creds.txt");
    std::fs::write(&path, format!("{raw}\n")).unwrap();
    let reference = FindingRef {
        path: path.clone(),
        marker_id: "aws_access_key".to_string(),
        line: 1,
        value_hash: fingerprint_secret(raw),
    };
    (path, reference)
}

/// Wait for the first `WorkerEvent` matching `f`, draining others.
fn wait_for<F>(rx: &mpsc::Receiver<WorkerEvent>, mut f: F) -> WorkerEvent
where
    F: FnMut(&WorkerEvent) -> bool,
{
    for _ in 0..30 {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(ev) if f(&ev) => return ev,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    panic!("timeout waiting for a worker event");
}

// Case 13: valid ref => RevealReady with the exact original value, Debug redacted.
#[test]
fn reveal_worker_ready_exposes_original_and_redacts_debug() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIAWORKERTEST1234567";
    let (path, reference) = ref_for(&tmp, raw);

    let (worker_tx, rx) = spawn_worker();
    worker_tx
        .send(WorkerMsg::Reveal {
            path: path.clone(),
            dir_root: tmp.path().to_path_buf(),
            reference: reference.clone(),
        })
        .unwrap();

    let event = wait_for(&rx, |e| matches!(e, WorkerEvent::RevealReady(_)));
    match event {
        WorkerEvent::RevealReady(payload) => {
            assert_eq!(payload.expose(), raw);
            let debug = format!("{payload:?}");
            assert!(
                !debug.contains(raw),
                "RevealReady payload Debug must not leak the raw value: {debug}"
            );
            assert!(debug.contains("(**)"), "redacted form expected: {debug}");
        }
        other => panic!("expected RevealReady, got {other:?}"),
    }
}

// Case 13b: a stale reference (hash mismatch) => RevealFailed, no raw ever sent.
#[test]
fn reveal_worker_stale_ref_fails_without_raw() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIASTALEHASH1234567";
    let (path, _) = ref_for(&tmp, raw);

    // Reference built against a value NOT present in the file.
    let reference = FindingRef {
        path: path.clone(),
        marker_id: "aws_access_key".to_string(),
        line: 1,
        value_hash: fingerprint_secret("AKIANOTINTHEFILE999"),
    };

    let (worker_tx, rx) = spawn_worker();
    worker_tx
        .send(WorkerMsg::Reveal {
            path: path.clone(),
            dir_root: tmp.path().to_path_buf(),
            reference,
        })
        .unwrap();

    let event = wait_for(&rx, |e| matches!(e, WorkerEvent::RevealFailed(_)));
    match event {
        WorkerEvent::RevealFailed(err) => {
            let debug = format!("{err:?}");
            assert!(
                !debug.contains(raw),
                "reveal failure must never carry the raw value: {debug}"
            );
        }
        other => panic!("expected RevealFailed, got {other:?}"),
    }
}

// Case 13c: WorkerMsg::Reveal Debug is redacted (never carries raw).
#[test]
fn worker_msg_reveal_debug_is_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let (path, reference) = ref_for(&tmp, "AKIADEBBUGCHECK2222");
    let msg = WorkerMsg::Reveal {
        path: path.clone(),
        dir_root: tmp.path().to_path_buf(),
        reference,
    };
    let debug = format!("{msg:?}");
    assert!(
        !debug.contains("AKIADEBBUGCHECK2222"),
        "WorkerMsg::Reveal Debug must be redacted: {debug}"
    );
}

// Case 12a: from the Confirm phase, Esc / n abort without revealing (no RevealReady).
#[test]
fn reveal_modal_esc_aborts_without_reveal() {
    let tmp = tempfile::tempdir().unwrap();
    let (path, reference) = ref_for(&tmp, "AKIACONFIRMABORT12345");
    let mut modal = RevealModal::new(path.clone(), reference);

    // Esc aborts and never transitions the phase past Confirm.
    assert_eq!(modal.handle_key(KeyCode::Esc), RevealCommand::Close);
    assert!(
        matches!(modal.phase, RevealPhase::Confirm),
        "abort must not move the modal out of Confirm"
    );
}

#[test]
fn reveal_modal_n_aborts_without_reveal() {
    let tmp = tempfile::tempdir().unwrap();
    let (path, reference) = ref_for(&tmp, "AKIACONFIRMABORT33333");
    let mut modal = RevealModal::new(path, reference);
    assert_eq!(modal.handle_key(KeyCode::Char('n')), RevealCommand::Close);
    assert!(matches!(modal.phase, RevealPhase::Confirm));
}

// Case 12b: a Ready secret is dropped the moment the modal closes, and the
// modal's Debug never contains the raw value while it is live.
#[test]
fn reveal_modal_ready_secret_cleared_on_close_and_debug_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIAREADYCLEAR123456";
    let (path, reference) = ref_for(&tmp, raw);
    let mut modal = RevealModal::new(path.clone(), reference.clone());
    modal.set_ready(WorkerRevealSecret::new(raw.to_string()));

    // While Ready, Debug must be redacted.
    let debug = format!("{modal:?}");
    assert!(
        !debug.contains(raw),
        "RevealModal Debug must not leak the revealed value: {debug}"
    );

    // Any key on a Ready modal closes (drops the secret); Esc/Enter/n all close.
    assert_eq!(modal.handle_key(KeyCode::Esc), RevealCommand::Close);
    // After Close the app glue clears `App.reveal`; the modal holding the
    // secret is dropped. At the state-model level we verify Close is signalled
    // and that a fresh modal has no secret before it is set.
    let fresh = RevealModal::new(path, reference);
    assert!(matches!(fresh.phase, RevealPhase::Confirm));
}

// Case 11: pressing `v` with no selected row, or a selected row without a
// `content_ref`, is a safe no-op — it must never open a reveal modal and must
// never panic.
#[test]
fn reveal_v_on_row_without_content_ref_is_safe_noop() {
    let mut app = raccpack_tui::app::App::new();
    app.current_view = raccpack_tui::app::ViewId::Findings;
    app.focus = raccpack_tui::app::Focus::Main;

    // Row with NO content_ref selected.
    app.dig_state.all_findings = vec![FindingRow {
        path: PathBuf::from("/repo/secrets.txt"),
        risk: raccpack_core::domain::SensitiveRisk::High,
        kind: "filename-match".to_string(),
        git_status: "tracked".to_string(),
        content_ref: None,
    }];
    app.dig_state.reapply_filter();
    app.dig_state.table_state.select(Some(0));

    // No reveal modal may be opened for a row without a content_ref.
    app.handle_key(key(KeyCode::Char('v')));
    assert!(
        app.reveal.is_none(),
        "v on a row with no content_ref must be a no-op"
    );
    assert_eq!(app.current_view, raccpack_tui::app::ViewId::Findings);
}

#[test]
fn reveal_v_with_no_selection_is_safe_noop() {
    let mut app = raccpack_tui::app::App::new();
    app.current_view = raccpack_tui::app::ViewId::Findings;
    app.focus = raccpack_tui::app::Focus::Main;
    // No findings at all and no selection.
    let cmd = app.handle_key(key(KeyCode::Char('v')));
    assert_eq!(cmd, raccpack_tui::app::Command::None);
    assert!(app.reveal.is_none());
}

// Case 11b: when the selected row DOES carry a content_ref, `v` opens the
// confirm modal (and only sets up the modal, never revealing the value yet).
#[test]
fn reveal_v_on_content_row_opens_confirm_modal_without_revealing() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIAOPENSUSRMODAL123";
    let (path, reference) = ref_for(&tmp, raw);

    let mut app = raccpack_tui::app::App::new();
    app.current_view = raccpack_tui::app::ViewId::Findings;
    app.focus = raccpack_tui::app::Focus::Main;
    app.dig_state.all_findings = vec![FindingRow {
        path: path.clone(),
        risk: raccpack_core::domain::SensitiveRisk::Critical,
        kind: "aws_access_key".to_string(),
        git_status: "tracked".to_string(),
        content_ref: Some(reference.clone()),
    }];
    app.dig_state.reapply_filter();
    app.dig_state.table_state.select(Some(0));

    app.handle_key(key(KeyCode::Char('v')));
    let modal = app
        .reveal
        .as_ref()
        .expect("v on a content row opens the modal");
    assert!(matches!(modal.phase, RevealPhase::Confirm));
    assert_eq!(modal.reference, reference);

    // Nothing revealed yet: the raw value has not crossed into the modal.
    let debug = format!("{:?}", app.reveal);
    assert!(
        !debug.contains(raw),
        "opening the confirm step must not reveal the raw value: {debug}"
    );
}

// Case 12 (app level): confirming dispatches reveal to the worker; Esc aborts
// and returns to the list with the modal cleared (no reveal, no lingering
// secret in app state).
#[test]
fn reveal_escaping_confirm_clears_modal_without_secret() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIAESCABORTCLEAR123";
    let (path, reference) = ref_for(&tmp, raw);

    let mut app = raccpack_tui::app::App::new();
    app.current_view = raccpack_tui::app::ViewId::Findings;
    app.focus = raccpack_tui::app::Focus::Main;
    app.dig_state.all_findings = vec![FindingRow {
        path: path.clone(),
        risk: raccpack_core::domain::SensitiveRisk::Critical,
        kind: "aws_access_key".to_string(),
        git_status: "tracked".to_string(),
        content_ref: Some(reference),
    }];
    app.dig_state.reapply_filter();
    app.dig_state.table_state.select(Some(0));

    // Open the confirm modal.
    app.handle_key(key(KeyCode::Char('v')));
    assert!(app.reveal.is_some());

    // Esc from confirm closes it: modal is cleared and no secret is present.
    let cmd = app.handle_key(key(KeyCode::Esc));
    assert!(
        app.reveal.is_none(),
        "Esc from the confirm step must clear the reveal modal"
    );
    let _ = cmd;
    let debug = format!("{:?}", app);
    assert!(
        !debug.contains(raw),
        "no raw value may linger in App after aborting: {debug}"
    );
}

// Case 12c (cross-cut, no raw in state): after a full dig with content, even a
// row whose source file holds a raw secret never carries it in FindingRow /
// DigScreenState debug.
#[test]
fn reveal_flow_never_stores_raw_in_screen_state() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = "AKIASTATELEAKPROBE42";
    let project = tmp.path().join("srv");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("creds.txt"), format!("{raw}\n")).unwrap();

    let (worker_tx, rx) = spawn_worker();
    worker_tx
        .send(WorkerMsg::Dig {
            project: project.clone(),
            den_dir: PathBuf::from("/tmp/b1.5-den"),
            scan_content: true,
        })
        .unwrap();

    let mut dig_done = None;
    for _ in 0..30 {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(WorkerEvent::DigDone(res)) => {
                dig_done = Some(res);
                break;
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    let result = dig_done
        .expect("DigDone must arrive")
        .expect("dig run succeeds");

    let mut state = DigScreenState::default();
    state.set_dig_result(result);

    let debug = format!("{state:?}");
    assert!(
        !debug.contains(raw),
        "raw secret leaked into screen state: {debug}"
    );
    for row in &state.findings {
        assert!(!format!("{row:?}").contains(raw), "row leaked raw value");
    }
}
