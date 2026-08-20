//! Integration tests for A2.2 — facade `rinse` DryRun/Commit + bytes freed.
//!
//! Covers the 8 required cases from `docs/alpha/a2/a2.2-facade-rinse.md` §6:
//! 1. DryRun lists the matched dirs and leaves the filesystem unchanged;
//! 2. Commit removes the matched dirs and reports `bytes_freed > 0`;
//! 3. `strategies` from options override the config's `enabled_strategies`
//!    (and `None` falls back to the config list);
//! 4. an unknown strategy string fails with `Error::Config`;
//! 5. a path outside the target is never deleted (sibling survives);
//! 6. an empty project returns `Ok` with nothing removed and 0 bytes in both
//!    DryRun and Commit;
//! 7. a symlink trash dir (rust `target` -> external dir) is never removed and
//!    the external content stays intact, plus the direct `remove_trash_dir`
//!    symlink guard is a no-op;
//! 8. progress events follow the spec table (DryRun 0/40/100, Commit
//!    additionally 70 "Removing…", final `phase_complete`).
//!
//! Extras beyond the mandatory list:
//! - an empty `strategies` Vec returns `Ok` with nothing removed;
//! - `include_custom_patterns` is a reserved no-op (same result as `false`);
//! - Commit `bytes_freed` equals the sum of the detect-time `size_bytes`
//!   (exact because all fixtures are 1-byte files).
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no sleeps. The
//! symlink test is Linux/Unix-only and guarded with `#[cfg(unix)]`.

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use raccpack_core::{
    remove_trash_dir, rinse, AppContext, CleanupConfig, Error, NullProgress, OperationKind,
    ProgressEvent, ProgressSink, RaccConfig, RinseOptions, RinseResult, RunMode,
};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a 1-byte placeholder at `root/rel`.
fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, b"x").expect("write fixture file");
}

/// Create a hermetic workspace root containing an empty `proj/` directory.
fn project_dir() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create temp dir");
    let proj = temp.path().join("proj");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj)
}

/// Build an `AppContext` for a `rinse` run: scan root = the project itself,
/// and the den is always an explicit TempDir path so the real
/// `~/.raccpack/den` is never touched.
fn ctx_for(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default()
        .with_scan_root(project_root)
        .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Like [`ctx_for`], but with explicit `cleanup.enabled_strategies`.
fn ctx_with_strategies(
    project_root: &Path,
    den_dir: &Path,
    mode: RunMode,
    enabled: Vec<String>,
) -> AppContext {
    let config = RaccConfig {
        cleanup: CleanupConfig {
            enabled_strategies: enabled,
        },
        ..RaccConfig::default()
    }
    .with_scan_root(project_root)
    .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Default rinse options for a project; `None` strategies fall back to config.
fn rinse_options(target: &Path, strategies: Option<Vec<String>>) -> RinseOptions {
    RinseOptions {
        target: target.to_path_buf(),
        strategies,
        include_custom_patterns: false,
        collect_only: false,
    }
}

/// Run `rinse` with a null sink; panics with context on error.
fn rinse_once(ctx: &AppContext, opts: &RinseOptions) -> RinseResult {
    let mut progress = NullProgress;
    rinse(ctx, opts, &mut progress).expect("rinse should succeed")
}

/// Run `rinse` recording every progress event; returns the events.
fn rinse_recorded(ctx: &AppContext, opts: &RinseOptions) -> Vec<ProgressEvent> {
    let mut sink = RecordingSink::default();
    rinse(ctx, opts, &mut sink).expect("rinse should succeed");
    sink.events
}

/// Sink that collects emitted events for assertions.
#[derive(Default)]
struct RecordingSink {
    events: Vec<ProgressEvent>,
}

impl ProgressSink for RecordingSink {
    fn emit(&mut self, event: ProgressEvent) {
        self.events.push(event);
    }
}

// --- Case 1: DryRun lists dirs, filesystem unchanged --------------------------

#[test]
fn dry_run_lists_dirs_and_leaves_filesystem_unchanged() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = rinse_once(&ctx, &rinse_options(&proj, None));

    assert!(result.dry_run, "dry run must be reported");
    assert_eq!(result.removed.len(), 2, "both trash dirs must be listed");
    let names: Vec<&str> = result
        .removed
        .iter()
        .map(|d| d.pattern_name.as_str())
        .collect();
    assert_eq!(names, vec!["node_modules", "target"], "sorted by path");
    assert!(
        result.bytes_freed > 0,
        "dry run still reports estimated bytes"
    );

    assert!(
        proj.join("node_modules").exists(),
        "node_modules must survive a dry run"
    );
    assert!(
        proj.join("target").exists(),
        "target must survive a dry run"
    );
    assert!(
        proj.join("target/debug/x").is_file(),
        "fixture content must survive a dry run"
    );
}

// --- Case 2: Commit removes matched dirs, bytes_freed > 0 -----------------------

#[test]
fn commit_removes_matched_dirs_and_reports_bytes() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &rinse_options(&proj, None));

    assert!(!result.dry_run, "commit must report dry_run == false");
    assert_eq!(result.removed.len(), 2, "both trash dirs must be removed");
    assert!(result.bytes_freed > 0, "commit must report freed bytes");

    assert!(
        !proj.join("node_modules").exists(),
        "node_modules must be gone after commit"
    );
    assert!(
        !proj.join("target").exists(),
        "target must be gone after commit"
    );
}

