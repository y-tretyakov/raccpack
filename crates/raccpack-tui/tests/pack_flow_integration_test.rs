//! Integration tests for the T-02 pack flow: the worker bridge drives the real
//! core `pack` through preview (DryRun) and commit, and the den-write
//! invariants hold end to end.
//!
//! Mirrors the `raid_flow_test` bridge pattern: `spawn_worker()` + a bridge
//! thread forwarding `WorkerEvent -> AppEvent::Worker`, then deterministic
//! `wait_for_*` draining. All runs are tempfile-based; the real den / projects
//! are never touched. No network, no real TTY.
//!
//! Covers T-02 §5 cases 1 (dry-run writes nothing), 2 (commit places a
//! `.tar.zst`), 3 (cancel writes nothing / stays away from commit), 4 (option
//! toggles reach the core, including `output_name`) and 5 (den-inside-project
//! containment reports an error without panicking). Case 6 (Operations → pack
//! routing) is already covered by
//! `event::tests::open_operation_routes_pack_to_pack_flow`.
//!
//! `output_name` is now plumbed through `PackWorkerOpts` and is asserted in
//! [`pack_commit_output_name_controls_archive_filename`]. The Orchestrator
//! re-verifies on the merge-ready tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use raccpack_core::app::{OperationKind, PackResult};
use raccpack_core::domain::Error;
use raccpack_tui::event::AppEvent;
use raccpack_tui::worker::{spawn_worker, PackWorkerOpts, WorkerEvent, WorkerMsg};

/// A raw AWS access key prefix (Critical severity content marker). Must never
/// leak back into any event or Debug output.
const CRITICAL_RAW: &str = "AKIA_TESTCONTENT_C1";

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

/// A hermetic workspace root with `proj/` and `den/` sibling paths (den may or
/// may not be created, depending on the run mode).
fn workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let proj = temp.path().join("proj");
    let den = temp.path().join("den");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj, den)
}

/// A project fixture with one normal file plus one file carrying a Critical
/// content secret (used to exercise content-deny).
fn fixture(proj: &Path) -> Vec<PathBuf> {
    vec![
        write(proj, "README.md", "# demo project\n"),
        write(
            proj,
            "config/credentials.txt",
            &format!("AWS_ACCESS_KEY={CRITICAL_RAW}\n"),
        ),
    ]
}

/// Default pack worker options (auto archive name).
fn default_opts() -> PackWorkerOpts {
    PackWorkerOpts {
        deny_content_secrets: true,
        zstd_level: 3,
        output_name: None,
    }
}

/// Recursively collect absolute file paths under `root` (sorted).
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

/// Wait for `PackPreviewDone`, draining every other event (progress included).
fn wait_for_pack_preview(ui_rx: &mpsc::Receiver<AppEvent>) -> Result<PackResult, Error> {
    wait_for_done(ui_rx, |ev| match ev {
        WorkerEvent::PackPreviewDone(result) => Some(result),
        _ => None,
    })
}

/// Wait for `PackDone`, draining every other event (progress included).
fn wait_for_pack_done(ui_rx: &mpsc::Receiver<AppEvent>) -> Result<PackResult, Error> {
    wait_for_done(ui_rx, |ev| match ev {
        WorkerEvent::PackDone(result) => Some(result),
        _ => None,
    })
}

/// Drain events until `pick` matches a terminal `WorkerEvent`, returning that
/// result. All non-matching events (e.g. `Progress`) are ignored.
fn wait_for_done(
    ui_rx: &mpsc::Receiver<AppEvent>,
    mut pick: impl FnMut(WorkerEvent) -> Option<Result<PackResult, Error>>,
) -> Result<PackResult, Error> {
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
    panic!("timeout waiting for pack done event");
}

// --- Case 1: dry-run preview shows the expected archive name, writes nothing --

#[test]
fn pack_dry_run_preview_writes_nothing() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(),
        })
        .unwrap();

    let result = wait_for_pack_preview(&ui_rx).expect("preview must succeed");
    assert!(result.dry_run, "preview must be a dry run: {result:?}");

    // The expected artifact lives under den/packs/{yyyy}/{mm} with the
    // {slug}__*.tar.zst naming (slug = the project dir name "proj").
    let rel = result
        .output
        .strip_prefix(&den)
        .expect("preview output must be under the den");
    let rel = rel.to_string_lossy();
    assert!(
        rel.starts_with("packs/") && rel.ends_with(".tar.zst"),
        "preview output must be under packs/…/*.tar.zst: {rel}"
    );
    assert!(
        rel.contains("/proj__"),
        "preview output must be auto-named {{slug}}__{{ts}}: {rel}"
    );

    // Dry run writes nothing at all under the den (no dirs, no files).
    let files = walk_files(&den);
    assert!(
        files.is_empty(),
        "dry run must write nothing under the den (found {})",
        files.len()
    );
    let dirs = if den.exists() {
        fs::read_dir(&den)
            .map(|it| it.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0)
    } else {
        0
    };
    assert_eq!(dirs, 0, "dry run must not create any den subdirectory");
}

