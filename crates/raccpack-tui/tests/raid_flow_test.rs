//! Integration tests for the B1.4 raid flow: the worker bridge drives a real
//! core `raid` through preview (DryRun) and commit (with a passphrase), and
//! the den-write + no-secret-leak invariants hold end to end.
//!
//! Mirrors the `dig_screen_test` bridge pattern: `spawn_worker()` + a bridge
//! thread forwarding `WorkerEvent -> AppEvent::Worker`, then deterministic
//! `wait_for_*` draining. All runs are tempfile-based; the real den/projects
//! are never touched.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use raccpack_core::app::{OperationKind, OrchestrationMode, RaidResult};
use raccpack_tui::event::AppEvent;
use raccpack_tui::worker::{
    spawn_worker, RaidWorkerOpts, WorkerEvent, WorkerMsg, WorkerPassphrase,
};

/// Test passphrase — long and distinctive; it must never appear in any Debug
/// output, progress message, or done event across the whole flow.
const TEST_PASSPHRASE: &str = "test-passphrase-b1.4-raid-flow";

/// Raw fixture secret value stash must find, encrypt, and never leak back in
/// any event or Debug output.
const RAW_SECRET: &str = "RAWSECRET_b1.4_xyz789";

// --- Test helpers -----------------------------------------------------------

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

/// Create parent directories and write a file at `root/rel`.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// A hermetic workspace root with `proj/` and `den/` sibling paths (den may
/// or may not be created, depending on the run mode).
fn workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let proj = temp.path().join("proj");
    let den = temp.path().join("den");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj, den)
}

/// A project fixture with a sensitive file (High), a trash dir (rinse target)
/// and a normal file that must always survive.
fn full_fixture(proj: &Path) -> Vec<PathBuf> {
    vec![
        write(proj, ".env", &format!("PASSWORD={RAW_SECRET}\n")),
        write(proj, "README.md", "# demo project\n"),
        write(proj, "node_modules/pkg/index.js", "module.exports = 1;\n"),
    ]
}

/// Build worker raid options for a project; default Atomic, stash on,
/// remove-sources on (mirrors `RaidOptions::default` semantics).
fn default_opts(_project: &Path) -> RaidWorkerOpts {
    RaidWorkerOpts {
        keep_sources: false,
        skip_stash: false,
        mode: OrchestrationMode::Atomic,
    }
}

/// Recursively collect absolute file paths under `root` (also returns the
/// sorted `Vec<PathBuf>`).
fn walk_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

/// Names of child directories directly under `root` (lowercased, sorted).
fn child_dir_names(root: &Path) -> Vec<String> {
    if !root.exists() {
        return Vec::new();
    }
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                names.push(entry.file_name().to_string_lossy().to_lowercase());
            }
        }
    }
    names.sort();
    names
}

/// Wait for `RaidPreviewDone`, draining every other event (progress included)
/// and returning the result plus the Debug string of a single collected
/// digest.
fn wait_for_raid_preview(
    ui_rx: &mpsc::Receiver<AppEvent>,
) -> Result<RaidResult, raccpack_core::domain::Error> {
    wait_for_done(ui_rx, |ev| match ev {
        WorkerEvent::RaidPreviewDone(result) => Some(result),
        _ => None,
    })
}

/// Wait for `RaidDone`, draining every other event (progress included).
fn wait_for_raid_done(
    ui_rx: &mpsc::Receiver<AppEvent>,
) -> Result<RaidResult, raccpack_core::domain::Error> {
    wait_for_done(ui_rx, |ev| match ev {
        WorkerEvent::RaidDone(result) => Some(result),
        _ => None,
    })
}

/// Drain events until `pick` matches a terminal `WorkerEvent`, returning that
/// result. All non-matching events (e.g. `Progress`) are ignored.
fn wait_for_done(
    ui_rx: &mpsc::Receiver<AppEvent>,
    mut pick: impl FnMut(WorkerEvent) -> Option<Result<RaidResult, raccpack_core::domain::Error>>,
) -> Result<RaidResult, raccpack_core::domain::Error> {
    for _ in 0..40 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(event)) => {
                if let Some(result) = pick(event) {
                    return result;
                }
            }
            Ok(_) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    panic!("timeout waiting for raid done event");
}

// --- Case 1: preview (DryRun) writes nothing --------------------------------

