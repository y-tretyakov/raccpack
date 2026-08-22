//! Composite workspace detection: [`WorkspaceDetector`] builds the recursive
//! [`StackNode`] tree for one project out of pre-collected marker hits.
//!
//! # Data flow
//!
//! [`WorkspaceDetector::detect_tree`] does **not** walk the filesystem itself:
//! every scope arrives through the `markers_by_path` parameter (typically the
//! output of [`crate::scan::find_candidates`] mapped to `(path, markers)`
//! pairs). The only filesystem reads happen inside individual detectors'
//! shallow probes (`read_dir_names`, one `config/` peek), never below a scope.
//!
//! # Scope identity and ordering
//!
//! Paths are compared through their components (`components()`) with interior
//! `.` removed, so `./web/`, `web/` and `web` are one scope. Symlinks are not
//! canonicalized for these comparisons. Children are kept sorted by this
//! normalized key ascending, so equal inputs always produce equal trees.
//!
//! # Multi-ecosystem scopes (current limitation)
//!
//! Several ecosystems matching on one scope collapse into a **single** node:
//! primary ecosystem = first applicable detector in registry order, frameworks
//! = union over all applicable detectors (same merge policy as
//! [`super::detect_stack`]). Full opinion merging is stage D2.2
//! (`docs/detect/d2/d2.2-conflict-merge.md`); confidence semantics are refined
//! there too.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use crate::domain::{Error, Result};
use crate::scan::{ensure_scan_root, is_path_under_root, MarkerHit};

use super::types::{clamp_confidence, resolve_language, Detection, StackNode};
use super::{detector_registry, sorted_unique_marker_names, StackDetector};

/// Ecosystem assigned to a scope whose hits match no registered detector.
const UNKNOWN_ECOSYSTEM: &str = "unknown";

/// Composite stack detector: one [`StackNode`] per scope, linked into a tree
/// rooted at the project directory.
pub struct WorkspaceDetector {
    registry: &'static [&'static dyn StackDetector],
}

impl Default for WorkspaceDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// One detection scope: a directory with non-empty marker hits plus its
/// normalized comparison key.
struct Scope {
    key: Vec<OsString>,
    path: PathBuf,
    hits: Vec<MarkerHit>,
}

impl WorkspaceDetector {
    /// Detector over the default [`detector_registry()`].
    pub fn new() -> Self {
        Self {
            registry: detector_registry(),
        }
    }

    /// Build the composite stack tree rooted at `project_root`.
    ///
    /// Every entry of `markers_by_path` with non-empty markers becomes one
    /// node; each must equal `project_root` or lie under it ([`Error::Other`]
    /// otherwise). Non-root scopes attach to the nearest strictly containing
    /// other scope, or directly to the root when no such scope exists.
    pub fn detect_tree(
        &self,
        project_root: &Path,
        markers_by_path: &[(PathBuf, Vec<MarkerHit>)],
    ) -> Result<StackNode> {
        ensure_scan_root(project_root)?;

        let root_key = normalization_key(project_root);
        let scopes = collect_scopes(project_root, markers_by_path)?;

        let mut drafts: Vec<(Vec<OsString>, StackNode)> = Vec::with_capacity(scopes.len());
        let mut root_node: Option<StackNode> = None;
        for scope in &scopes {
            let node = StackNode {
                detection: self.detect_scope(&scope.path, &scope.hits)?,
                children: Vec::new(),
            };
            if scope.key == root_key {
                root_node.get_or_insert(node);
            } else {
                drafts.push((scope.key.clone(), node));
            }
        }

        // Ascending key order makes insertion order irrelevant: parents sort
        // before descendants, children end up sorted at every level.
        drafts.sort_by(|a, b| a.0.cmp(&b.0));

        let mut root = root_node.unwrap_or_else(|| placeholder_stack_node(project_root));
        for (key, node) in drafts {
            attach_draft(&mut root, &key, node);
        }
        Ok(root)
    }

