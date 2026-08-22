//! Integration tests for M2.3 — facade `sniff` + versioned cache.
//!
//! Covers the 9 required cases from spec §8: empty root, two-project
//! fixture, skip-policy, cache hit, force_refresh, max_depth keying,
//! progress events, bad root, and a direct cache serde roundtrip, plus two
//! direct cache-API miss cases.
//!
//! Every test that touches `XDG_CACHE_HOME` is `#[serial]` and uses a fresh
//! isolated cache directory so tests never read or write the real
//! `~/.cache/raccpack`.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::app::{
    sniff, AppContext, NullProgress, OperationKind, ProgressEvent, ProgressSink, RunMode,
    SniffOptions, SniffResult,
};
use raccpack_core::cache::{store_sniff_cache, try_load_sniff_cache};
use raccpack_core::{Error, Project, RaccConfig, ScanReport, Stack};
use serial_test::serial;
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Restores the previous `XDG_CACHE_HOME` value on drop, even on panic.
struct CacheEnvGuard {
    previous: Option<OsString>,
}

impl CacheEnvGuard {
    /// Capture the current `XDG_CACHE_HOME` and point it at `cache_home`.
    fn set(cache_home: &Path) -> Self {
        let previous = env::var_os("XDG_CACHE_HOME");
        env::set_var("XDG_CACHE_HOME", cache_home);
        Self { previous }
    }
}

impl Drop for CacheEnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => env::set_var("XDG_CACHE_HOME", value),
            None => env::remove_var("XDG_CACHE_HOME"),
        }
    }
}

/// Create a fresh, empty cache directory inside `work` and return its path.
fn isolated_cache_dir(work: &TempDir) -> PathBuf {
    let dir = work.path().join("xdg-cache");
    fs::create_dir_all(&dir).expect("create isolated cache dir");
    dir
}

/// Create a workspace: a `TempDir` with an existing `projects/` scan root.
/// Returns the tempdir and the scan root.
fn workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create work dir");
    let projects = temp.path().join("projects");
    fs::create_dir_all(&projects).expect("create projects dir");
    (temp, projects)
}

/// Build an `AppContext` from a config pointing at `root` (den is derived as a
/// sibling of the scan root so no real `~/.raccpack/den` is ever touched).
fn ctx_for(root: &Path) -> AppContext {
    let den = root.parent().expect("scan root has a parent").join("den");
    let config = RaccConfig::default()
        .with_scan_root(root)
        .with_den_dir(&den);
    AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config")
}

/// Run sniff with a `NullProgress` sink and return the result.
fn sniff_once(ctx: &AppContext, opts: &SniffOptions) -> SniffResult {
    let mut progress = NullProgress;
    sniff(ctx, opts, &mut progress).expect("sniff should succeed")
}

/// Run sniff recording every progress event; returns the result and the events.
fn sniff_recorded(ctx: &AppContext, opts: &SniffOptions) -> (SniffResult, Vec<ProgressEvent>) {
    let mut sink = RecordingSink::default();
    let result = sniff(ctx, opts, &mut sink).expect("sniff should succeed");
    (result, sink.events)
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

/// Sorted project names of a report (walk order is not meaningful).
fn project_names(report: &ScanReport) -> Vec<String> {
    let mut names: Vec<String> = report.projects.iter().map(|p| p.name.clone()).collect();
    names.sort();
    names
}

/// Sorted project paths of a report (walk order is not meaningful).
fn project_paths(report: &ScanReport) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = report.projects.iter().map(|p| p.path.clone()).collect();
    paths.sort();
    paths
}

/// Size in bytes of the project with the given name, if present.
fn size_of(report: &ScanReport, name: &str) -> Option<u64> {
    report
        .projects
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.size_bytes)
}

/// Create `projects/app-rust` (Cargo.toml + src/main.rs) and
/// `projects/app-node` (package.json + index.js), each with >0 bytes.
fn write_two_project_fixture(root: &Path) {
    let rust = root.join("app-rust");
    fs::create_dir_all(rust.join("src")).expect("create app-rust/src");
    fs::write(rust.join("Cargo.toml"), "[package]\nname = \"app-rust\"\n")
        .expect("write Cargo.toml");
    fs::write(rust.join("src/main.rs"), "fn main() {}\n").expect("write src/main.rs");

    let node = root.join("app-node");
    fs::create_dir_all(&node).expect("create app-node");
    fs::write(node.join("package.json"), "{\"name\": \"app-node\"}\n").expect("write package.json");
    fs::write(node.join("index.js"), "console.log('hi')\n").expect("write index.js");
}

// --- Case 1: Empty root -----------------------------------------------------