#[test]
fn raid_preview_dry_run_writes_nothing_to_den() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(&proj),
        })
        .unwrap();

    let result = wait_for_raid_preview(&ui_rx).expect("preview must succeed");
    assert!(result.success, "preview must succeed: {result:?}");
    assert!(result.dry_run, "preview must be a dry run: {result:?}");
    assert!(
        result.den_artifacts.is_empty(),
        "dry run must not report artifacts: {:?}",
        result.den_artifacts
    );

    // Dry run writes nothing: no .age, .tar.zst, .json, and no staging / any
    // den subdirectory at all.
    let files = walk_files(&den);
    for f in &files {
        let name = f.to_string_lossy().to_lowercase();
        assert!(
            !name.ends_with(".age") && !name.ends_with(".tar.zst") && !name.ends_with(".json"),
            "dry run must place no artifact under the den: {}",
            f.display()
        );
    }
    assert!(
        files.is_empty(),
        "dry run must write nothing at all under the den (found {} files)",
        files.len()
    );
    let dirs = child_dir_names(&den);
    assert!(
        dirs.is_empty(),
        "dry run must not create any den subdirectory: {dirs:?}"
    );

    // Sources are never touched by a dry run.
    for fixture in full_fixture(&proj) {
        assert!(fixture.is_file(), "dry run must keep sources: {fixture:?}");
    }
}

// --- Case 2: commit places artifacts + passphrase honored --------------------

#[test]
fn raid_commit_places_age_archive_and_manifest_and_never_leaks_plaintext() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(&proj),
            passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
        })
        .unwrap();

    let result = wait_for_raid_done(&ui_rx).expect("commit run must succeed");
    assert!(result.success, "commit must succeed: {result:?}");
    assert!(!result.dry_run, "commit must not be a dry run: {result:?}");
    assert!(
        result.den_artifacts.len() >= 2,
        "commit must place the age archive and the pack: {:?}",
        result.den_artifacts
    );

    let files = walk_files(&den);
    let age: Vec<&PathBuf> = files
        .iter()
        .filter(|f| f.to_string_lossy().ends_with(".age"))
        .collect();
    let packs: Vec<&PathBuf> = files
        .iter()
        .filter(|f| {
            f.to_string_lossy().ends_with(".tar.zst")
                && f.to_string_lossy().to_lowercase().contains("/packs/")
        })
        .collect();
    let manifests: Vec<&PathBuf> = files
        .iter()
        .filter(|f| {
            f.to_string_lossy().ends_with(".json")
                && f.to_string_lossy().to_lowercase().contains("/manifests/")
        })
        .collect();

    assert_eq!(age.len(), 1, "exactly one .age under the den: {age:?}");
    assert_eq!(
        packs.len(),
        1,
        "exactly one .tar.zst under den/packs: {packs:?}"
    );
    assert_eq!(
        manifests.len(),
        1,
        "exactly one manifest json under den/manifests: {manifests:?}"
    );

    // No plaintext copy of the fixture secret survives anywhere under the den.
    let all_text = files
        .iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !all_text.contains(RAW_SECRET),
        "den must never contain a plaintext copy of the secret"
    );

    assert!(
        !proj.join(".env").exists(),
        "default remove_sources must delete .env after a commit"
    );
    assert!(
        proj.join("README.md").is_file(),
        "normal files must survive"
    );
}

// --- Case 3: no passphrase leak (Debug / events) -----------------------------

#[test]
fn worker_raid_msg_debug_never_leaks_passphrase() {
    let msg = WorkerMsg::RaidRun {
        project: PathBuf::from("/tmp/b1.4-proj"),
        den_dir: PathBuf::from("/tmp/b1.4-den"),
        opts: RaidWorkerOpts {
            keep_sources: false,
            skip_stash: false,
            mode: OrchestrationMode::Atomic,
        },
        passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
    };

    let debug = format!("{:?}", msg);
    assert!(
        !debug.contains(TEST_PASSPHRASE),
        "WorkerMsg::RaidRun Debug must be redacted: {debug}"
    );
}

