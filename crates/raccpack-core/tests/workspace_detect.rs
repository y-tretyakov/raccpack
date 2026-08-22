//! Integration tests for D2.1 — composite workspace detection.
//!
//! Covers the 11 required cases of the stage spec:
//! 1.  monorepo rust root + node web subproject in one tree,
//! 2.  single Cargo project → one primary node without children,
//! 3.  markerless root → "unknown" placeholder with children attached,
//! 4.  scope outside the project root is rejected,
//! 5.  missing root / file root map to typed errors,
//! 6.  children sorted by scope regardless of input order (+ determinism),
//! 7.  nested scopes link through the nearest containing ancestor,
//! 8.  produced tree serde roundtrip,
//! 9.  facade sniff in composite mode fills `stack_tree` and flat `stack`,
//! 10. facade priority_table keeps `stack_tree: None` (regression),
//! 11. symlinked directories never become scopes (`follow_links(false)`).
//!
//! Direct-API tests build `markers_by_path` by hand; the symlink test feeds
//! the detector through the real scanner (`find_candidates`) because that is
//! where the no-follow invariant actually lives. Facade tests are `#[serial]`
//! with an isolated `XDG_CACHE_HOME` so tests never touch the real cache.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::app::{sniff, AppContext, NullProgress, RunMode, SniffOptions, SniffResult};
use raccpack_core::detect::{DetectMode, StackNode, WorkspaceDetector};
use raccpack_core::scan::{find_candidates, CandidateOptions, MarkerHit, MarkerKind};
use raccpack_core::{Error, Project, RaccConfig, ScanReport};
use serial_test::serial;
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// A `MarkerHit` as produced by the real marker tables (file-name kind).
fn hit(name: &str, hint: Option<&str>) -> MarkerHit {
    MarkerHit {
        name: name.to_string(),
        kind: MarkerKind::FileName,
        language_hint: hint.map(str::to_string),
    }
}

/// `Cargo.toml` hit exactly as the scan layer would produce it.
fn cargo_hit() -> MarkerHit {
    hit("Cargo.toml", Some("Rust"))
}

/// `package.json` hit exactly as the scan layer would produce it.
fn package_json_hit() -> MarkerHit {
    hit("package.json", Some("JavaScript"))
}

/// `pyproject.toml` hit exactly as the scan layer would produce it.
fn pyproject_hit() -> MarkerHit {
    hit("pyproject.toml", Some("Python"))
}

/// Write a file, creating parent directories on the way.
fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent dir");
    }
    fs::write(path, contents).expect("write fixture file");
}

/// Monorepo fixture: `<tmp>/proj/{Cargo.toml, src/main.rs, web/package.json}`.
struct MonorepoFixture {
    _temp: TempDir,
    root: PathBuf,
}

impl MonorepoFixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temp dir");
        let root = temp.path().join("proj");
        write_file(&root.join("Cargo.toml"), "[package]\nname = \"proj\"\n");
        write_file(&root.join("src/main.rs"), "fn main() {}\n");
        write_file(&root.join("web/package.json"), "{\"name\": \"web\"}\n");
        Self { _temp: temp, root }
    }

    fn web(&self) -> PathBuf {
        self.root.join("web")
    }

    /// `(path, hits)` pairs for root and web, as a real scan would yield them.
    fn markers_by_path(&self) -> Vec<(PathBuf, Vec<MarkerHit>)> {
        vec![
            (self.root.clone(), vec![cargo_hit()]),
            (self.web(), vec![package_json_hit()]),
        ]
    }
}

/// Pre-order list of every scope in a subtree.
fn scopes_preorder(node: &StackNode) -> Vec<PathBuf> {
    let mut out = vec![node.detection.scope.clone()];
    for child in &node.children {
        out.extend(scopes_preorder(child));
    }
    out
}

// Facade helpers (same pattern as `detect_mode_config.rs`).

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

/// The project with the given name, if present in the report.
fn project_named<'a>(report: &'a ScanReport, name: &str) -> &'a Project {
    report
        .projects
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("project {name:?} must be present in the report"))
}

