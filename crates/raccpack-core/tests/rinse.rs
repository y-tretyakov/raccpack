//! Integration tests for `rinse`: DryRun/Commit + bytes freed (A2.2) and
//! DAG-scoped rinse (D3.1).
//!
//! A2.2 — covers the 8 required cases from
//! `docs/alpha/a2/a2.2-facade-rinse.md` §6:
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
//! D3.1 — DAG-scoped rinse (monorepo fixture tests):
//! - D1–D6: basic DAG dry-run/commit, scoped isolation, priority_table
//!   unchanged, empty scopes, collect_only.
//! - M1–M6: monorepo fixture (`backend/` + `web/`), three-level nesting,
//!   bidirectional scoped isolation, mismatched strategies.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no sleeps. The
//! symlink test is Linux/Unix-only and guarded with `#[cfg(unix)]`.

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use raccpack_core::{
    remove_trash_dir, rinse, AppContext, CleanupConfig, DetectMode, Detection, Error, NullProgress,
    OperationKind, ProgressEvent, ProgressSink, RaccConfig, RinseOptions, RinseResult, RunMode,
    StackNode,
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
        stack_tree: None,
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

// --- DAG helpers -----------------------------------------------------------------

/// Build a `Detection` for a given ecosystem and scope path.
fn detection(ecosystem: &str, scope: &Path) -> Detection {
    Detection {
        ecosystem: ecosystem.to_string(),
        language: None,
        frameworks: Vec::new(),
        confidence: 0.9,
        scope: scope.to_path_buf(),
        markers: Vec::new(),
    }
}

/// Build a leaf `StackNode`.
fn leaf_node(ecosystem: &str, scope: &Path) -> StackNode {
    StackNode {
        detection: detection(ecosystem, scope),
        children: Vec::new(),
    }
}

/// RinseOptions with `stack_tree` set to `Some(tree)` and config in `CompositeDag` mode.
fn dag_rinse_options(target: &Path, tree: StackNode) -> RinseOptions {
    RinseOptions {
        target: target.to_path_buf(),
        strategies: None,
        include_custom_patterns: false,
        collect_only: false,
        stack_tree: Some(tree),
    }
}

/// AppContext with `detect.mode = CompositeDag`.
fn ctx_dag(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig {
        detect: raccpack_core::config::DetectConfig {
            mode: DetectMode::CompositeDag,
        },
        ..RaccConfig::default()
    }
    .with_scan_root(project_root)
    .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

// --- Case D1: DAG dry-run — scoped discovery only ------------------------------

#[test]
fn dag_dry_run_scoped_discovery_only() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    let ctx = ctx_dag(&proj, &den, RunMode::DryRun);
    let tree = StackNode {
        detection: detection("rust", &proj),
        children: vec![leaf_node("node", &proj.join("web"))],
    };

    let result = rinse_once(&ctx, &dag_rinse_options(&proj, tree));

    assert!(result.dry_run, "must be dry run");
    assert!(proj.join("target").exists(), "target must survive dry run");
    assert!(
        proj.join("web/node_modules").exists(),
        "node_modules must survive dry run"
    );
}

// --- Case D2: DAG commit — each scope removes its own trash --------------------

#[test]
fn dag_commit_each_scope_removes_its_own_trash() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    let ctx = ctx_dag(&proj, &den, RunMode::Commit);
    let tree = StackNode {
        detection: detection("rust", &proj),
        children: vec![leaf_node("node", &proj.join("web"))],
    };

    let result = rinse_once(&ctx, &dag_rinse_options(&proj, tree));

    assert!(!result.dry_run, "must be commit");
    // Root scope (rust) removes target
    assert!(
        !proj.join("target").exists(),
        "rust scope target must be removed"
    );
    // Child scope (node) removes node_modules under its territory
    assert!(
        !proj.join("web/node_modules").exists(),
        "node scope node_modules must be removed by the child node scope"
    );
    assert_eq!(
        result.removed.len(),
        2,
        "both scoped trash dirs must be removed"
    );
}

// --- Case D3: DAG — scoped rust path does NOT touch node paths -----------------

#[test]
fn dag_scoped_rust_does_not_touch_node_paths() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    write(&proj, "web/target/debug/y");
    let ctx = ctx_dag(&proj, &den, RunMode::Commit);
    let tree = StackNode {
        detection: detection("node", &proj),
        children: vec![leaf_node("rust", &proj.join("web"))],
    };

    rinse_once(&ctx, &dag_rinse_options(&proj, tree));

    assert!(
        proj.join("target").exists(),
        "root target survives (only node enabled for root)"
    );
    assert!(
        !proj.join("web/target").exists(),
        "child rust scope target must be removed"
    );
    assert!(
        proj.join("web/node_modules").exists(),
        "child node scope node_modules survives (only rust strategy in child)"
    );
}

// --- Case D4: priority_table mode unchanged — flat walk ignores stack_tree --------

