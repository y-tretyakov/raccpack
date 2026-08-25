//! Conflict/merge policy for detection opinions (stage D2.2,
//! `docs/detect/d2/d2.2-conflict-merge.md`).
//!
//! The composite pipeline can hold several independent opinions about one
//! project: nested scopes of different ecosystems, several detectors matching
//! the same scope, duplicate scope spellings in the input. This module fixes
//! how such conflicts combine. There is **no "one winner" for a whole
//! monorepo** — every scope keeps its own opinion, and conflicts are merged
//! locally by these rules:
//!
//! 1. **Scope nesting** — a deeper scope becomes a child of its nearest
//!    containing ancestor ([`attach_draft`] / [`normalization_key`]); the
//!    rinse target of a node is its [`Detection::scope`] path.
//! 2. **Confidence** — low-confidence opinions may be filtered by a threshold.
//!    [`DEFAULT_CONFIDENCE_THRESHOLD`] is `0.0`, i.e. keep everything;
//!    actual filtering stays optional until a consumer needs it.
//! 3. **Frameworks** — union over all contributing opinions, deduplicated
//!    with stable order by registry (first occurrence wins):
//!    [`extend_frameworks_union`].
//! 4. **Same ecosystem twice at one scope** — [`merge_same_scope`] unions the
//!    frameworks, takes the maximum confidence, unions marker names and keeps
//!    the base language when present. The same rule applies at pipeline input:
//!    duplicate scope entries contribute their marker hits to one scope
//!    ([`union_hits_keep_first`]).
//! 5. **Language at root** — for the flat [`crate::domain::Stack`] summary the language is
//!    still resolved by the §4.1 priority table over root-level detections
//!    (`types::resolve_language`), preserving MVP compatibility; the tree
//!    itself keeps every scope's own resolved language.

use std::ffi::OsString;
use std::path::{Component, Path};

use crate::scan::MarkerHit;

use super::types::{clamp_confidence, Detection, StackNode};

/// Confidence floor below which opinions may be dropped.
///
/// The default `0.0` means **keep all** opinions; threshold filtering is an
/// optional later refinement (rule 2) and deliberately has no CLI/config
/// plumbing yet.
pub const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.0;

/// Merge two [`Detection`] opinions about the same scope (rule 4).
///
/// Frameworks are the union with first-occurrence order preserved (base
/// first), confidence is the clamped maximum of both sides, marker names are
/// the sorted unique union, and the base language wins unless absent. Base
/// also provides `ecosystem` and `scope`; callers must only pass detections
/// for the *same* scope.
pub fn merge_same_scope(base: Detection, extra: Detection) -> Detection {
    let mut frameworks = base.frameworks;
    extend_frameworks_union(&mut frameworks, extra.frameworks);
    Detection {
        ecosystem: base.ecosystem,
        language: base.language.or(extra.language),
        frameworks,
        confidence: clamp_confidence(base.confidence.max(extra.confidence)),
        scope: base.scope,
        markers: merged_marker_names(base.markers, extra.markers),
    }
}

/// Extend `target` with framework names, skipping duplicates (rule 3).
///
/// First occurrence wins, so feeding contributions in registry order keeps
/// the union deterministically ordered. Shared by the flat
/// [`super::detect_stack`] pipeline and the per-scope composite detection.
pub(super) fn extend_frameworks_union(
    target: &mut Vec<String>,
    additions: impl IntoIterator<Item = String>,
) {
    for framework in additions {
        if !target.contains(&framework) {
            target.push(framework);
        }
    }
}

/// Union marker hits into `hits` by name, keeping the first occurrence whole
/// (rule 4 at input level): a later hit with the same name never overrides
/// the earlier name/kind/hint.
pub(super) fn union_hits_keep_first(hits: &mut Vec<MarkerHit>, additions: &[MarkerHit]) {
    for hit in additions {
        if !hits.iter().any(|existing| existing.name == hit.name) {
            hits.push(hit.clone());
        }
    }
}

/// Sorted lexically, deduplicated union of two marker-name lists.
fn merged_marker_names(base: Vec<String>, extra: Vec<String>) -> Vec<String> {
    let mut names = base;
    names.extend(extra);
    sorted_unique_names(names)
}

/// Sort `names` lexically and remove duplicates.
///
/// Single source of truth for the sorted-unique convention shared by the flat
/// stack summary and the merge policy.
pub(super) fn sorted_unique_names(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

/// Insert `node` below `parent` at its nearest containing ancestor.
///
/// The deepest existing child whose normalized key strictly prefixes `key`
/// receives it recursively; with no such child the node is inserted as a
/// direct child at its sorted position (children stay sorted ascending).
pub(super) fn attach_draft(parent: &mut StackNode, key: &[OsString], node: StackNode) {
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

/// Normalized comparison key: components with interior `.` removed.
///
/// `components()` already drops trailing slashes; filtering `CurDir` also
/// collapses a leading `./`. No symlink resolution happens here.
pub(super) fn normalization_key(path: &Path) -> Vec<OsString> {
    path.components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| component.as_os_str().to_os_string())
        .collect()
}
