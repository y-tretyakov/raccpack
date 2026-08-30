//! Unit tests for the worker thread and its raid sinks.

use std::time::Duration;

use raccpack_core::app::{OperationKind, OrchestrationMode, ProgressEvent};

use super::RaidProgressSink;
use super::*;

#[test]
fn worker_can_spawn_and_receive_cancel() {
    let (worker_tx, event_rx) = spawn_worker();

    worker_tx.send(WorkerMsg::Cancel).unwrap();

    // Should receive Cancelled event
    let event = event_rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert!(matches!(event, WorkerEvent::Cancelled));
}

#[test]
fn worker_can_spawn_and_receive_sniff_done() {
    let (worker_tx, event_rx) = spawn_worker();
    let scan_root = std::path::PathBuf::from("/tmp/nonexistent");
    let den_dir = std::path::PathBuf::from("/tmp/den");

    // This will fail because scan_root doesn't exist, but we should get a SniffDone event with error
    worker_tx
        .send(WorkerMsg::Sniff {
            scan_root,
            den_dir,
            force_refresh: true,
            detect_mode: None,
            max_depth: None,
        })
        .unwrap();

    // Should receive SniffDone event (with error)
    let event = event_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(event, WorkerEvent::SniffDone(_)));
}

#[test]
fn worker_can_dig_a_project_and_report_findings() {
    let tmp = std::env::temp_dir().join(format!("raccpack-tui-dig-test-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join(".env"), "DATABASE_URL=postgres://localhost/app").unwrap();

    let (worker_tx, event_rx) = spawn_worker();
    let den_dir = std::path::PathBuf::from("/tmp/den");
    worker_tx
        .send(WorkerMsg::Dig {
            project: tmp.clone(),
            den_dir,
            scan_content: true,
        })
        .unwrap();

    // Digest progress events until DigDone arrives.
    let mut done = None;
    for _ in 0..20 {
        let event = event_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        if let WorkerEvent::DigDone(result) = event {
            done = Some(result);
            break;
        }
    }
    std::fs::remove_dir_all(&tmp).unwrap();

    let result = done.expect("DigDone must arrive").expect("dig run ok");
    assert!(
        result.files.iter().any(|f| f.path.ends_with(".env")),
        "the .env fixture must be reported"
    );
}

#[test]
fn worker_preview_writes_nothing_to_den() {
    let temp = tempfile::TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    let den = temp.path().join("den");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join(".env"), "TOKEN=hunter2").unwrap();

    let (worker_tx, event_rx) = spawn_worker();
    worker_tx
        .send(WorkerMsg::RaidPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: RaidWorkerOpts {
                keep_sources: false,
                skip_stash: false,
                mode: OrchestrationMode::Atomic,
            },
        })
        .unwrap();

    let mut done = None;
    for _ in 0..20 {
        let event = event_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        if let WorkerEvent::RaidPreviewDone(result) = event {
            done = Some(result);
            break;
        }
    }
    let result = done
        .expect("RaidPreviewDone must arrive")
        .expect("preview ok");
    assert!(result.dry_run);
    assert!(result.success);
    assert!(result.den_artifacts.is_empty());
    assert!(!den.exists(), "preview must not create the den directory");
}

#[test]
fn worker_commit_missing_project_reports_error() {
    let (worker_tx, event_rx) = spawn_worker();
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: std::path::PathBuf::from("/nonexistent-b1.4-project"),
            den_dir: std::path::PathBuf::from("/tmp/b1.4-den"),
            opts: RaidWorkerOpts {
                keep_sources: false,
                skip_stash: false,
                mode: OrchestrationMode::Atomic,
            },
            passphrase: WorkerPassphrase::new("test-passphrase".to_string()),
        })
        .unwrap();

    let event = event_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    match event {
        WorkerEvent::RaidDone(result) => assert!(result.is_err()),
        other => panic!("expected RaidDone error, got {other:?}"),
    }
}

#[test]
fn worker_raid_msg_debug_redacts_passphrase() {
    let msg = WorkerMsg::RaidRun {
        project: std::path::PathBuf::from("/tmp/proj"),
        den_dir: std::path::PathBuf::from("/tmp/den"),
        opts: RaidWorkerOpts {
            keep_sources: false,
            skip_stash: false,
            mode: OrchestrationMode::Atomic,
        },
        passphrase: WorkerPassphrase::new("supersecretpassphrase".to_string()),
    };
    let debug = format!("{msg:?}");
    assert!(
        !debug.contains("supersecretpassphrase"),
        "WorkerMsg Debug must be redacted: {debug}"
    );
    assert!(debug.contains("redacted"));
}

#[test]
fn raid_progress_sink_filters_non_raid_operations() {
    let (tx, rx) = mpsc::channel();
    let mut sink = RaidProgressSink::new(TuiProgressSink::new(tx));

    let event = |operation: OperationKind, phase: &str| ProgressEvent {
        operation,
        phase: phase.to_string(),
        phase_index: 0,
        phase_count: 1,
        percent: 100,
        overall_percent: 50,
        message: "x".to_string(),
        phase_complete: true,
    };
    sink.emit(event(OperationKind::Stash, "stash"));
    sink.emit(event(OperationKind::Raid, "stash"));

    match rx.recv_timeout(Duration::from_millis(500)) {
        Ok(WorkerEvent::Progress(ev)) => {
            assert_eq!(ev.operation, OperationKind::Raid);
            assert_eq!(ev.phase, "stash");
        }
        other => panic!("expected the Raid progress event, got {other:?}"),
    }
    assert!(
        rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "the Stash operation-kind must be filtered out"
    );
}