#[test]
#[serial]
fn sniff_empty_root_returns_zero_projects() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    let ctx = ctx_for(&root);

    let result = sniff_once(&ctx, &SniffOptions::default());

    assert!(!result.from_cache, "a fresh cache cannot produce a hit");
    let report = &result.report;
    assert_eq!(report.root, root);
    assert!(report.projects.is_empty(), "no markers => no projects");
    assert_eq!(report.total_size_bytes, 0);
    assert_eq!(report.schema_version, 1);
}

// --- Case 2: Two-project fixture --------------------------------------------

#[test]
#[serial]
fn sniff_two_projects_discovers_both_with_sizes() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_two_project_fixture(&root);
    let ctx = ctx_for(&root);

    let report = &sniff_once(&ctx, &SniffOptions::default()).report;

    assert_eq!(
        project_names(report),
        vec!["app-node".to_string(), "app-rust".to_string()]
    );
    assert_eq!(
        project_paths(report),
        vec![root.join("app-node"), root.join("app-rust")]
    );
    assert!(
        report.total_size_bytes > 0,
        "fixture files must count toward total size"
    );
    assert!(report.projects.iter().all(|p| p.size_bytes > 0));
    assert_eq!(report.schema_version, 1);

    let rust = report
        .projects
        .iter()
        .find(|p| p.name == "app-rust")
        .expect("app-rust present");
    assert_eq!(rust.stack.language.as_deref(), Some("Rust"));
    let node = report
        .projects
        .iter()
        .find(|p| p.name == "app-node")
        .expect("app-node present");
    assert_eq!(node.stack.language.as_deref(), Some("JavaScript"));
}

// --- Case 3: Skip policy (node_modules / target) -----------------------------

#[test]
#[serial]
fn sniff_skips_node_modules_and_target_dirs() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));

    let node = root.join("app-node");
    fs::create_dir_all(node.join("node_modules/pkg")).expect("create node_modules/pkg");
    fs::write(node.join("package.json"), "{}").expect("write package.json");
    fs::write(node.join("index.js"), "console.log('hi')\n").expect("write index.js");
    fs::write(node.join("node_modules/pkg/package.json"), "{}").expect("write nested package.json");

    let rust = root.join("app-rust");
    fs::create_dir_all(rust.join("target/debug")).expect("create target/debug");
    fs::write(rust.join("Cargo.toml"), "[package]\n").expect("write Cargo.toml");
    fs::write(rust.join("target/debug/Cargo.toml"), "[package]\n")
        .expect("write target Cargo.toml");

    let ctx = ctx_for(&root);
    let report = &sniff_once(&ctx, &SniffOptions::default()).report;

    assert_eq!(
        project_names(report),
        vec!["app-node".to_string(), "app-rust".to_string()]
    );
    assert!(
        report.projects.iter().all(|p| {
            !p.path.to_string_lossy().contains("node_modules")
                && !p.path.to_string_lossy().contains("target")
        }),
        "directories under node_modules/target must never become projects"
    );
    let expected = "{}".len() + "console.log('hi')\n".len();
    assert_eq!(
        size_of(report, "app-node"),
        Some(expected as u64),
        "bytes under node_modules must not count toward project size"
    );
}

// --- Case 4: Cache hit ------------------------------------------------------

#[test]
#[serial]
fn sniff_cache_hit_on_second_call() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_two_project_fixture(&root);
    let ctx = ctx_for(&root);

    let opts = SniffOptions::default();
    let first = sniff_once(&ctx, &opts);
    let second = sniff_once(&ctx, &opts);

    assert!(!first.from_cache, "first sniff must compute from scratch");
    assert!(second.from_cache, "second sniff must hit the cache");
    assert_eq!(
        first.report, second.report,
        "cached report must equal a fresh report"
    );
}

// --- Case 5: force_refresh ---------------------------------------------------

#[test]
#[serial]
fn sniff_force_refresh_recomputes() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_two_project_fixture(&root);
    let ctx = ctx_for(&root);

    let opts = SniffOptions::default();
    let first = sniff_once(&ctx, &opts);
    assert!(!first.from_cache);

    let forced = SniffOptions {
        force_refresh: true,
        max_depth: None,
        detect_mode: None,
    };
    let refreshed = sniff_once(&ctx, &forced);
    assert!(
        !refreshed.from_cache,
        "force_refresh must ignore an existing cache"
    );
    assert_eq!(first.report, refreshed.report);

    let again = sniff_once(&ctx, &opts);
    assert!(
        again.from_cache,
        "cache must remain usable after a force refresh"
    );
}

// --- Case 6: max_depth change ------------------------------------------------

