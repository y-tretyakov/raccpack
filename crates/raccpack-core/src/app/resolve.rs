//! Composite DAG stack-tree resolution for rinse/raid.
//!
//! [`resolve_stack_tree`] runs a targeted `sniff` on the parent directory of
//! the target project to discover its composite DAG tree, which rinse then
//! uses for scoped trash discovery. This is the E2E wiring that makes
//! DAG-rinse work from CLI: without it, callers always pass
//! `stack_tree: None` and rinse falls back to flat walk.

use crate::detect::{DetectMode, StackNode};
use crate::domain::ScanReport;

use super::context::AppContext;

/// Resolve the composite DAG `stack_tree` for `target` via a targeted sniff.
///
/// When `ctx.config.detect.mode` is `CompositeDag`, runs `sniff` on the
/// **parent** of `target` (the scan root that contains the project), finds
/// the project whose path matches `target`, and returns its `stack_tree`.
/// Returns `None` when:
/// - detect mode is not `CompositeDag`
/// - sniff finds no project matching `target`
/// - any IO/cache error occurs (treated as cache miss, not a hard failure)
pub fn resolve_stack_tree(
    ctx: &AppContext,
    target: &std::path::Path,
    sniff_opts: &super::sniff::SniffOptions,
) -> Option<StackNode> {
    use super::context::{RunMode, WorkspacePaths};
    use super::sniff::sniff;

    if ctx.config.detect.mode != DetectMode::CompositeDag {
        return None;
    }

    // sniff needs scan_root = parent of target (the directory containing the project).
    let scan_root = target.parent()?;

    let sniff_ctx = AppContext {
        config: ctx.config.clone(),
        paths: WorkspacePaths {
            scan_root: scan_root.to_path_buf(),
            den_dir: ctx.paths.den_dir.clone(),
        },
        mode: RunMode::DryRun,
        exit_policy: ctx.exit_policy,
    };

    let mut progress = super::progress::NullProgress;
    let result = sniff(&sniff_ctx, sniff_opts, &mut progress).ok()?;

    resolve_stack_tree_from_result(ctx.config.detect.mode, &result.report, target)
}

/// Resolve the composite stack tree from a pre-computed [`ScanReport`].
///
/// Internal helper shared by [`resolve_stack_tree`] (which runs sniff) and
/// callers that already have a report (e.g. unit tests, atomic raid).
pub(crate) fn resolve_stack_tree_from_result(
    detect_mode: DetectMode,
    report: &ScanReport,
    target: &std::path::Path,
) -> Option<StackNode> {
    if detect_mode != DetectMode::CompositeDag {
        return None;
    }

    let target = target.canonicalize().ok()?;

    // The scan root itself (no parent) has no scoped tree.
    target.parent()?;

    report
        .projects
        .iter()
        .find(|p| target.starts_with(&p.path) && p.stack_tree.is_some())
        .and_then(|p| p.stack_tree.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::Detection;
    use crate::domain::{Project, ScanReport, Stack};
    use std::path::{Path, PathBuf};

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

    fn leaf_node(ecosystem: &str, scope: &Path) -> StackNode {
        StackNode {
            detection: detection(ecosystem, scope),
            children: Vec::new(),
        }
    }

    fn project_with(path: &Path, stack_tree: Option<StackNode>) -> Project {
        Project {
            path: path.to_path_buf(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            stack: Stack::default(),
            stack_tree,
            size_bytes: 0,
            is_git_repo: false,
        }
    }

    #[test]
    fn composite_dag_mode_and_match() {
        let temp = tempfile::tempdir().unwrap();
        let proj = temp.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        let tree = leaf_node("rust", &proj);
        let report = ScanReport {
            root: temp.path().to_path_buf(),
            projects: vec![project_with(&proj, Some(tree.clone()))],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &proj);

        let returned = result.expect("must return Some(tree)");
        assert_eq!(returned.detection.ecosystem, "rust");
    }

    #[test]
    fn priority_table_mode_returns_none() {
        let proj = PathBuf::from("/tmp/test/scan/proj");
        let tree = leaf_node("rust", &proj);
        let report = ScanReport {
            root: PathBuf::from("/tmp/test/scan"),
            projects: vec![project_with(&proj, Some(tree))],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::PriorityTable, &report, &proj);

        assert!(result.is_none(), "PriorityTable must return None");
    }

    #[test]
    fn no_matching_project_returns_none() {
        let proj = PathBuf::from("/tmp/test/scan/proj");
        let other = PathBuf::from("/tmp/test/scan/other");
        let tree = leaf_node("rust", &proj);
        let report = ScanReport {
            root: PathBuf::from("/tmp/test/scan"),
            projects: vec![project_with(&proj, Some(tree))],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &other);

        assert!(result.is_none(), "non-matching target must return None");
    }

    #[test]
    fn parent_is_none_returns_none() {
        let root = PathBuf::from("/");
        let report = ScanReport {
            root: root.clone(),
            projects: vec![project_with(&root, Some(leaf_node("rust", &root)))],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &root);

        assert!(result.is_none(), "root path (no parent) must return None");
    }

    #[test]
    fn sniff_error_returns_none() {
        let target = PathBuf::from("/nonexistent/path");
        let report = ScanReport::default();

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &target);

        assert!(
            result.is_none(),
            "empty report must return None (canonicalize fails on nonexistent path)"
        );
    }

    #[test]
    fn project_without_stack_tree_returns_none() {
        let proj = PathBuf::from("/tmp/test/scan/proj");
        let report = ScanReport {
            root: PathBuf::from("/tmp/test/scan"),
            projects: vec![project_with(&proj, None)],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &proj);

        assert!(
            result.is_none(),
            "project without stack_tree must return None"
        );
    }

    #[test]
    fn nested_target_matches_parent_project() {
        let temp = tempfile::tempdir().unwrap();
        let proj = temp.path().join("proj");
        let nested = proj.join("web");
        std::fs::create_dir_all(&nested).unwrap();
        let tree = StackNode {
            detection: detection("rust", &proj),
            children: vec![leaf_node("node", &nested)],
        };
        let report = ScanReport {
            root: temp.path().to_path_buf(),
            projects: vec![project_with(&proj, Some(tree.clone()))],
            total_size_bytes: 0,
            schema_version: 1,
        };

        let result = resolve_stack_tree_from_result(DetectMode::CompositeDag, &report, &nested);

        let returned = result.expect("nested target must match parent project");
        assert_eq!(returned.detection.ecosystem, "rust");
        assert_eq!(returned.children.len(), 1);
        assert_eq!(returned.children[0].detection.ecosystem, "node");
    }
}