/// Create `<scan_root>/mono` (Cargo.toml + src/main.rs) containing a nested
/// `<scan_root>/mono/web` (package.json); returns both paths.
fn write_facade_monorepo(scan_root: &Path) -> (PathBuf, PathBuf) {
    let mono = scan_root.join("mono");
    write_file(&mono.join("Cargo.toml"), "[package]\nname = \"mono\"\n");
    write_file(&mono.join("src/main.rs"), "fn main() {}\n");
    let web = mono.join("web");
    write_file(&web.join("package.json"), "{\"name\": \"web\"}\n");
    (mono, web)
}

// --- Case 1: Monorepo rust root + node web subproject -----------------------

#[test]
fn monorepo_rust_root_and_web_node_both_in_tree() {
    let fx = MonorepoFixture::new();

    let tree = WorkspaceDetector::new()
        .detect_tree(&fx.root, &fx.markers_by_path())
        .expect("monorepo tree must build");

    assert_eq!(tree.detection.ecosystem, "rust", "root scope must be rust");
    assert_eq!(tree.detection.language.as_deref(), Some("Rust"));
    assert_eq!(tree.detection.confidence, 1.0, "real nodes are confident");
    assert_eq!(tree.detection.scope, fx.root);
    assert_eq!(tree.detection.markers, vec!["Cargo.toml".to_string()]);

    assert_eq!(tree.children.len(), 1, "web must be the only nested scope");
    let web = &tree.children[0];
    assert_eq!(web.detection.ecosystem, "node");
    assert_eq!(web.detection.language.as_deref(), Some("JavaScript"));
    assert_eq!(web.detection.confidence, 1.0);
    assert_eq!(web.detection.scope, fx.web());
    assert_eq!(web.detection.markers, vec!["package.json".to_string()]);
    assert!(web.children.is_empty(), "web has no nested scopes");
}

// --- Case 2: Single project => one primary node ------------------------------

#[test]
fn single_cargo_project_yields_one_primary_node_without_children() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("solo");
    write_file(&root.join("Cargo.toml"), "[package]\nname = \"solo\"\n");

    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &[(root.clone(), vec![cargo_hit()])])
        .expect("single-scope tree must build");

    assert_eq!(tree.detection.ecosystem, "rust");
    assert_eq!(tree.detection.language.as_deref(), Some("Rust"));
    assert_eq!(tree.detection.scope, root);
    assert_eq!(tree.detection.markers, vec!["Cargo.toml".to_string()]);
    assert!(
        tree.children.is_empty(),
        "no nested marker scopes => no children"
    );
}

// --- Case 3: Markerless root placeholder -------------------------------------

#[test]
fn markerless_root_gets_unknown_placeholder_with_children() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("proj");
    let lib = root.join("lib-py");
    write_file(&lib.join("pyproject.toml"), "[project]\n");

    // No entry for the root itself — only the subfolder carries markers.
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &[(lib.clone(), vec![pyproject_hit()])])
        .expect("placeholder-root tree must build");

    assert_eq!(
        tree.detection.ecosystem, "unknown",
        "markerless root must be an unknown placeholder"
    );
    assert_eq!(tree.detection.confidence, 0.0);
    assert_eq!(tree.detection.language, None);
    assert!(tree.detection.markers.is_empty());

    assert_eq!(tree.children.len(), 1, "child scope must still be present");
    assert_eq!(tree.children[0].detection.scope, lib);
    assert_eq!(tree.children[0].detection.ecosystem, "python");
    assert_eq!(
        tree.children[0].detection.language.as_deref(),
        Some("Python")
    );
    assert_eq!(tree.children[0].detection.confidence, 1.0);
}

// --- Case 4: Scope outside the project root ----------------------------------

#[test]
fn scope_outside_project_root_is_rejected() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("proj");
    write_file(&root.join("Cargo.toml"), "[package]\n");

    // Physically existing sibling next to the project root.
    let sibling = temp.path().join("sibling-web");
    write_file(&sibling.join("package.json"), "{}\n");

    let err = WorkspaceDetector::new()
        .detect_tree(
            &root,
            &[
                (root.clone(), vec![cargo_hit()]),
                (sibling.clone(), vec![package_json_hit()]),
            ],
        )
        .expect_err("scope outside project_root must be rejected");

    let rendered = err.to_string();
    assert!(
        rendered.contains("sibling-web"),
        "error must name the offending scope, got: {rendered}"
    );
}