#[test]
#[serial]
fn sniff_cache_miss_on_max_depth_change() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_two_project_fixture(&root);
    let ctx = ctx_for(&root);

    let deep = SniffOptions {
        force_refresh: false,
        max_depth: Some(6),
        detect_mode: None,
    };
    let shallow = SniffOptions {
        force_refresh: false,
        max_depth: Some(2),
        detect_mode: None,
    };

    let first = sniff_once(&ctx, &deep);
    assert!(!first.from_cache);

    let second = sniff_once(&ctx, &shallow);
    assert!(
        !second.from_cache,
        "a different max_depth must be a cache miss"
    );

    let third = sniff_once(&ctx, &deep);
    assert!(
        third.from_cache,
        "the same max_depth must hit the cache again"
    );
}

// --- Case 7: Progress -------------------------------------------------------

#[test]
#[serial]
fn sniff_emits_monotonic_scan_progress() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_two_project_fixture(&root);
    let ctx = ctx_for(&root);

    let (result, events) = sniff_recorded(&ctx, &SniffOptions::default());

    assert!(!result.from_cache);
    assert!(
        !events.is_empty(),
        "sniff must emit at least one progress event"
    );
    assert!(
        events.iter().all(|e| e.operation == OperationKind::Sniff),
        "all progress events must belong to Sniff"
    );
    assert!(
        events.iter().all(|e| e.phase == "scan"),
        "sniff progress phase must be \"scan\""
    );
    assert!(
        events.windows(2).all(|w| w[0].percent <= w[1].percent),
        "progress percent must never decrease"
    );

    let last = events.last().expect("events is non-empty");
    assert!(
        last.phase_complete,
        "the last event must mark the phase complete"
    );
    assert_eq!(last.percent, 100);
    assert_eq!(last.overall_percent, 100);
    assert_eq!(events[0].phase_index, 0, "phase indices are 0-based");
}

// --- Case 8: Bad root -------------------------------------------------------

#[test]
#[serial]
fn sniff_missing_scan_root_is_path_not_found() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    let missing = root.join("does-not-exist");
    let mut ctx = ctx_for(&root);
    ctx.paths.scan_root = missing.clone();

    let mut progress = NullProgress;
    let err = sniff(&ctx, &SniffOptions::default(), &mut progress).expect_err("sniff must fail");

    assert!(matches!(err, Error::PathNotFound { path } if path == missing));
}

#[test]
#[serial]
fn sniff_file_scan_root_is_not_a_directory() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    let file = root.join("a-file.txt");
    fs::write(&file, "not a directory").expect("write file");
    let mut ctx = ctx_for(&root);
    ctx.paths.scan_root = file.clone();

    let mut progress = NullProgress;
    let err = sniff(&ctx, &SniffOptions::default(), &mut progress).expect_err("sniff must fail");

    assert!(matches!(err, Error::NotADirectory { path } if path == file));
}

// --- Case 9: Direct cache API serde roundtrip --------------------------------

#[test]
#[serial]
fn cache_api_serde_roundtrip() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));

    let report = ScanReport {
        root: root.clone(),
        projects: vec![Project {
            path: root.join("app-rust"),
            name: "app-rust".to_string(),
            stack: Stack {
                language: Some("Rust".to_string()),
                frameworks: vec!["Axum".to_string()],
                markers: vec!["Cargo.toml".to_string()],
            },
            stack_tree: None,
            size_bytes: 4096,
            is_git_repo: false,
        }],
        total_size_bytes: 4096,
        schema_version: 1,
    };

    store_sniff_cache(&root, 6, "default_scan_v1", &report).expect("store must succeed");
    let loaded = try_load_sniff_cache(&root, 6, "default_scan_v1")
        .expect("load must succeed")
        .expect("the cache file must exist after store");

    assert_eq!(
        loaded, report,
        "serde roundtrip through the cache must preserve the report"
    );
}

// --- Direct cache API: miss cases -------------------------------------------

#[test]
#[serial]
fn cache_api_miss_when_no_file() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));

    let loaded = try_load_sniff_cache(&root, 6, "default_scan_v1").expect("load must succeed");
    assert!(loaded.is_none(), "no cache file => no hit");
}

#[test]
#[serial]
fn cache_api_miss_on_different_max_depth() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));

    let report = ScanReport {
        root: root.clone(),
        projects: Vec::new(),
        total_size_bytes: 0,
        schema_version: 1,
    };
    store_sniff_cache(&root, 6, "default_scan_v1", &report).expect("store must succeed");

    let loaded = try_load_sniff_cache(&root, 5, "default_scan_v1").expect("load must succeed");
    assert!(
        loaded.is_none(),
        "different max_depth => different cache key"
    );
}