#[test]
fn priority_table_mode_ignores_stack_tree_flat_walk() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    // Config in default PriorityTable mode — DAG options are ignored.
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let tree = StackNode {
        detection: detection("node", &proj),
        children: vec![leaf_node("rust", &proj.join("web"))],
    };

    rinse_once(&ctx, &dag_rinse_options(&proj, tree));

    // PriorityTable mode uses default strategies (rust, node, python)
    // and walks the entire target tree.
    assert!(
        !proj.join("target").exists(),
        "flat walk must remove target (rust strategy enabled by default)"
    );
    assert!(
        !proj.join("web/node_modules").exists(),
        "flat walk must remove node_modules (node strategy enabled by default)"
    );
}

// --- Case D5: empty scopes list → no trash found ------------------------------

#[test]
fn dag_empty_scopes_finds_nothing() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    let ctx = ctx_dag(&proj, &den, RunMode::Commit);
    let tree = StackNode {
        detection: detection("", &proj),
        children: vec![],
    };

    // strategies: Some(vec![]) overrides config → empty strategy list → no patterns
    let opts = RinseOptions {
        target: proj.clone(),
        strategies: Some(vec![]),
        include_custom_patterns: false,
        collect_only: false,
        stack_tree: Some(tree),
    };

    let result = rinse_once(&ctx, &opts);

    assert!(
        result.removed.is_empty(),
        "empty strategies must find nothing: {:?}",
        result.removed
    );
}

// --- Case D6: collect_only with DAG tree --------------------------------------

#[test]
fn dag_collect_only_does_not_delete() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "target/debug/x");
    write(&proj, "web/node_modules/a");
    let ctx = ctx_dag(&proj, &den, RunMode::Commit);
    let tree = StackNode {
        detection: detection("rust", &proj),
        children: vec![leaf_node("node", &proj.join("web"))],
    };

    let opts = RinseOptions {
        target: proj.clone(),
        strategies: None,
        include_custom_patterns: false,
        collect_only: true,
        stack_tree: Some(tree),
    };

    let result = rinse_once(&ctx, &opts);

    assert!(
        proj.join("target").exists(),
        "collect_only must not delete target"
    );
    assert!(
        proj.join("web/node_modules").exists(),
        "collect_only must not delete node_modules"
    );
    assert!(
        !result.removed.is_empty(),
        "collect_only still reports found dirs"
    );
}

// --- Monorepo fixture (D3.1 spec §4) -----------------------------------------

/// Build a monorepo fixture matching the D3.1 spec:
///
/// ```text
/// monorepo/
/// ├── backend/
/// │   ├── Cargo.toml
/// │   └── target/debug/x
/// └── web/
///     ├── package.json
///     └── node_modules/dep/index.js
/// ```
fn monorepo_fixture() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("monorepo");
    // Rust subtree
    write(&root, "backend/Cargo.toml");
    write(&root, "backend/target/debug/x");
    // Node subtree
    write(&root, "web/package.json");
    write(&root, "web/node_modules/dep/index.js");
    (temp, root)
}

/// Build a [`StackNode`] tree matching the monorepo fixture.
fn monorepo_stack_tree(root: &Path) -> StackNode {
    StackNode {
        detection: detection("rust", root),
        children: vec![leaf_node("node", &root.join("web"))],
    }
}

// --- Case M1: DAG dry-run monorepo — scoped discovery only --------------------

#[test]
fn dag_monorepo_dry_run_finds_scoped_trash() {
    let (temp, root) = monorepo_fixture();
    let den = temp.path().join("den");
    let tree = monorepo_stack_tree(&root);
    let ctx = ctx_dag(&root, &den, RunMode::DryRun);

    let result = rinse_once(&ctx, &dag_rinse_options(&root, tree));

    assert!(result.dry_run);
    let patterns: Vec<&str> = result
        .removed
        .iter()
        .map(|d| d.pattern_name.as_str())
        .collect();
    assert!(
        patterns.contains(&"target"),
        "rust scope must find backend/target"
    );
    assert!(
        patterns.contains(&"node_modules"),
        "node scope must find web/node_modules"
    );
    // Nothing deleted
    assert!(root.join("backend/target").exists());
    assert!(root.join("web/node_modules").exists());
    // Source files untouched
    assert!(root.join("backend/Cargo.toml").is_file());
    assert!(root.join("web/package.json").is_file());
}

// --- Case M2: DAG commit monorepo — removes scoped trash, preserves sources ---

#[test]
fn dag_monorepo_commit_removes_scoped_trash_preserves_sources() {
    let (temp, root) = monorepo_fixture();
    let den = temp.path().join("den");
    let tree = monorepo_stack_tree(&root);
    let ctx = ctx_dag(&root, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &dag_rinse_options(&root, tree));

    assert!(!result.dry_run);
    assert_eq!(
        result.removed.len(),
        2,
        "two scoped trash dirs must be removed"
    );
    assert!(
        !root.join("backend/target").exists(),
        "backend/target must be removed"
    );
    assert!(
        !root.join("web/node_modules").exists(),
        "web/node_modules must be removed"
    );
    // Source files survive
    assert!(
        root.join("backend/Cargo.toml").is_file(),
        "Cargo.toml must survive"
    );
    assert!(
        root.join("web/package.json").is_file(),
        "package.json must survive"
    );
}