// --- Case 5: Missing / non-directory roots -----------------------------------

#[test]
fn missing_root_and_file_root_map_to_typed_errors() {
    let temp = TempDir::new().expect("create temp dir");
    let missing = temp.path().join("absent-project");
    let file_root = temp.path().join("plain-file.txt");
    fs::write(&file_root, "not a directory").expect("write fixture file");

    let detector = WorkspaceDetector::new();

    let err = detector
        .detect_tree(&missing, &[])
        .expect_err("missing root must fail");
    assert!(
        matches!(err, Error::PathNotFound { ref path } if *path == missing),
        "expected PathNotFound for a missing root"
    );

    let err = detector
        .detect_tree(&file_root, &[])
        .expect_err("file-as-root must fail");
    assert!(
        matches!(err, Error::NotADirectory { ref path } if *path == file_root),
        "expected NotADirectory for a plain-file root"
    );
}

// --- Case 6: Sorted children + deterministic output --------------------------

#[test]
fn children_are_sorted_by_scope_regardless_of_input_order() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("proj");
    let a = root.join("a-service");
    let z = root.join("z-service");
    write_file(&a.join("Cargo.toml"), "[package]\n");
    write_file(&z.join("Cargo.toml"), "[package]\n");

    let a_entry = (a.clone(), vec![cargo_hit()]);
    let z_entry = (z.clone(), vec![cargo_hit()]);
    let detector = WorkspaceDetector::new();

    let first = detector
        .detect_tree(&root, &[z_entry.clone(), a_entry.clone()])
        .expect("first run must succeed");
    let second = detector
        .detect_tree(&root, &[a_entry, z_entry])
        .expect("second run must succeed");

    assert_eq!(
        first, second,
        "equal inputs must produce PartialEq-equal trees regardless of order"
    );
    assert_eq!(first.children.len(), 2);
    assert!(
        first
            .children
            .windows(2)
            .all(|w| w[0].detection.scope < w[1].detection.scope),
        "children must be sorted ascending by scope path"
    );
    assert_eq!(first.children[0].detection.scope, a);
    assert_eq!(first.children[1].detection.scope, z);
    assert!(first
        .children
        .iter()
        .all(|c| c.detection.ecosystem == "rust"));
}

// --- Case 7: Nearest-ancestor nesting ----------------------------------------

#[test]
fn nested_scopes_link_through_nearest_ancestor() {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("proj");
    let a = root.join("a");
    let ab = a.join("b");
    write_file(&a.join("Cargo.toml"), "[package]\n");
    write_file(&ab.join("package.json"), "{}\n");

    // Root has no markers; a/ and a/b/ both do.
    let tree = WorkspaceDetector::new()
        .detect_tree(
            &root,
            &[
                (a.clone(), vec![cargo_hit()]),
                (ab.clone(), vec![package_json_hit()]),
            ],
        )
        .expect("nested tree must build");

    assert_eq!(tree.detection.ecosystem, "unknown");
    assert_eq!(
        tree.children.len(),
        1,
        "only a/ may hang directly off the root"
    );

    let a_node = &tree.children[0];
    assert_eq!(a_node.detection.scope, a);
    assert_eq!(a_node.detection.ecosystem, "rust");
    assert_eq!(
        a_node.children.len(),
        1,
        "a/b must nest under its nearest ancestor a/"
    );

    let ab_node = &a_node.children[0];
    assert_eq!(ab_node.detection.scope, ab);
    assert_eq!(ab_node.detection.ecosystem, "node");
    assert!(ab_node.children.is_empty());
}

// --- Case 8: Serde roundtrip -------------------------------------------------

#[test]
fn produced_tree_serde_roundtrips() {
    let fx = MonorepoFixture::new();

    let tree = WorkspaceDetector::new()
        .detect_tree(&fx.root, &fx.markers_by_path())
        .expect("monorepo tree must build");

    let json = serde_json::to_string(&tree).expect("serialize StackNode");
    let back: StackNode = serde_json::from_str(&json).expect("deserialize StackNode");
    assert_eq!(back, tree, "serde roundtrip must preserve the whole tree");
}

