//! Integration tests for D1.3 — `detect.mode` config/CLI surface (core).
//!
//! Covers the config→mode contract ([`RaccConfig`] parsing, unknown-mode
//! rejection) and the facade behaviour of the resolved mode inside `sniff`:
//! precedence (CLI override > config > default), the composite pipeline
//! execution (`stack_tree` filled, flat stack kept) and that the default path
//! (cache hit, report shape) is unchanged.
//!
//! Every test that reaches the cache layer is `#[serial]` and uses a fresh
//! isolated cache directory so tests never read or write the real
//! `~/.cache/raccpack`.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::app::{sniff, AppContext, NullProgress, RunMode, SniffOptions, SniffResult};
use raccpack_core::config::{ConfigError, RaccConfig};
use raccpack_core::detect::DetectMode;
use raccpack_core::ScanReport;
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
fn ctx_for(config: RaccConfig, root: &Path) -> AppContext {
    let den = root.parent().expect("scan root has a parent").join("den");
    let config = config.with_scan_root(root).with_den_dir(&den);
    AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config")
}

/// Default config with `detect.mode` replaced.
fn config_with_mode(mode: DetectMode) -> RaccConfig {
    let mut config = RaccConfig::default();
    config.detect.mode = mode;
    config
}

/// Load a config from a TOML body written into a throwaway directory.
fn load_toml(body: &str) -> Result<RaccConfig, ConfigError> {
    let dir = TempDir::new().expect("create config dir");
    let path = dir.path().join("config.toml");
    fs::write(&path, body).expect("write config fixture");
    RaccConfig::load_from_path(&path)
}

/// Run sniff with a `NullProgress` sink and return the result.
fn sniff_once(ctx: &AppContext, opts: &SniffOptions) -> SniffResult {
    let mut progress = NullProgress;
    sniff(ctx, opts, &mut progress).expect("sniff should succeed")
}

/// Language reported for the project with the given name, if present.
fn language_of(report: &ScanReport, name: &str) -> Option<String> {
    report
        .projects
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| p.stack.language.clone())
}

/// Create `projects/app-rust` (Cargo.toml + src/main.rs).
fn write_rust_project_fixture(root: &Path) {
    let rust = root.join("app-rust");
    fs::create_dir_all(rust.join("src")).expect("create app-rust/src");
    fs::write(rust.join("Cargo.toml"), "[package]\nname = \"app-rust\"\n")
        .expect("write Cargo.toml");
    fs::write(rust.join("src/main.rs"), "fn main() {}\n").expect("write src/main.rs");
}

// --- Case 1: Missing [detect] section ---------------------------------------

#[test]
fn missing_detect_section_defaults_to_priority_table() {
    let config = load_toml("[paths]\nscan_root = '/tmp'\n[scanner]\nmax_depth = 3\n")
        .expect("minimal TOML must parse");

    assert_eq!(config.detect.mode, DetectMode::PriorityTable);
}

// --- Case 2: Canonical TOML strings -----------------------------------------

#[test]
fn detect_mode_parses_from_toml_strings() {
    for (text, expected) in [
        ("priority_table", DetectMode::PriorityTable),
        ("composite_dag", DetectMode::CompositeDag),
    ] {
        let config =
            load_toml(&format!("[detect]\nmode = \"{text}\"\n")).expect("canonical mode parses");
        assert_eq!(config.detect.mode, expected, "mode string {text:?}");
    }
}

// --- Case 3: Unknown mode ----------------------------------------------------

#[test]
fn unknown_config_mode_is_rejected_with_suggestion() {
    let err = load_toml("[detect]\nmode = \"bogus_pipeline\"\n")
        .expect_err("unknown mode must be rejected");

    assert!(
        matches!(err, ConfigError::UnknownDetectMode { ref value } if value == "bogus_pipeline"),
        "expected UnknownDetectMode, got {err:?}"
    );
    let rendered = err.to_string();
    assert!(
        rendered.contains("priority_table") && rendered.contains("composite_dag"),
        "error text must list both valid modes, got: {rendered}"
    );
    let suggestion = err.suggestion().expect("unknown mode must carry a hint");
    assert!(
        suggestion.contains("priority_table") && suggestion.contains("composite_dag"),
        "suggestion must list both valid modes, got: {suggestion}"
    );
}

// --- Case 4: CompositeDag pipeline runs --------------------------------------