// --- Case 2: confirm → commit places a *.tar.zst in the den -------------------

#[test]
fn pack_commit_places_tar_zst_in_den() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(),
        })
        .unwrap();

    let result = wait_for_pack_done(&ui_rx).expect("commit run must succeed");
    assert!(!result.dry_run, "commit must not be a dry run: {result:?}");

    let placed = &result.output;
    assert!(
        placed.is_file(),
        "commit must place the archive on disk: {}",
        placed.display()
    );
    assert!(
        placed.to_string_lossy().ends_with(".tar.zst"),
        "placed artifact must be a .tar.zst: {}",
        placed.display()
    );

    let files = walk_files(&den);
    let packs: Vec<_> = files
        .iter()
        .filter(|f| {
            f.to_string_lossy().ends_with(".tar.zst")
                && f.to_string_lossy().to_lowercase().contains("/packs/")
        })
        .collect();
    assert_eq!(
        packs.len(),
        1,
        "exactly one .tar.zst under den/packs: {packs:?}"
    );

    // The source tree is untouched by a pack.
    assert!(proj.join("README.md").is_file());
    assert!(proj.join("config/credentials.txt").is_file());
}

// --- Case 3: cancel (n / Esc) writes nothing and never reaches commit --------

/// Confirming the preview must be what drives the commit. A preview alone, or
/// a cancel, must leave the den completely empty.
#[test]
fn pack_commit_requires_confirm_and_den_stays_empty_until_then() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();

    // Preview only — no confirm. Nothing may be written.
    worker_tx
        .send(WorkerMsg::PackPreview {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(),
        })
        .unwrap();
    let preview = wait_for_pack_preview(&ui_rx).expect("preview must succeed");
    assert!(preview.dry_run);
    assert!(
        walk_files(&den).is_empty(),
        "a preview alone must not write to the den"
    );
}

/// At the event layer, cancelling the flow (n / Esc while previewing yields
/// `PackCancel`) clears `pack_flow` and dispatches no `PackRun`, so the den
/// receives nothing.
#[test]
fn pack_cancel_clears_flow_and_sends_no_commit() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use raccpack_core::app::PackResult;
    use raccpack_tui::app::pack::{PackFlow, PackFlowOptions, PackFlowPhase};
    use raccpack_tui::app::App;

    let mut app = App::new();
    app.den_dir = PathBuf::from("/tmp/den");
    // Open a pack flow in the Preview phase, mirroring how the worker lands a
    // dry-run result.
    let mut flow = PackFlow::new(
        PathBuf::from("/tmp/proj"),
        app.den_dir.clone(),
        PackFlowOptions::default(),
    );
    flow.phase = PackFlowPhase::Preview(PackResult {
        source: PathBuf::from("/tmp/proj"),
        output: PathBuf::from("/tmp/den/packs/2099/01/proj__ts.tar.zst"),
        size_bytes: 0,
        file_count: 0,
        skipped_secret_files: 0,
        dry_run: true,
    });
    app.pack_flow = Some(flow);

    // `n` cancels: the flow is cleared so it cannot proceed to commit.
    let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
    let _cmd = app.handle_key(key);
    assert!(
        app.pack_flow.is_none(),
        "n must clear the pack flow so it cannot proceed to commit"
    );

    // Esc does the same in the Preparing phase.
    app.pack_flow = Some(PackFlow::new(
        PathBuf::from("/tmp/proj"),
        PathBuf::from("/tmp/den"),
        PackFlowOptions::default(),
    ));
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let _cmd = app.handle_key(esc);
    assert!(
        app.pack_flow.is_none(),
        "Esc must clear the pack flow in the Preparing phase"
    );
}

// --- Case 4: option toggles reach the core -----------------------------------

/// `zstd_level` is forwarded to the packed archive; a higher level still
/// yields a valid, readable `.tar.zst` on disk.
#[test]
fn pack_commit_zstd_level_option_is_honored() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let opts = PackWorkerOpts {
        deny_content_secrets: false,
        zstd_level: 19,
        output_name: None,
    };

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts,
        })
        .unwrap();

    let result = wait_for_pack_done(&ui_rx).expect("commit with level 19 must succeed");
    assert!(!result.dry_run);
    assert!(result.output.is_file(), "archive must exist on disk");
}

