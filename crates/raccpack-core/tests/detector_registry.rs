//! Integration tests for D1.1 — the per-ecosystem detector registry.
//!
//! Locks the contract introduced by the Detect v2 refactor: a stable,
//! ordered registry of [`raccpack_core::detect::StackDetector`] trait objects
//! (`detector_registry()`) behind the central `detect_stack` orchestrator.
//!
//! Covered behavior (stage spec §4):
//! 1. Registry shape — non-empty, unique/non-empty ids, fixed id order
//!    (rust, node, go, python, jvm, ruby, php, cpp, make, git);
//! 2. Every ecosystem detector fires on its fixture through `detect_stack`
//!    (language resolved centrally from marker hints, marker kept, framework
//!    enrichment where applicable);
//! 3. Empty markers ⇒ probe-all smoke (single case; full coverage lives in
//!    `tests/detect_stack.rs`);
//! 4. Behavior-preserving refactor — existing narrow suites stay green.
//!
//! Fixtures are hermetic `tempfile::TempDir`s; no network, no git binary.
//! Language hints mirror the ground-truth tables in `scan/markers/*.rs`.

use std::fs;
use std::path::Path;

use raccpack_core::detect::{detect_stack, detector_registry, StackDetector};
use raccpack_core::scan::{MarkerHit, MarkerKind};

// --- Test helpers -----------------------------------------------------------

/// Build a [`MarkerHit`] with the given fields.
fn hit(name: &str, kind: MarkerKind, language_hint: Option<&str>) -> MarkerHit {
    MarkerHit {
        name: name.to_string(),
        kind,
        language_hint: language_hint.map(str::to_string),
    }
}

/// Write an empty-ish file at `dir/rel`, creating parent directories.
fn write_file(dir: &Path, rel: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, "x").expect("write fixture file");
}

// --- 1. Registry shape -------------------------------------------------------

/// Explicitly pin the registry element type to the trait object: calling
/// through `&'static dyn StackDetector` fails to compile if the registry ever
/// leaks concrete detector types instead.
fn id_of(detector: &dyn StackDetector) -> &'static str {
    detector.id()
}

#[test]
fn registry_is_non_empty_and_ids_are_stable_unique_nonempty() {
    let registry = detector_registry();
    assert!(!registry.is_empty(), "registry must not be empty");

    let ids: Vec<&str> = registry.iter().map(|d| id_of(*d)).collect();
    let expected = [
        "rust", "node", "go", "python", "jvm", "ruby", "php", "cpp", "make", "git",
    ];
    assert_eq!(
        ids, expected,
        "registry ids must appear exactly once, in the fixed order"
    );

    for id in &ids {
        assert!(!id.is_empty(), "every detector id must be non-empty");
    }
}

#[test]
fn registry_returns_the_same_slice_on_every_call() {
    let a = detector_registry();
    let b = detector_registry();
    assert_eq!(a.len(), b.len(), "two calls must agree on length");
    assert!(
        a.iter().zip(b.iter()).all(|(x, y)| x.id() == y.id()),
        "two calls must agree on every detector, order included"
    );
}

// --- 2. Every ecosystem detector fires on its fixture -------------------------

#[test]
fn rust_detector_fires_on_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "proj/Cargo.toml");

    let markers = vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))];
    let stack = detect_stack(&dir.path().join("proj"), &markers).expect("detect must succeed");

    assert_eq!(stack.language.as_deref(), Some("Rust"));
    assert!(stack.markers.contains(&"Cargo.toml".to_string()));
}

#[test]
fn node_detector_fires_on_package_json_with_next_js() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "web/package.json");
    write_file(dir.path(), "web/next.config.mjs");

    let markers = vec![hit(
        "package.json",
        MarkerKind::FileName,
        Some("JavaScript"),
    )];
    let stack = detect_stack(&dir.path().join("web"), &markers).expect("detect must succeed");

    assert_eq!(stack.language.as_deref(), Some("JavaScript"));
    assert!(
        stack.frameworks.contains(&"Next.js".to_string()),
        "next.config.mjs must enrich Next.js, got {:?}",
        stack.frameworks
    );
    assert!(stack.markers.contains(&"package.json".to_string()));
}

#[test]
fn go_detector_fires_on_go_mod() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "svc/go.mod");

    let markers = vec![hit("go.mod", MarkerKind::FileName, Some("Go"))];
    let stack = detect_stack(&dir.path().join("svc"), &markers).expect("detect must succeed");

    assert_eq!(stack.language.as_deref(), Some("Go"));
    assert!(stack.markers.contains(&"go.mod".to_string()));
}