#[test]
#[serial]
fn composite_dag_sniff_fills_stack_tree_and_keeps_flat_stack() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));

    // Monorepo fixture: rust root + nested node package.
    let repo = root.join("monorepo");
    fs::create_dir_all(repo.join("web")).expect("create monorepo/web");
    fs::write(repo.join("Cargo.toml"), "[package]\nname = \"mono\"\n").expect("write Cargo.toml");
    fs::write(repo.join("web").join("package.json"), "{}").expect("write package.json");

    let ctx = ctx_for(config_with_mode(DetectMode::CompositeDag), &root);
    let result = sniff_once(&ctx, &SniffOptions::default());

    let project = result
        .report
        .projects
        .iter()
        .find(|p| p.name == "monorepo")
        .expect("monorepo must be discovered");
    // Flat-stack invariant holds in composite mode too.
    assert_eq!(project.stack.language.as_deref(), Some("Rust"));
    assert!(project.stack.frameworks.is_empty());

    // Composite tree carries both ecosystems.
    let tree = project.stack_tree.as_ref().expect("stack_tree is filled");
    assert_eq!(tree.detection.ecosystem, "rust");
    assert_eq!(tree.detection.language.as_deref(), Some("Rust"));
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].detection.ecosystem, "node");
    assert_eq!(
        tree.children[0].detection.language.as_deref(),
        Some("JavaScript")
    );
}

// --- Case 5: CLI override beats the config section ---------------------------

#[test]
#[serial]
fn cli_override_wins_over_config() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_rust_project_fixture(&root);

    // config = composite_dag (real TOML parse) + CLI override priority_table
    // => sniff proceeds normally.
    let dag_config =
        load_toml("[detect]\nmode = \"composite_dag\"\n").expect("composite_dag config parses");
    let ctx = ctx_for(dag_config, &root);
    let overridden = SniffOptions {
        detect_mode: Some(DetectMode::PriorityTable),
        ..SniffOptions::default()
    };
    let result = sniff_once(&ctx, &overridden);
    assert!(!result.from_cache);
    let flat_project = result
        .report
        .projects
        .iter()
        .find(|p| p.name == "app-rust")
        .expect("app-rust discovered");
    assert_eq!(
        language_of(&result.report, "app-rust").as_deref(),
        Some("Rust"),
        "override must restore the default pipeline"
    );
    assert!(
        flat_project.stack_tree.is_none(),
        "priority_table must not produce a stack tree"
    );

    // config = priority_table + CLI override composite_dag => DAG pipeline runs.
    let ctx = ctx_for(RaccConfig::default(), &root);
    let forced_dag = SniffOptions {
        detect_mode: Some(DetectMode::CompositeDag),
        ..SniffOptions::default()
    };
    let dag_result = sniff_once(&ctx, &forced_dag);
    assert!(!dag_result.from_cache);
    let dag_project = dag_result
        .report
        .projects
        .iter()
        .find(|p| p.name == "app-rust")
        .expect("app-rust discovered");
    assert_eq!(
        language_of(&dag_result.report, "app-rust").as_deref(),
        Some("Rust")
    );
    assert!(
        dag_project.stack_tree.is_some(),
        "override must select the composite pipeline"
    );
}

// --- Case 6: Explicit priority_table == default ------------------------------

#[test]
#[serial]
fn explicit_priority_table_matches_default_behavior() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_rust_project_fixture(&root);
    let ctx = ctx_for(RaccConfig::default(), &root);

    let default_run = sniff_once(&ctx, &SniffOptions::default());
    let explicit_opts = SniffOptions {
        force_refresh: true,
        max_depth: None,
        detect_mode: Some(DetectMode::PriorityTable),
    };
    let explicit_run = sniff_once(&ctx, &explicit_opts);

    assert!(!explicit_run.from_cache, "force_refresh must recompute");
    assert_eq!(
        default_run.report.projects.len(),
        explicit_run.report.projects.len(),
        "both runs must discover the same number of projects"
    );
    assert_eq!(
        language_of(&default_run.report, "app-rust"),
        language_of(&explicit_run.report, "app-rust"),
        "explicit priority_table must resolve the same language as the default"
    );
}

// --- Case 7: Default run keeps the versioned cache path ----------------------

#[test]
#[serial]
fn default_run_keeps_cache_hit() {
    let (temp, root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_rust_project_fixture(&root);
    let ctx = ctx_for(RaccConfig::default(), &root);

    let opts = SniffOptions::default();
    let first = sniff_once(&ctx, &opts);
    let second = sniff_once(&ctx, &opts);

    assert!(!first.from_cache, "first default run must compute");
    assert!(
        second.from_cache,
        "the default detect path must keep using the sniff cache"
    );
    assert_eq!(first.report, second.report);
}
