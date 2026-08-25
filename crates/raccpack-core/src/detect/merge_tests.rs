//! Unit tests for the D2.2 conflict/merge policy ([`super::merge`]):
//! same-scope [`Detection`] merging, framework union/dedup, the confidence
//! threshold constant and the deterministic ordering of composite trees built
//! by [`WorkspaceDetector`] (spec `docs/detect/d2/d2.2-conflict-merge.md` §3).
//!
//! Tree fixtures build real directories under a `tempfile::TempDir`, matching
//! the [`super::workspace_tests`] style: scope paths must exist because
//! containment validation canonicalizes them.

use std::path::{Path, PathBuf};

use super::merge::{extend_frameworks_union, merge_same_scope, DEFAULT_CONFIDENCE_THRESHOLD};
use super::tests::hit;
use super::*;

/// Minimal [`Detection`] fixture over ecosystem, frameworks, confidence and
/// marker names; scope is a constant placeholder (merge policy must not care).
fn detection(ecosystem: &str, frameworks: &[&str], confidence: f32, markers: &[&str]) -> Detection {
    Detection {
        ecosystem: ecosystem.to_string(),
        language: None,
        frameworks: frameworks.iter().map(|name| name.to_string()).collect(),
        confidence,
        scope: PathBuf::from("/tmp/fixture"),
        markers: markers.iter().map(|name| name.to_string()).collect(),
    }
}

/// Create an existing directory under `base` and return its path.
fn make_dir(base: &Path, name: &str) -> PathBuf {
    let path = base.join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn merge_same_scope_dedups_frameworks_keeping_first_occurrence_order() {
    let base = detection("node", &["Next.js", "Vite"], 0.8, &["package.json"]);
    let extra = detection("node", &["Vite", "Deno"], 0.9, &["deno.json"]);

    let merged = merge_same_scope(base, extra);

    assert_eq!(
        merged.frameworks,
        vec![
            "Next.js".to_string(),
            "Vite".to_string(),
            "Deno".to_string()
        ]
    );
}

#[test]
fn merge_same_scope_confidence_is_max_of_both_sides_in_either_direction() {
    let higher_first = merge_same_scope(
        detection("node", &[], 0.9, &["package.json"]),
        detection("node", &[], 0.4, &["deno.json"]),
    );
    assert_eq!(higher_first.confidence, 0.9);

    let higher_second = merge_same_scope(
        detection("node", &[], 0.4, &["package.json"]),
        detection("node", &[], 0.9, &["deno.json"]),
    );
    assert_eq!(higher_second.confidence, 0.9);
}

#[test]
fn merge_same_scope_clamps_out_of_range_confidence_like_producers_must() {
    let merged = merge_same_scope(
        detection("node", &[], 2.5, &["package.json"]),
        detection("node", &[], 0.5, &["deno.json"]),
    );

    assert_eq!(merged.confidence, clamp_confidence(2.5));
    assert_eq!(merged.confidence, 1.0);
}

#[test]
fn merge_same_scope_unions_markers_sorted_and_unique() {
    let merged = merge_same_scope(
        detection("node", &[], 0.5, &["zeta.json", "alpha.json"]),
        detection("node", &[], 0.5, &["alpha.json", "mid.json"]),
    );

    assert_eq!(
        merged.markers,
        vec![
            "alpha.json".to_string(),
            "mid.json".to_string(),
            "zeta.json".to_string()
        ]
    );
}

#[test]
fn merge_same_scope_language_prefers_base_and_falls_back_to_extra() {
    let mut base_with = detection("rust", &[], 1.0, &["Cargo.toml"]);
    base_with.language = Some("Rust".to_string());
    let mut extra_with = detection("rust", &[], 0.5, &["Makefile"]);
    extra_with.language = Some("Other".to_string());

    assert_eq!(
        merge_same_scope(base_with.clone(), extra_with.clone())
            .language
            .as_deref(),
        Some("Rust")
    );

    let mut base_without = base_with;
    base_without.language = None;
    assert_eq!(
        merge_same_scope(base_without, extra_with)
            .language
            .as_deref(),
        Some("Other")
    );
}

#[test]
fn extend_frameworks_union_appends_only_new_names_in_input_order() {
    let mut target = vec!["Axum".to_string(), "Vite".to_string()];

    extend_frameworks_union(
        &mut target,
        [
            "Vite".to_string(),
            "Deno".to_string(),
            "Axum".to_string(),
            "Rocket".to_string(),
        ],
    );

    assert_eq!(
        target,
        vec![
            "Axum".to_string(),
            "Vite".to_string(),
            "Deno".to_string(),
            "Rocket".to_string()
        ]
    );
}

#[test]
fn default_confidence_threshold_is_zero_keep_all_semantics() {
    assert_eq!(DEFAULT_CONFIDENCE_THRESHOLD, 0.0);
}

#[test]
fn merge_tree_keeps_nested_frontend_and_backend_both_visible() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "mono");
    let web = make_dir(&root, "web");
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::write(web.join("package.json"), "{}").unwrap();

    let pairs = vec![
        (root.clone(), vec![hit("Cargo.toml", Some("Rust"))]),
        (web.clone(), vec![hit("package.json", Some("JavaScript"))]),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    // No single winner: the backend root keeps its own detection...
    assert_eq!(tree.detection.ecosystem, "rust");
    assert_eq!(tree.detection.scope, root);
    assert_eq!(tree.detection.markers, vec!["Cargo.toml".to_string()]);
    // ...and the frontend survives as a nested child with its own scope.
    assert_eq!(tree.children.len(), 1);
    let frontend = &tree.children[0];
    assert_eq!(frontend.detection.ecosystem, "node");
    assert_eq!(frontend.detection.scope, web);
    assert_eq!(frontend.detection.markers, vec!["package.json".to_string()]);
    assert!(frontend.children.is_empty());
}