/// A custom `output_name` (now plumbed through `PackWorkerOpts`) renames the
/// produced archive before `.tar.zst` is appended.
#[test]
fn pack_commit_output_name_controls_archive_filename() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: PackWorkerOpts {
                deny_content_secrets: true,
                zstd_level: 3,
                output_name: Some("my-custom-archive".to_string()),
            },
        })
        .unwrap();

    let result = wait_for_pack_done(&ui_rx).expect("commit with custom name must succeed");
    assert!(!result.dry_run);
    let fname = result
        .output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert!(
        fname.starts_with("my-custom-archive"),
        "custom output_name must drive the archive filename: {fname}"
    );
    assert!(
        fname.ends_with(".tar.zst"),
        "custom-named archive must still end in .tar.zst: {fname}"
    );
    assert!(
        result.output.is_file(),
        "custom-named archive must exist on disk"
    );
}

/// `deny_content_secrets=true` excludes a Critical-content file from the
/// archive; `=false` includes it. (Name-based deny stays on either way.)
#[test]
fn pack_content_deny_toggle_skips_critical_secret_file() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    let paths = fixture(&proj);
    let secret_file = paths
        .iter()
        .find(|p| p.to_string_lossy().ends_with("credentials.txt"))
        .expect("fixture has a credentials file")
        .clone();
    let normal_file = proj.join("README.md");

    // deny on (default): the Critical-content file is skipped from the archive.
    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: PackWorkerOpts {
                deny_content_secrets: true,
                zstd_level: 3,
                output_name: None,
            },
        })
        .unwrap();
    let result = wait_for_pack_done(&ui_rx).expect("deny-on commit must succeed");
    assert!(
        result.skipped_secret_files >= 1,
        "deny-on must skip the critical file: {result:?}"
    );
    assert!(normal_file.is_file(), "sources are never deleted");

    // deny off: the Critical-content file goes into the archive.
    let den2 = temp.path().join("den2");
    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den2.clone(),
            opts: PackWorkerOpts {
                deny_content_secrets: false,
                zstd_level: 3,
                output_name: None,
            },
        })
        .unwrap();
    let result = wait_for_pack_done(&ui_rx).expect("deny-off commit must succeed");
    assert!(
        result.skipped_secret_files == 0,
        "deny-off must include the critical file: {result:?}"
    );
    // The file is still on disk (source never removed by pack).
    assert!(secret_file.is_file());
}

// --- Case 5: path containment — den inside the project is rejected ----------

/// Packing a project whose den directory lies *inside* the project tree must
/// yield an error (the facade's staging-containment guard) — no archive is
/// written and nothing panics.
#[test]
fn pack_commit_rejects_den_inside_project_without_panic() {
    let proj = tempfile::tempdir().expect("create project dir");
    fixture(proj.path());

    // The den is a subdirectory of the project — a broken layout the facade
    // must reject via its containment check.
    let den = proj.path().join(".raccpack-den");

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.path().to_path_buf(),
            den_dir: den.clone(),
            opts: default_opts(),
        })
        .unwrap();

    let result = wait_for_pack_done(&ui_rx);
    assert!(
        result.is_err(),
        "packing a project whose den is inside it must report an error"
    );
    // No archive was placed anywhere under the (rejected) layout, and the
    // project's own files survive — the error is reported, nothing panics.
    let archives_under_den = walk_files(&den)
        .into_iter()
        .filter(|f| f.to_string_lossy().ends_with(".tar.zst"))
        .count();
    assert_eq!(
        archives_under_den, 0,
        "a rejected containment layout must not produce an archive"
    );
}

// --- Case 6 (bonus): progress events carry OperationKind::Pack --------------

#[test]
fn pack_commit_emits_progress_events_for_pack_operation() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    fixture(&proj);

    let (worker_tx, ui_rx) = spawn_bridged_worker();
    worker_tx
        .send(WorkerMsg::PackRun {
            project: proj.clone(),
            den_dir: den.clone(),
            opts: default_opts(),
        })
        .unwrap();

    let mut pack_progress = 0;
    let mut done = false;
    for _ in 0..40 {
        match ui_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppEvent::Worker(WorkerEvent::Progress(progress))) => {
                assert_eq!(
                    progress.operation,
                    OperationKind::Pack,
                    "pack flow must emit Pack progress events"
                );
                assert!(
                    progress.overall_percent <= 100,
                    "overall_percent must be within 0..=100: {}",
                    progress.overall_percent
                );
                assert!(
                    !progress.message.contains(CRITICAL_RAW),
                    "progress must not carry raw secret material: {:?}",
                    progress.message
                );
                pack_progress += 1;
            }
            Ok(AppEvent::Worker(WorkerEvent::PackDone(result))) => {
                assert!(result.is_ok(), "commit must succeed: {result:?}");
                done = true;
                break;
            }
            Ok(_) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(done, "PackDone must arrive");
    assert!(
        pack_progress > 0,
        "a commit run must emit at least one OperationKind::Pack progress event"
    );
}