// --- Case 3: Strategies filter from options overrides config ---------------------

#[test]
fn strategies_from_options_override_config_defaults() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    // Config defaults to rust/node/python; only rust must be honored.
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &rinse_options(&proj, Some(vec!["rust".into()])));

    assert_eq!(result.removed.len(), 1, "only the rust pattern may match");
    assert_eq!(result.removed[0].pattern_name, "target");
    assert!(
        proj.join("node_modules").exists(),
        "node_modules must be untouched with rust-only strategies"
    );
    assert!(
        !proj.join("target").exists(),
        "target must be removed by the rust strategy"
    );
}

#[test]
fn strategies_none_uses_config_enabled_strategies() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_with_strategies(&proj, &den, RunMode::Commit, vec!["node".into()]);

    let result = rinse_once(&ctx, &rinse_options(&proj, None));

    assert_eq!(
        result.removed.len(),
        1,
        "only the config-enabled node pattern may match"
    );
    assert_eq!(result.removed[0].pattern_name, "node_modules");
    assert!(
        !proj.join("node_modules").exists(),
        "node_modules must be removed by the node strategy"
    );
    assert!(
        proj.join("target").exists(),
        "target must survive a node-only config"
    );
}

// --- Case 4: Unknown strategy string → Error --------------------------------------

#[test]
fn unknown_strategy_is_config_error() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = rinse_options(&proj, Some(vec!["nope".into()]));
    let err = rinse(&ctx, &opts, &mut NullProgress).expect_err("unknown strategy must fail");
    assert!(
        matches!(err, Error::Config { .. }),
        "expected Error::Config, got {err:?}"
    );
    assert!(
        proj.join("target").exists(),
        "a failed run must not delete anything"
    );
}

// --- Case 5: Path outside target not deleted ---------------------------------------

#[test]
fn sibling_node_modules_outside_target_is_untouched() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    fs::create_dir_all(&proj).expect("create project dir");
    write(&proj, "target/debug/x");
    // A `node_modules` SIBLING directly under the TempDir, outside the target.
    write(temp.path(), "node_modules/a");
    let den = temp.path().join("den");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &rinse_options(&proj, None));

    let sibling = temp.path().join("node_modules");
    assert!(
        sibling.exists(),
        "the sibling outside the target must survive"
    );
    assert!(sibling.join("a").is_file(), "sibling content must survive");
    assert!(
        result.removed.iter().all(|d| d.path.starts_with(&proj)),
        "no removed entry may lie outside the target: {:?}",
        result.removed.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
    assert!(
        !proj.join("target").exists(),
        "the in-target target must still be removed"
    );
}

// --- Case 6: Empty project → Ok ------------------------------------------------------

#[test]
fn empty_project_is_ok_with_nothing_removed() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");

    let dry_ctx = ctx_for(&proj, &den, RunMode::DryRun);
    let dry = rinse_once(&dry_ctx, &rinse_options(&proj, None));
    assert!(dry.dry_run, "empty project in dry run must be Ok");
    assert!(dry.removed.is_empty(), "nothing to list: {:?}", dry.removed);
    assert_eq!(dry.bytes_freed, 0);

    let commit_ctx = ctx_for(&proj, &den, RunMode::Commit);
    let commit = rinse_once(&commit_ctx, &rinse_options(&proj, None));
    assert!(!commit.dry_run, "empty project in commit must be Ok");
    assert!(
        commit.removed.is_empty(),
        "nothing to remove: {:?}",
        commit.removed
    );
    assert_eq!(commit.bytes_freed, 0);
}

// --- Case 7: Symlink trash dir to an external dir is never removed -------------------