#[test]
fn merge_tree_json_is_byte_identical_across_repeated_builds() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "mono");
    let web = make_dir(&root, "web");
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::write(web.join("package.json"), "{}").unwrap();

    let pairs = vec![
        (root.clone(), vec![hit("Cargo.toml", Some("Rust"))]),
        (web.clone(), vec![hit("package.json", Some("JavaScript"))]),
    ];

    let first = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");
    let second = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    let first_json = serde_json::to_string_pretty(&first).unwrap();
    let second_json = serde_json::to_string_pretty(&second).unwrap();
    assert_eq!(first_json, second_json);

    // Serialized children follow the sorted scope order of the tree itself.
    let scopes: Vec<String> = second
        .children
        .iter()
        .map(|child| child.detection.scope.display().to_string())
        .collect();
    let mut sorted = scopes.clone();
    sorted.sort();
    assert_eq!(scopes, sorted);
}

#[test]
fn merge_tree_children_ordered_lexicographically_with_deepest_nesting() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "ws");
    let services = make_dir(&root, "services");
    let api = make_dir(&services, "api");
    let tools = make_dir(&root, "tools");
    std::fs::write(services.join("go.mod"), "").unwrap();
    std::fs::write(api.join("requirements.txt"), "").unwrap();
    std::fs::write(tools.join("Gemfile"), "").unwrap();

    // Deliberately unordered input: deepest scope first, siblings reversed.
    let pairs = vec![
        (api.clone(), vec![hit("requirements.txt", Some("Python"))]),
        (tools.clone(), vec![hit("Gemfile", Some("Ruby"))]),
        (services.clone(), vec![hit("go.mod", Some("Go"))]),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    // Top-level children are strictly lexicographic by normalized scope.
    let scopes: Vec<PathBuf> = tree
        .children
        .iter()
        .map(|child| child.detection.scope.clone())
        .collect();
    assert_eq!(scopes, vec![services, tools]);

    // The deeper scope nests under its nearest ancestor, not the root.
    assert_eq!(tree.children[0].detection.ecosystem, "go");
    let nested: Vec<PathBuf> = tree.children[0]
        .children
        .iter()
        .map(|child| child.detection.scope.clone())
        .collect();
    assert_eq!(nested, vec![api]);
    assert_eq!(tree.children[0].children[0].detection.ecosystem, "python");
    assert!(tree.children[1].children.is_empty());
}
