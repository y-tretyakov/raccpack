//! Detect languages and frameworks from marker hits and shallow file names.
//!
//! Entry points: [`stack_from_candidate`] (pure, marker-based),
//! [`detect_stack`] (path + markers, enriches frameworks from the top level of
//! the project directory), [`detect_stacks`] (fail-fast batch),
//! [`WorkspaceDetector::detect_tree`] (composite tree over nested scopes),
//! [`candidate_to_project`] (assembly helper) and
//! [`crate::scan::project_size_bytes`].
//!
//! The pipeline itself is selected by [`DetectMode`] (config `detect.mode`):
//! the default [`DetectMode::PriorityTable`] keeps the flat §4.1 behaviour,
//! [`DetectMode::CompositeDag`] fills `Project.stack_tree` through the
//! composite pipeline (experimental; see [`workspace`]).
//!
//! # Merge policy
//!
//! Detectors are organized per ecosystem — one `*.rs` per language — and this
//! module is the only place that enumerates them ([`detector_registry()`]) and
//! merges their contributions into a single [`Stack`]:
//!
//! - **language** — resolved centrally from marker `language_hint`s by the §4.1
//!   priority table (`types::resolve_language`); detector-provided languages
//!   are ignored, so detectors leave `language` unset.
//! - **frameworks** — union of the `frameworks` returned by the applicable
//!   detectors, in registry order, deduplicated (first occurrence wins).
//! - **markers** — the names of every hit, sorted lexically and deduplicated.
//!
//! Conflicts between several opinions — nested scopes, repeated ecosystems at
//! one scope, duplicate scope spellings — resolve by the rules documented in
//! [`merge`]; there is never a single winner for a whole monorepo.
//!
//! A detector applies when it matches the hits (one of its ecosystem markers is
//! present). When `markers` is empty — the path-only [`detect_stack`] case the
//! spec describes as "markers ещё не собраны" — every detector probes the
//! directory so framework files are still found.
//!
//! # Errors
//!
//! [`detect_stack`] / [`detect_stacks`] validate the path first
//! ([`Error::PathNotFound`], [`Error::NotADirectory`]); shallow `read_dir`
//! failures surface as [`Error::Io`]. [`stack_from_candidate`] never touches
//! the filesystem.
//!
//! # Determinism
//!
//! Registry order is fixed, detector framework order is fixed, directory names
//! are sorted before matching and `markers` are sorted before deduplication, so
//! equal inputs always produce equal [`Stack`]s regardless of `read_dir` order.

use std::path::Path;

use crate::domain::{Error, Project, Result, Stack};
use crate::scan::{MarkerHit, ProjectCandidate};

mod cpp;
mod git;
mod go;
mod jvm;
mod make;
pub mod merge;
mod mode;
mod node;
mod php;
mod python;
mod ruby;
mod rust;
mod traits;
mod types;
mod workspace;

pub use mode::DetectMode;
pub use traits::StackDetector;
pub use types::{clamp_confidence, Detection, StackNode};
pub use workspace::WorkspaceDetector;

use cpp::CppDetector;
use git::GitDetector;
use go::GoDetector;
use jvm::JvmDetector;
use make::MakeDetector;
use merge::{extend_frameworks_union, sorted_unique_names};
use node::NodeDetector;
use php::PhpDetector;
use python::PythonDetector;
use ruby::RubyDetector;
use rust::RustDetector;
use types::resolve_language;

/// Stable order registry (ecosystem modules).
///
/// Order is fixed (rust, node, go, python, jvm, ruby, php, cpp, make, git) and
/// mirrors the marker registry order, so the framework union order is
/// deterministic.
pub fn detector_registry() -> &'static [&'static dyn StackDetector] {
    &[
        &RustDetector,
        &NodeDetector,
        &GoDetector,
        &PythonDetector,
        &JvmDetector,
        &RubyDetector,
        &PhpDetector,
        &CppDetector,
        &MakeDetector,
        &GitDetector,
    ]
}

/// Build a [`Stack`] from an already-discovered candidate.
///
/// PURE: never touches the filesystem. Language is resolved from the marker
/// `language_hint`s by the §4.1 priority table, `markers` are the sorted unique
/// hit names, and `frameworks` stay empty — enrichment requires reading the
/// project directory, which only [`detect_stack`] does.
pub fn stack_from_candidate(candidate: &ProjectCandidate) -> Stack {
    let language = resolve_language(&candidate.markers);
    let markers = sorted_unique_marker_names(&candidate.markers);
    Stack {
        language,
        frameworks: Vec::new(),
        markers,
    }
}

/// Detect a [`Stack`] for a project directory given its marker hits.
///
/// `path` must be an existing directory; a missing path maps to
/// [`Error::PathNotFound`] and a non-directory to [`Error::NotADirectory`].
/// Framework hints are collected from the top-level file names (see the module
/// merge policy); with an empty `markers` slice every detector probes the
/// directory. Shallow-read IO failures surface as [`Error::Io`].
pub fn detect_stack(path: &Path, markers: &[MarkerHit]) -> Result<Stack> {
    if !path.exists() {
        return Err(Error::PathNotFound {
            path: path.to_path_buf(),
        });
    }
    if !path.is_dir() {
        return Err(Error::NotADirectory {
            path: path.to_path_buf(),
        });
    }

    let probe_all = markers.is_empty();
    let mut frameworks: Vec<String> = Vec::new();
    for detector in detector_registry() {
        if !probe_all && !detector.matches(markers) {
            continue;
        }
        let contribution = detector.detect(markers, path)?;
        extend_frameworks_union(&mut frameworks, contribution.frameworks);
    }

    let language = resolve_language(markers);
    let markers = sorted_unique_marker_names(markers);
    Ok(Stack {
        language,
        frameworks,
        markers,
    })
}

/// Detect stacks for every candidate.
///
/// Fail-fast: the first failing candidate path aborts the whole batch. Input
/// order is preserved in the returned pairs.
pub fn detect_stacks(candidates: &[ProjectCandidate]) -> Result<Vec<(ProjectCandidate, Stack)>> {
    candidates
        .iter()
        .map(|candidate| {
            let stack = detect_stack(&candidate.path, &candidate.markers)?;
            Ok((candidate.clone(), stack))
        })
        .collect()
}

/// Assemble a [`Project`] from a candidate, its [`Stack`] and its size.
pub fn candidate_to_project(candidate: ProjectCandidate, stack: Stack, size_bytes: u64) -> Project {
    Project {
        path: candidate.path,
        name: candidate.name,
        stack,
        stack_tree: None,
        size_bytes,
        is_git_repo: candidate.is_git_repo,
    }
}

/// Names of every hit, sorted lexically and deduplicated.
fn sorted_unique_marker_names(markers: &[MarkerHit]) -> Vec<String> {
    sorted_unique_names(markers.iter().map(|hit| hit.name.clone()).collect())
}

#[cfg(test)]
mod merge_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_tests;
