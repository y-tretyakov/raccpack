//! Unit tests for the D2.1 [`WorkspaceDetector`] composite tree pipeline.
//!
//! Fixtures build real directories under a `tempfile::TempDir`: scope paths
//! must exist because containment validation canonicalizes them and detectors
//! probe the scope directory itself.

use std::path::{Path, PathBuf};

use super::tests::hit;
use super::*;
use crate::scan::MarkerKind;

/// Create an existing directory under `base` and return its path.
fn make_dir(base: &Path, name: &str) -> PathBuf {
    let path = base.join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn workspace_tree_covers_both_ecosystems_in_monorepo() {
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

    assert_eq!(tree.detection.ecosystem, "rust");
    assert_eq!(tree.detection.language.as_deref(), Some("Rust"));
    assert_eq!(tree.detection.confidence, 1.0);
    assert_eq!(tree.detection.scope, root);
    assert_eq!(tree.detection.markers, vec!["Cargo.toml".to_string()]);
    assert_eq!(tree.children.len(), 1);
    let web_node = &tree.children[0];
    assert_eq!(web_node.detection.ecosystem, "node");
    assert_eq!(web_node.detection.language.as_deref(), Some("JavaScript"));
    assert_eq!(web_node.detection.scope, web);
    assert!(web_node.children.is_empty());
}

#[test]
fn workspace_single_project_yields_childless_root_node() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "solo");
    std::fs::write(root.join("Cargo.toml"), "").unwrap();

    let pairs = vec![(root.clone(), vec![hit("Cargo.toml", Some("Rust"))])];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    assert_eq!(tree.detection.ecosystem, "rust");
    assert!(tree.children.is_empty());
}

#[test]
fn workspace_root_without_hits_is_unknown_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "plain");
    let svc = make_dir(&root, "svc");
    std::fs::write(svc.join("go.mod"), "").unwrap();

    // Only the nested scope arrives; the project root itself has no entry.
    let pairs = vec![(svc.clone(), vec![hit("go.mod", Some("Go"))])];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    assert_eq!(tree.detection.ecosystem, "unknown");
    assert_eq!(tree.detection.language, None);
    assert_eq!(tree.detection.confidence, 0.0);
    assert!(tree.detection.frameworks.is_empty());
    assert!(tree.detection.markers.is_empty());
    assert_eq!(tree.detection.scope, root);
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].detection.ecosystem, "go");
}

#[test]
fn workspace_scope_outside_project_root_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "proj");
    let outsider = make_dir(dir.path(), "other");

    let pairs = vec![(outsider, vec![hit("Cargo.toml", Some("Rust"))])];
    let err = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect_err("scope outside the root must fail");

    match err {
        Error::Other { message } => {
            assert!(message.contains("outside the project root"), "{message}");
        }
        other => panic!("expected Error::Other, got {other:?}"),
    }
}

#[test]
fn workspace_validates_project_root_existence_and_kind() {
    let detector = WorkspaceDetector::new();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    assert!(matches!(
        detector.detect_tree(&missing, &[]),
        Err(Error::PathNotFound { .. })
    ));

    let file = dir.path().join("a.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(matches!(
        detector.detect_tree(&file, &[]),
        Err(Error::NotADirectory { .. })
    ));
}

#[test]
fn workspace_children_are_sorted_by_scope_regardless_of_input_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "ws");
    let zeta = make_dir(&root, "zeta");
    let alpha = make_dir(&root, "alpha");
    std::fs::write(zeta.join("go.mod"), "").unwrap();
    std::fs::write(alpha.join("Cargo.toml"), "").unwrap();

    // Deliberately unordered: zeta before alpha before the root entry.
    let pairs = vec![
        (zeta.clone(), vec![hit("go.mod", Some("Go"))]),
        (alpha.clone(), vec![hit("Cargo.toml", Some("Rust"))]),
        (
            root.clone(),
            vec![MarkerHit {
                name: ".git".to_string(),
                kind: MarkerKind::DirName,
                language_hint: None,
            }],
        ),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    let scopes: Vec<PathBuf> = tree
        .children
        .iter()
        .map(|child| child.detection.scope.clone())
        .collect();
    assert_eq!(scopes, vec![alpha, zeta]);
}

#[test]
fn workspace_nested_scope_attaches_to_nearest_ancestor_not_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "ws");
    let mid = make_dir(&root, "services");
    let leaf = make_dir(&mid, "api");
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::write(mid.join("go.mod"), "").unwrap();
    std::fs::write(leaf.join("requirements.txt"), "").unwrap();

    let pairs = vec![
        (leaf.clone(), vec![hit("requirements.txt", Some("Python"))]),
        (mid.clone(), vec![hit("go.mod", Some("Go"))]),
        (root.clone(), vec![hit("Cargo.toml", Some("Rust"))]),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    assert_eq!(tree.detection.ecosystem, "rust");
    assert_eq!(tree.children.len(), 1);
    let services = &tree.children[0];
    assert_eq!(services.detection.scope, mid);
    assert_eq!(services.detection.ecosystem, "go");
    assert_eq!(services.children.len(), 1);
    assert_eq!(services.children[0].detection.scope, leaf);
    assert_eq!(services.children[0].detection.ecosystem, "python");
}

#[test]
fn workspace_scope_identity_ignores_dot_segments_and_trailing_separator() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "ws");
    let web = make_dir(&root, "web");
    std::fs::write(web.join("package.json"), "{}").unwrap();

    // Same directory spelled with a redundant `.` segment; plus a duplicate of
    // the plain form — deduplication keeps a single node either way.
    let dotted = root.join("./web");
    let plain = root.join("web").join("");
    let pairs = vec![
        (
            dotted.clone(),
            vec![hit("package.json", Some("JavaScript"))],
        ),
        (plain, vec![hit("package.json", Some("JavaScript"))]),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].detection.scope, dotted);
}

#[test]
fn workspace_tree_serde_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_dir(dir.path(), "mono");
    let web = make_dir(&root, "web");
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::write(web.join("package.json"), "{}").unwrap();

    let pairs = vec![
        (root.clone(), vec![hit("Cargo.toml", Some("Rust"))]),
        (web, vec![hit("package.json", Some("JavaScript"))]),
    ];
    let tree = WorkspaceDetector::new()
        .detect_tree(&root, &pairs)
        .expect("tree ok");

    let json = serde_json::to_string(&tree).expect("serialize");
    let back: StackNode = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(tree, back);
}