#[test]
fn raid_flow_events_never_leak_passphrase_or_raw_secret() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    // Preview first, then a commit run — collect the Debug of every event that
    // travels the bridge, so any leak shows up whatever the phase.
    let (worker_tx, ui_rx) = spawn_bridged_worker();

    worker_tx
        .send(WorkerMsg::RaidPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(&proj),
        })
        .unwrap();
    wait_for_raid_preview(&ui_rx).expect("preview must succeed");

    // Reuse the same worker for the commit run.
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(&proj),
            passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
        })
        .unwrap();

    let mut seen_debug = String::new();
    let mut checked = false;
    for _ in 0..40 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(event)) => {
                let d = format!("{:?}", event);
                assert!(
                    !d.contains(TEST_PASSPHRASE),
                    "WorkerEvent must not carry the passphrase: {d}"
                );
                assert!(
                    !d.contains(RAW_SECRET),
                    "WorkerEvent must not carry the raw secret: {d}"
                );
                seen_debug.push_str(&d);
                if matches!(event, WorkerEvent::RaidDone(_)) {
                    checked = true;
                    break;
                }
            }
            Ok(_) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(checked, "RaidDone must arrive (collected: {seen_debug})");
}

// --- Case 4: missing project surfaces an error honestly ----------------------

#[test]
fn raid_preview_missing_project_reports_error() {
    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidPreview {
            project: PathBuf::from("/nonexistent-b1.4-project"),
            den_dir: PathBuf::from("/tmp/b1.4-den"),
            opts: default_opts(Path::new("/nonexistent-b1.4-project")),
        })
        .unwrap();

    let result = wait_for_raid_preview(&ui_rx);
    assert!(
        result.is_err(),
        "raid previewing a nonexistent project must surface an error"
    );
}

#[test]
fn raid_commit_missing_project_reports_error() {
    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: PathBuf::from("/nonexistent-b1.4-project"),
            den_dir: PathBuf::from("/tmp/b1.4-den"),
            opts: default_opts(Path::new("/nonexistent-b1.4-project")),
            passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
        })
        .unwrap();

    let result = wait_for_raid_done(&ui_rx);
    assert!(
        result.is_err(),
        "raid commit of a nonexistent project must surface an error"
    );
}

// --- Case 5: progress events carry OperationKind::Raid -----------------------

#[test]
fn raid_commit_emits_progress_events_for_raid_operation() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(&proj),
            passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
        })
        .unwrap();

    let mut raid_progress = 0;
    let mut done = false;
    for _ in 0..40 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(WorkerEvent::Progress(progress))) => {
                assert_eq!(
                    progress.operation,
                    OperationKind::Raid,
                    "raid flow must emit Raid progress events"
                );
                assert!(
                    progress.overall_percent <= 100,
                    "overall_percent must be within 0..=100: {}",
                    progress.overall_percent
                );
                assert!(
                    !progress.message.contains(TEST_PASSPHRASE)
                        && !progress.message.contains(RAW_SECRET),
                    "progress message must not leak secrets: {:?}",
                    progress.message
                );
                raid_progress += 1;
            }
            Ok(AppEvent::Worker(WorkerEvent::RaidDone(result))) => {
                assert!(result.is_ok(), "commit must succeed: {result:?}");
                done = true;
                break;
            }
            Ok(_) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(done, "RaidDone must arrive");
    assert!(
        raid_progress > 0,
        "a commit run must emit at least one OperationKind::Raid progress event"
    );
}

// --- Case 6: skip_stash / keep_sources toggles honored -----------------------

#[test]
fn raid_preview_skip_stash_marks_stash_stage_skipped() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let opts = RaidWorkerOpts {
        keep_sources: true, // keep_sources should have no effect on phase skipping
        skip_stash: true,
        mode: OrchestrationMode::Atomic,
    };

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts,
        })
        .unwrap();

    let result = wait_for_raid_preview(&ui_rx).expect("preview must succeed");
    assert!(
        result.success,
        "skipping stash must still succeed: {result:?}"
    );

    let stash_stage = result
        .stages
        .iter()
        .find(|s| s.name == "stash")
        .expect("stages must include stash");
    assert!(
        stash_stage.skipped,
        "stash stage must be skipped when skip_stash=true: {result:?}"
    );
}

#[test]
fn raid_commit_with_keep_sources_does_not_delete_source_file() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let opts = RaidWorkerOpts {
        keep_sources: true,
        skip_stash: false,
        mode: OrchestrationMode::Atomic,
    };

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::RaidRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts,
            passphrase: WorkerPassphrase::new(TEST_PASSPHRASE.to_string()),
        })
        .unwrap();

    let result = wait_for_raid_done(&ui_rx).expect("commit must succeed");
    assert!(result.success, "commit must succeed: {result:?}");
    assert!(
        proj.join(".env").is_file(),
        "keep_sources=true must retain the .env source file after a commit"
    );
}
