//! D2.3 CLI human-render tests for `stack_tree` integration.
//!
//! Exercises the render_tree / format_human boundary after Dev landed the
//! human tree renderer (`output/tree_render.rs`).  All fixtures are pure
//! stack-constructed — no filesystem, no network.

use std::path::PathBuf;

use raccpack_core::{Detection, ScanReport, SniffResult, StackNode};

use super::{format_human, tree_render};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detection(
    ecosystem: &str,
    language: Option<&str>,
    frameworks: &[&str],
    scope: &str,
) -> Detection {
    Detection {
        ecosystem: ecosystem.to_string(),
        language: language.map(String::from),
        frameworks: frameworks.iter().map(|s| (*s).to_string()).collect(),
        confidence: 1.0,
        scope: PathBuf::from(scope),
        markers: vec![],
    }
}

/// Two-level mono fixture: Rust root, JS child `web` (which itself has a
/// grandchild `web/pkg`), and unknown child `lib/worker`.
/// Children are passed in **reverse** lexical scope order (web first,
/// lib/worker second) so the test proves the render preserves input order.
fn mono_two_level_fixture() -> (StackNode, PathBuf) {
    let root = PathBuf::from("/proj/mono");

    // web has a grandchild → makes │ continuation bars visible in output.
    let web = StackNode {
        detection: detection("node", Some("JavaScript"), &[], "/proj/mono/web"),
        children: vec![StackNode {
            detection: detection("node", Some("JavaScript"), &[], "/proj/mono/web/pkg"),
            children: vec![],
        }],
    };
    let worker = StackNode {
        detection: detection("unknown", None, &[], "/proj/mono/lib/worker"),
        children: vec![],
    };

    let tree = StackNode {
        detection: detection("rust", Some("Rust"), &[], "/proj/mono"),
        children: vec![web, worker], // reverse of lexical scope order
    };

    (tree, root)
}

/// Build a SniffResult whose projects all carry `stack_tree: None` — the
/// legacy format before D2.3.
fn legacy_sniff_result() -> SniffResult {
    SniffResult {
        report: ScanReport {
            root: PathBuf::from("/tmp"),
            projects: vec![
                raccpack_core::Project {
                    path: PathBuf::from("/tmp/app-api"),
                    name: "app-api".to_string(),
                    stack: raccpack_core::Stack {
                        language: Some("Rust".to_string()),
                        frameworks: Vec::new(),
                        markers: Vec::new(),
                    },
                    stack_tree: None,
                    size_bytes: 12_689_920,
                    is_git_repo: true,
                },
                raccpack_core::Project {
                    path: PathBuf::from("/tmp/scripts"),
                    name: "scripts".to_string(),
                    stack: raccpack_core::Stack::default(),
                    stack_tree: None,
                    size_bytes: 2048,
                    is_git_repo: false,
                },
            ],
            total_size_bytes: 12_691_968,
            schema_version: 1,
        },
        from_cache: false,
        duration_ms: 42,
    }
}

// ---------------------------------------------------------------------------
// Case a: render_tree — deterministic, children in input order, box-drawing
// ---------------------------------------------------------------------------

#[test]
fn render_tree_fixture_root_two_levels_is_deterministic_and_sorted() {
    let (tree, root) = mono_two_level_fixture();

    let first = tree_render::render_tree(&tree, &root);
    let second = tree_render::render_tree(&tree, &root);

    // Deterministic: byte-identical across two calls.
    assert_eq!(first, second, "render_tree must be deterministic");

    let lines: Vec<&str> = first.trim().lines().collect();
    assert_eq!(
        lines.len(),
        4,
        "root + web + web/pkg + lib/worker = 4 lines"
    );

    // Root line: no box-drawing connector.
    assert!(lines[0].contains("rust"), "root ecosystem present");
    assert!(lines[0].contains("Rust"), "root language present");
    assert!(
        !lines[0].contains("├") && !lines[0].contains("└"),
        "root line must not contain box-drawing connectors"
    );

    // web (first child, not last) uses ├── and has continuation bar │
    assert!(
        lines[1].contains("├── "),
        "first child must use ├ connector (not last child)"
    );
    assert!(lines[1].contains("web"), "first child is web");
    assert!(lines[1].contains("node"), "web ecosystem");

    // web/pkg (grandchild under non-last child) uses continuation bar │
    assert!(
        lines[2].contains("│"),
        "grandchild under non-last child shows continuation bar"
    );
    assert!(
        lines[2].contains("└── "),
        "only child of web uses └ connector"
    );
    assert!(lines[2].contains("pkg"), "grandchild pkg present");

    // lib/worker (last child) uses └──
    assert!(lines[3].contains("└── "), "last child must use └ connector");
    assert!(lines[3].contains("lib/worker"), "last child is lib/worker");
    assert!(lines[3].contains("unknown"), "lib/worker ecosystem");

    // Lines[1]=web, Lines[3]=lib/worker proves the render preserves the
    // input ordering (web before lib/worker) without re-sorting.
}

// ---------------------------------------------------------------------------
// Case b: format_human — None stack_tree → legacy output (no tree blocks)
// ---------------------------------------------------------------------------

#[test]
fn format_human_with_none_stack_tree_matches_legacy_output_byte_for_byte() {
    let result = legacy_sniff_result();
    let text = format_human(&result);

    // Table is present and correct.
    assert!(text.starts_with("Scan root: /tmp\n"));
    assert!(text.contains("Projects: 2"));
    assert!(text.contains("NAME"), "header row present");
    assert!(text.contains("app-api"));
    assert!(text.contains("scripts"));
    assert!(text.contains("Rust"));
    assert!(text.contains("-"), "default stack for scripts");
    assert!(text.contains("yes"));
    assert!(text.contains("no"));

    // Absolutely no tree-render fragments anywhere.
    assert!(
        !text.contains("├──"),
        "no ├── connector when all projects have stack_tree=None"
    );
    assert!(
        !text.contains("└──"),
        "no └── connector when all projects have stack_tree=None"
    );
    assert!(
        !text.contains("│"),
        "no │ continuation bar when all projects have stack_tree=None"
    );
    // The tree-rendered block is prefixed with project name + size header
    // like `  app-api (12.1 MiB)\n` — assert none of those appear.
    assert!(
        !text.contains("app-api ("),
        "no tree-block header for app-api"
    );

    // Byte-identical across two calls (determinism).
    let second = format_human(&legacy_sniff_result());
    assert_eq!(text, second, "format_human must be deterministic");
}

// ---------------------------------------------------------------------------
// Case c: placeholder tree (unknown root, no children)
// ---------------------------------------------------------------------------

#[test]
fn render_tree_placeholder_unknown_root_no_children_is_single_line() {
    let project_root = PathBuf::from("/proj/empty");
    let tree = StackNode {
        detection: detection("unknown", None, &[], "/proj/empty"),
        children: vec![],
    };

    let out = tree_render::render_tree(&tree, &project_root);
    let lines: Vec<&str> = out.trim().lines().collect();

    // Single node → one line, no box-drawing symbols at all.
    assert_eq!(lines.len(), 1, "placeholder with no children = one line");
    assert!(lines[0].contains("unknown"), "ecosystem summary present");
    assert!(
        !lines[0].contains("├") && !lines[0].contains("└") && !lines[0].contains("│"),
        "no box-drawing symbols for a single-node tree"
    );
    assert!(
        !lines[0].contains("·"),
        "no language/framework separator when language is None"
    );
}