// --- Case M3: Scoped isolation — rust strategy does NOT remove node trash ------

#[test]
fn dag_scoped_isolation_rust_does_not_remove_node_trash() {
    let (temp, root) = monorepo_fixture();
    let den = temp.path().join("den");
    // Add a target inside web/ — rust strategy should NOT touch it
    write(&root, "web/target/debug/y");
    let tree = monorepo_stack_tree(&root);
    let ctx = ctx_dag(&root, &den, RunMode::DryRun);

    let result = rinse_once(&ctx, &dag_rinse_options(&root, tree));

    let paths: Vec<&Path> = result.removed.iter().map(|d| d.path.as_path()).collect();
    assert!(
        paths
            .iter()
            .any(|p| *p == root.join("backend/target").as_path()),
        "backend/target must be found by rust scope"
    );
    assert!(
        !paths.iter().any(|p| p.starts_with(root.join("web/target"))),
        "rust strategy must NOT walk inside web/ scope for target"
    );
    // web/node_modules should still be found by node scope
    assert!(
        paths
            .iter()
            .any(|p| *p == root.join("web/node_modules").as_path()),
        "web/node_modules must be found by node scope"
    );
}

// --- Case M4: Scoped isolation — node strategy does NOT remove rust trash ------

#[test]
fn dag_scoped_isolation_node_does_not_remove_rust_trash() {
    let (temp, root) = monorepo_fixture();
    let den = temp.path().join("den");
    // Add node_modules inside backend/ — node strategy should NOT touch it
    write(&root, "backend/node_modules/pkg");
    let tree = monorepo_stack_tree(&root);
    let ctx = ctx_dag(&root, &den, RunMode::DryRun);

    let result = rinse_once(&ctx, &dag_rinse_options(&root, tree));

    let paths: Vec<&Path> = result.removed.iter().map(|d| d.path.as_path()).collect();
    assert!(
        !paths
            .iter()
            .any(|p| p.starts_with(root.join("backend/node_modules"))),
        "node strategy must NOT walk inside backend/ scope for node_modules"
    );
    // backend/target should still be found by rust scope
    assert!(
        paths
            .iter()
            .any(|p| *p == root.join("backend/target").as_path()),
        "backend/target must be found by rust scope"
    );
}

// --- Case M5: Three-level nesting — root → rust → node → python --------------

#[test]
fn dag_three_level_nesting_scopes_correctly() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("repo");
    // Level 1: Rust root
    write(&root, "Cargo.toml");
    write(&root, "target/debug/x");
    // Level 2: Node in rust
    write(&root, "frontend/package.json");
    write(&root, "frontend/node_modules/dep");
    // Level 3: Python in node
    write(&root, "frontend/scripts/requirements.txt");
    write(&root, "frontend/scripts/.venv/lib/x");

    let tree = StackNode {
        detection: detection("rust", &root.to_path_buf()),
        children: vec![StackNode {
            detection: detection("node", &root.join("frontend")),
            children: vec![leaf_node("python", &root.join("frontend/scripts"))],
        }],
    };
    let den = temp.path().join("den");
    let ctx = ctx_dag(&root, &den, RunMode::Commit);

    let result = rinse_once(&ctx, &dag_rinse_options(&root, tree));

    assert!(!result.dry_run);
    // All three scopes cleaned
    assert!(
        !root.join("target").exists(),
        "root rust scope target removed"
    );
    assert!(
        !root.join("frontend/node_modules").exists(),
        "level-2 node scope node_modules removed"
    );
    assert!(
        !root.join("frontend/scripts/.venv").exists(),
        "level-3 python scope .venv removed"
    );
    // Source files survive
    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join("frontend/package.json").is_file());
    assert!(root.join("frontend/scripts/requirements.txt").is_file());
}

// --- Case M6: All scopes mismatched → empty result ---------------------------

#[test]
fn dag_all_scopes_mismatched_strategies_returns_empty() {
    let (temp, root) = monorepo_fixture();
    let den = temp.path().join("den");
    let tree = monorepo_stack_tree(&root);
    let ctx = ctx_dag(&root, &den, RunMode::DryRun);

    // Only "generic" strategy enabled — neither "rust" nor "node" are in the tree
    let opts = RinseOptions {
        target: root.clone(),
        strategies: Some(vec!["generic".into()]),
        include_custom_patterns: false,
        collect_only: false,
        stack_tree: Some(tree),
    };

    let result = rinse_once(&ctx, &opts);

    assert!(
        result.removed.is_empty(),
        "mismatched strategies must find nothing: {:?}",
        result.removed
    );
}