    /// Detect one scope into a flat [`Detection`].
    ///
    /// The ecosystem is the first applicable detector's id in registry order;
    /// frameworks are the deduplicated union over all applicable detectors
    /// (detector errors propagate). With no applicable detector the ecosystem
    /// is `"unknown"` and confidence is `0.0`.
    fn detect_scope(&self, scope_dir: &Path, hits: &[MarkerHit]) -> Result<Detection> {
        let applicable: Vec<&'static dyn StackDetector> = self
            .registry
            .iter()
            .copied()
            .filter(|detector| detector.matches(hits))
            .collect();

        let mut frameworks: Vec<String> = Vec::new();
        for detector in &applicable {
            let contribution = detector.detect(hits, scope_dir)?;
            for framework in contribution.frameworks {
                if !frameworks.contains(&framework) {
                    frameworks.push(framework);
                }
            }
        }

        Ok(Detection {
            ecosystem: applicable
                .first()
                .map_or(UNKNOWN_ECOSYSTEM, |detector| detector.id())
                .to_string(),
            language: resolve_language(hits),
            frameworks,
            confidence: clamp_confidence(if applicable.is_empty() { 0.0 } else { 1.0 }),
            scope: scope_dir.to_path_buf(),
            markers: sorted_unique_marker_names(hits),
        })
    }
}

/// Collect validated, deduplicated scopes from the raw `(path, markers)` input.
///
/// Entries without hits are ignored; duplicates by normalized path keep the
/// first occurrence. Each remaining scope must equal `project_root` or lie
/// under it.
fn collect_scopes(
    project_root: &Path,
    markers_by_path: &[(PathBuf, Vec<MarkerHit>)],
) -> Result<Vec<Scope>> {
    let mut scopes: Vec<Scope> = Vec::new();
    for (path, hits) in markers_by_path {
        if hits.is_empty() {
            continue;
        }
        if !is_path_under_root(path, project_root)? {
            return Err(Error::Other {
                message: format!(
                    "detection scope {} is outside the project root {}",
                    path.display(),
                    project_root.display()
                ),
            });
        }
        let key = normalization_key(path);
        if scopes.iter().any(|scope| scope.key == key) {
            continue;
        }
        scopes.push(Scope {
            key,
            path: path.clone(),
            hits: hits.clone(),
        });
    }
    Ok(scopes)
}

/// Insert `node` below `parent` at its nearest containing ancestor.
///
/// The deepest existing child whose normalized key strictly prefixes `key`
/// receives it recursively; with no such child the node is inserted as a
/// direct child at its sorted position (children stay sorted ascending).
fn attach_draft(parent: &mut StackNode, key: &[OsString], node: StackNode) {
    let mut target: Option<usize> = None;
    let mut target_len = 0;
    for (index, child) in parent.children.iter().enumerate() {
        let child_key = normalization_key(&child.detection.scope);
        if child_key.len() < key.len()
            && key.starts_with(&child_key)
            && child_key.len() > target_len
        {
            target = Some(index);
            target_len = child_key.len();
        }
    }

    match target {
        Some(index) => attach_draft(&mut parent.children[index], key, node),
        None => {
            let position = parent.children.partition_point(|child| {
                normalization_key(&child.detection.scope).as_slice() < key
            });
            parent.children.insert(position, node);
        }
    }
}

/// Placeholder detection for a tree root absent from the input.
fn placeholder_stack_node(project_root: &Path) -> StackNode {
    StackNode {
        detection: Detection {
            ecosystem: UNKNOWN_ECOSYSTEM.to_string(),
            language: None,
            frameworks: Vec::new(),
            confidence: 0.0,
            scope: project_root.to_path_buf(),
            markers: Vec::new(),
        },
        children: Vec::new(),
    }
}

/// Normalized comparison key: components with interior `.` removed.
///
/// `components()` already drops trailing slashes; filtering `CurDir` also
/// collapses a leading `./`. No symlink resolution happens here.
fn normalization_key(path: &Path) -> Vec<OsString> {
    path.components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| component.as_os_str().to_os_string())
        .collect()
}