#[cfg(unix)]
#[test]
fn commit_symlink_trash_dir_leaves_external_target_intact() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let external = TempDir::new().unwrap();
    fs::write(external.path().join("marker.txt"), "precious").expect("write marker");
    // `target` matches the rust strategy; as a symlink it must never be
    // reported or deleted (find_trash_dirs never records symlink dirs).
    symlink(external.path(), proj.join("target")).expect("create target symlink");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &rinse_options(&proj, Some(vec!["rust".into()])));

    assert!(
        result.removed.is_empty(),
        "symlink trash dirs must be skipped: {:?}",
        result.removed
    );
    assert!(
        external.path().exists(),
        "the external dir must survive a commit"
    );
    assert!(
        external.path().join("marker.txt").is_file(),
        "external content must survive a commit"
    );
}

#[cfg(unix)]
#[test]
fn remove_trash_dir_on_symlink_is_noop() {
    let temp = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    fs::write(external.path().join("marker.txt"), "precious").expect("write marker");
    let link = temp.path().join("target");
    symlink(external.path(), &link).expect("create target symlink");

    let freed = remove_trash_dir(&link).expect("symlink guard must return Ok");
    assert_eq!(freed, 0, "nothing may be counted as freed");
    assert!(
        external.path().exists(),
        "the external dir must survive remove_trash_dir"
    );
    assert!(
        external.path().join("marker.txt").is_file(),
        "external content must survive remove_trash_dir"
    );
}

// --- Case 8: Progress complete --------------------------------------------------------

#[test]
fn progress_dry_run_emits_spec_steps() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let events = rinse_recorded(&ctx, &rinse_options(&proj, None));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(
        percents,
        vec![0, 40, 100],
        "dry run must emit scan/found/done"
    );

    assert_eq!(events[0].message, "Scanning for build artifacts…");
    assert!(
        events[1].message.contains("Found"),
        "expected 'Found N directories (X MiB)', got {:?}",
        events[1].message
    );
    assert!(
        events[1].message.contains("directories"),
        "got {:?}",
        events[1].message
    );
    assert!(
        events[1].message.contains("MiB"),
        "got {:?}",
        events[1].message
    );
    assert_eq!(events[2].message, "Done");

    for e in &events {
        assert_eq!(e.operation, OperationKind::Rinse, "operation must be Rinse");
        assert_eq!(e.phase, "rinse", "phase must be \"rinse\"");
        assert_eq!(e.phase_index, 0);
        assert_eq!(e.phase_count, 1);
        assert_eq!(
            e.percent, e.overall_percent,
            "single-phase rinse must equate percent and overall_percent"
        );
    }
    let completes: Vec<bool> = events.iter().map(|e| e.phase_complete).collect();
    assert_eq!(
        completes,
        vec![false, false, true],
        "only the final event marks the phase complete"
    );
}

#[test]
fn progress_commit_emits_removal_step() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let events = rinse_recorded(&ctx, &rinse_options(&proj, None));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(
        percents,
        vec![0, 40, 70, 100],
        "commit must add the removing step"
    );

    assert_eq!(events[2].message, "Removing…");
    assert_eq!(events[3].message, "Done");
    for e in &events {
        assert_eq!(e.operation, OperationKind::Rinse, "operation must be Rinse");
        assert_eq!(e.phase, "rinse", "phase must be \"rinse\"");
    }
    assert!(
        events.last().unwrap().phase_complete,
        "the final event must mark the phase complete"
    );
}

// --- Extras ----------------------------------------------------------------------------

#[test]
fn empty_strategies_dry_run_is_ok_and_removes_nothing() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = rinse_once(&ctx, &rinse_options(&proj, Some(vec![])));

    assert!(result.dry_run);
    assert!(result.removed.is_empty(), "no strategies, no results");
    assert_eq!(result.bytes_freed, 0);
    assert!(
        proj.join("target").exists(),
        "nothing may be removed with no strategies"
    );
}

#[test]
fn include_custom_patterns_is_a_reserved_noop() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let mut with_flag = rinse_options(&proj, Some(vec!["rust".into()]));
    with_flag.include_custom_patterns = true;
    let flagged = rinse_once(&ctx, &with_flag);
    let unflagged = rinse_once(&ctx, &rinse_options(&proj, Some(vec!["rust".into()])));

    assert_eq!(flagged.removed, unflagged.removed, "flag must be a no-op");
    assert_eq!(
        flagged.bytes_freed, unflagged.bytes_freed,
        "flag must be a no-op"
    );
    assert_eq!(flagged.removed.len(), 1);
}

#[test]
fn commit_bytes_freed_matches_removed_size_sum() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "node_modules/a");
    write(&proj, "target/debug/x");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &rinse_options(&proj, None));

    let detect_sum: u64 = result.removed.iter().map(|d| d.size_bytes).sum();
    assert_eq!(
        result.bytes_freed, detect_sum,
        "1-byte fixtures keep detect-time sizes stable"
    );
    assert!(result.bytes_freed > 0);
}