// --- Case 9: Facade composite mode fills stack_tree + flat stack -------------

#[test]
#[serial]
fn facade_composite_sniff_fills_stack_tree_and_flat_stack() {
    let (temp, scan_root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    let (mono, web) = write_facade_monorepo(&scan_root);
    let ctx = ctx_for(&scan_root);

    let opts = SniffOptions {
        detect_mode: Some(DetectMode::CompositeDag),
        ..SniffOptions::default()
    };
    let result = sniff_once(&ctx, &opts);

    assert!(!result.from_cache, "first composite run must compute");
    let mono_project = project_named(&result.report, "mono");
    assert_eq!(
        mono_project.stack.language.as_deref(),
        Some("Rust"),
        "flat stack must stay filled alongside the tree"
    );

    let tree = mono_project
        .stack_tree
        .as_ref()
        .expect("composite mode must attach stack_tree to the monorepo project");
    assert_eq!(tree.detection.ecosystem, "rust", "tree root must be rust");
    assert_eq!(tree.detection.scope, mono);
    assert_eq!(tree.children.len(), 1, "exactly one nested scope expected");
    assert_eq!(tree.children[0].detection.ecosystem, "node");
    assert_eq!(tree.children[0].detection.scope, web);
}

// --- Case 10: Priority-table regression --------------------------------------

#[test]
#[serial]
fn facade_priority_table_keeps_stack_tree_none() {
    let (temp, scan_root) = workspace();
    let _env = CacheEnvGuard::set(&isolated_cache_dir(&temp));
    write_facade_monorepo(&scan_root);
    let ctx = ctx_for(&scan_root);

    let result = sniff_once(&ctx, &SniffOptions::default());

    assert!(!result.from_cache);

    let mut names: Vec<String> = result
        .report
        .projects
        .iter()
        .map(|p| p.name.clone())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["mono".to_string(), "web".to_string()],
        "flat discovery behavior must be unchanged"
    );

    for project in &result.report.projects {
        assert!(
            project.stack_tree.is_none(),
            "priority_table must not attach stack trees"
        );
    }
    let mono = project_named(&result.report, "mono");
    assert_eq!(mono.stack.language.as_deref(), Some("Rust"));
}

// --- Case 11: Symlinked directories never become scopes ----------------------

#[test]
fn symlinked_dir_is_never_a_scope() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("create temp dir");
    let proj = temp.path().join("proj");
    write_file(&proj.join("Cargo.toml"), "[package]\nname = \"proj\"\n");

    // A real node project OUTSIDE the project root, referenced only through
    // an in-project symlink named linked-web.
    let target = temp.path().join("outside/node-lib");
    write_file(&target.join("package.json"), "{\"name\": \"node-lib\"}\n");
    symlink(&target, proj.join("linked-web")).expect("create fixture symlink");
    assert!(
        proj.join("linked-web").exists(),
        "fixture sanity: the symlink must resolve"
    );

    // Feed the detector through the real scanner: the walk never descends
    // into symlinks, so the linked directory can never enter markers_by_path.
    let candidates = find_candidates(&proj, &CandidateOptions::default())
        .expect("scanning the fixture must succeed");
    assert_eq!(
        candidates.len(),
        1,
        "only the project root may become a candidate"
    );
    let markers_by_path: Vec<(PathBuf, Vec<MarkerHit>)> = candidates
        .into_iter()
        .map(|c| (c.path, c.markers))
        .collect();

    let tree = WorkspaceDetector::new()
        .detect_tree(&proj, &markers_by_path)
        .expect("tree must build");

    let all_scopes = scopes_preorder(&tree);
    assert!(
        !all_scopes.contains(&proj.join("linked-web")),
        "symlinked directory must never appear as a scope"
    );
    assert_eq!(tree.detection.ecosystem, "rust");
    assert!(
        tree.children.is_empty(),
        "no scopes may leak into the tree through symlinks"
    );
}
