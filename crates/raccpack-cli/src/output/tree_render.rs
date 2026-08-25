//! Deterministic human-readable tree render for composite_dag `StackNode`.

use std::path::Path;

use raccpack_core::StackNode;

/// Render a `StackNode` tree as indented lines with box-drawing connectors.
///
/// The root detection is rendered with a 4-space indent. Children use
/// `├── ` / `└── ` connectors with proper continuation bars for nesting.
/// The scope label (relative to `project_root`) is appended after the
/// detection summary for all nodes.
///
/// Children are assumed pre-sorted by the data layer (normalization_key ASC).
pub(super) fn render_tree(root: &StackNode, project_root: &Path) -> String {
    let summary_width = detection_summary_width(root);
    let mut lines = Vec::new();
    // Root: prefix unused (is_root), continuation = 4-space base indent
    render_node(
        root,
        project_root,
        "",
        "    ",
        true,
        &mut lines,
        summary_width,
    );
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Render a node and its children recursively.
///
/// - `prefix`: indentation for this node's line (includes connector for non-root).
/// - `continuation`: base indentation for children (includes vertical bars).
/// - `is_root`: true for the top-level detection (no connector).
fn render_node(
    node: &StackNode,
    project_root: &Path,
    prefix: &str,
    continuation: &str,
    is_root: bool,
    lines: &mut Vec<String>,
    summary_width: usize,
) {
    let summary = format_detection_summary(&node.detection);
    let label = display_label(&node.detection.scope, project_root);

    if is_root {
        let line = format!("{continuation}{summary:<summary_width$}  {label}");
        lines.push(line);
    } else {
        let line = format!("{prefix}{summary:<summary_width$}  {label}");
        lines.push(line);
    }

    let child_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i + 1 == child_count;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = format!("{continuation}{connector}");
        let child_cont = format!("{}{}", continuation, if is_last { "    " } else { "│   " });
        render_node(
            child,
            project_root,
            &child_prefix,
            &child_cont,
            false,
            lines,
            summary_width,
        );
    }
}

/// Format a detection summary: `Ecosystem · Language · Framework1 · Framework2`.
fn format_detection_summary(det: &raccpack_core::Detection) -> String {
    let mut parts = vec![det.ecosystem.clone()];
    if let Some(ref lang) = det.language {
        parts.push(lang.clone());
    }
    for fw in &det.frameworks {
        parts.push(fw.clone());
    }
    parts.join(" · ")
}

/// Compute the maximum detection summary width across the entire tree.
fn detection_summary_width(node: &StackNode) -> usize {
    let mut max = format_detection_summary(&node.detection).len();
    for child in &node.children {
        max = max.max(detection_summary_width(child));
    }
    max
}

/// Display label for a detection scope, relative to `project_root`.
///
/// For the root node (scope == project_root) returns empty string.
/// For children returns the relative directory name (e.g. `web`, `lib/worker`).
fn display_label(scope: &Path, project_root: &Path) -> String {
    scope
        .strip_prefix(project_root)
        .ok()
        .and_then(|rel| {
            let s = rel.to_string_lossy();
            if s.is_empty() || s == "." {
                None
            } else {
                Some(s.into_owned())
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use raccpack_core::{Detection, StackNode};
    use std::path::PathBuf;

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
            confidence: 0.9,
            scope: PathBuf::from(scope),
            markers: vec!["Cargo.toml".to_string()],
        }
    }

    #[test]
    fn single_root_node_renders_one_line() {
        let root = StackNode {
            detection: detection("rust", Some("Rust"), &["Axum"], "/proj"),
            children: vec![],
        };
        let out = render_tree(&root, Path::new("/proj"));
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("rust"));
        assert!(lines[0].contains("Rust"));
        assert!(lines[0].contains("Axum"));
    }

    #[test]
    fn root_with_children_uses_tree_connectors() {
        let root = StackNode {
            detection: detection("rust", Some("Rust"), &[], "/proj"),
            children: vec![
                StackNode {
                    detection: detection("node", Some("JavaScript"), &[], "/proj/web"),
                    children: vec![],
                },
                StackNode {
                    detection: detection("node", Some("TypeScript"), &["React"], "/proj/app"),
                    children: vec![],
                },
            ],
        };
        let out = render_tree(&root, Path::new("/proj"));
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("rust"));
        assert!(lines[1].contains("├── "));
        assert!(lines[1].contains("web"));
        assert!(lines[2].contains("└── "));
        assert!(lines[2].contains("app"));
    }

    #[test]
    fn nested_children_get_deeper_indent_with_continuation() {
        let root = StackNode {
            detection: detection("rust", Some("Rust"), &[], "/proj"),
            children: vec![
                StackNode {
                    detection: detection("node", Some("TypeScript"), &[], "/proj/web"),
                    children: vec![StackNode {
                        detection: detection("node", Some("JavaScript"), &[], "/proj/web/pkg"),
                        children: vec![],
                    }],
                },
                StackNode {
                    detection: detection("unknown", None, &[], "/proj/lib"),
                    children: vec![],
                },
            ],
        };
        let out = render_tree(&root, Path::new("/proj"));
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("rust"));
        assert!(lines[1].contains("├── "));
        assert!(lines[1].contains("web"));
        assert!(lines[2].contains("│"));
        assert!(lines[2].contains("└── "));
        assert!(lines[2].contains("pkg"));
        assert!(lines[3].contains("└── "));
        assert!(lines[3].contains("lib"));
    }

    #[test]
    fn no_language_shows_ecosystem_only() {
        let root = StackNode {
            detection: detection("unknown", None, &[], "/proj"),
            children: vec![],
        };
        let out = render_tree(&root, Path::new("/proj"));
        assert!(out.contains("unknown"));
        assert!(!out.contains("·"));
    }

    #[test]
    fn display_label_empty_for_root() {
        let label = display_label(Path::new("/proj"), Path::new("/proj"));
        assert_eq!(label, "");
    }

    #[test]
    fn display_label_shows_relative_for_child() {
        let label = display_label(Path::new("/proj/web"), Path::new("/proj"));
        assert_eq!(label, "web");
    }

    #[test]
    fn display_label_shows_nested_relative() {
        let label = display_label(Path::new("/proj/lib/worker"), Path::new("/proj"));
        assert_eq!(label, "lib/worker");
    }

    #[test]
    fn detection_summary_width_is_accurate() {
        let root = StackNode {
            detection: detection("rust", Some("Rust"), &["Axum"], "/proj"),
            children: vec![StackNode {
                detection: detection(
                    "node",
                    Some("TypeScript"),
                    &["React", "Next.js"],
                    "/proj/web",
                ),
                children: vec![],
            }],
        };
        let w = detection_summary_width(&root);
        let child_summary = format_detection_summary(&root.children[0].detection);
        assert_eq!(w, child_summary.len());
    }

    #[test]
    fn tree_output_ends_with_newline() {
        let root = StackNode {
            detection: detection("rust", Some("Rust"), &[], "/proj"),
            children: vec![],
        };
        let out = render_tree(&root, Path::new("/proj"));
        assert!(out.ends_with('\n'));
    }
}