#[test]
fn python_detector_fires_on_pyproject_toml_with_django() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "api/pyproject.toml");
    write_file(dir.path(), "api/manage.py");

    let markers = vec![hit("pyproject.toml", MarkerKind::FileName, Some("Python"))];
    let stack = detect_stack(&dir.path().join("api"), &markers).expect("detect must succeed");

    assert_eq!(stack.language.as_deref(), Some("Python"));
    assert!(
        stack.frameworks.contains(&"Django".to_string()),
        "manage.py must enrich Django, got {:?}",
        stack.frameworks
    );
    assert!(stack.markers.contains(&"pyproject.toml".to_string()));
}

#[test]
fn jvm_detector_fires_on_pom_xml() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "svc/pom.xml");

    let markers = vec![hit("pom.xml", MarkerKind::FileName, Some("Java"))];
    let stack = detect_stack(&dir.path().join("svc"), &markers).expect("detect must succeed");

    // Exact hint from scan/markers/jvm.rs (pom.xml → "Java", not "Java/Kotlin").
    assert_eq!(stack.language.as_deref(), Some("Java"));
    assert!(stack.markers.contains(&"pom.xml".to_string()));
}

#[test]
fn ruby_detector_fires_on_gemfile_with_rails() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "app/Gemfile");
    write_file(dir.path(), "app/config/application.rb");

    let markers = vec![hit("Gemfile", MarkerKind::FileName, Some("Ruby"))];
    let stack = detect_stack(&dir.path().join("app"), &markers).expect("detect must succeed");

    assert_eq!(stack.language.as_deref(), Some("Ruby"));
    assert!(
        stack.frameworks.contains(&"Rails".to_string()),
        "Gemfile + config/application.rb must enrich Rails, got {:?}",
        stack.frameworks
    );
    assert!(stack.markers.contains(&"Gemfile".to_string()));
}

#[test]
fn php_detector_fires_on_composer_json() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "site/composer.json");

    let markers = vec![hit("composer.json", MarkerKind::FileName, Some("PHP"))];
    let stack = detect_stack(&dir.path().join("site"), &markers).expect("detect must succeed");

    // Exact hint from scan/markers/php.rs.
    assert_eq!(stack.language.as_deref(), Some("PHP"));
    assert!(stack.markers.contains(&"composer.json".to_string()));
}

#[test]
fn cpp_detector_fires_on_cmake_lists_txt() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "engine/CMakeLists.txt");

    let markers = vec![hit("CMakeLists.txt", MarkerKind::FileName, Some("C++"))];
    let stack = detect_stack(&dir.path().join("engine"), &markers).expect("detect must succeed");

    // Exact hint from scan/markers/cpp.rs.
    assert_eq!(stack.language.as_deref(), Some("C++"));
    assert!(stack.markers.contains(&"CMakeLists.txt".to_string()));
}

#[test]
fn makefile_marker_kept_without_language() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "tooling/Makefile");

    let markers = vec![hit("Makefile", MarkerKind::FileName, None)];
    let stack = detect_stack(&dir.path().join("tooling"), &markers).expect("detect must succeed");

    assert_eq!(stack.language, None, "Makefile must not imply a language");
    assert!(stack.markers.contains(&"Makefile".to_string()));
}

#[test]
fn git_dirname_marker_kept_without_language() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("repo/.git")).expect("create .git fixture dir");

    let markers = vec![hit(".git", MarkerKind::DirName, None)];
    let stack = detect_stack(&dir.path().join("repo"), &markers).expect("detect must succeed");

    assert_eq!(stack.language, None, "`.git` must not imply a language");
    assert!(stack.markers.contains(&".git".to_string()));
    // is_git_repo flagging belongs to scan/find_candidates, not to detect.
}

// --- 3. Empty markers ⇒ probe-all (single smoke; see tests/detect_stack.rs) ---

#[test]
fn detect_stack_probes_all_detectors_when_markers_are_empty() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "app/vite.config.ts");
    write_file(dir.path(), "app/deno.json");

    let stack = detect_stack(&dir.path().join("app"), &[]).expect("detect must succeed");

    assert!(
        stack.frameworks.contains(&"Vite".to_string()),
        "probe-all must still find Vite, got {:?}",
        stack.frameworks
    );
    assert!(
        stack.frameworks.contains(&"Deno".to_string()),
        "probe-all must still find Deno, got {:?}",
        stack.frameworks
    );
    assert_eq!(
        stack.language, None,
        "framework-only fixture has no marker language"
    );
}
