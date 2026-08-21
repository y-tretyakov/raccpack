//! Directory scanning primitives: skip policy, traversal, candidates, and size.
//!
//! [`SkipPolicy`] decides which directories a walk never descends into;
//! [`walk_tree`] walks a tree honoring that policy, a max depth, and the
//! invariant that symlinks are never followed. [`ensure_scan_root`] validates
//! the scan root with the domain `Error` type. [`find_candidates`] discovers
//! candidate project roots by matching markers from the [`markers`] registry
//! ([`default_markers()`] plus `CandidateOptions::extra_markers`).
//! [`project_size_bytes`] sums regular-file sizes under a candidate root
//! honoring the same policy.

pub mod candidates;
pub mod markers;
pub mod size;
pub mod skip;
pub mod walk;

pub use candidates::{find_candidates, CandidateOptions, ProjectCandidate};
pub use markers::{default_markers, MarkerDef, MarkerHit, MarkerKind};
pub use size::project_size_bytes;
pub use skip::{SkipPolicy, SkipReason};
pub use walk::{ensure_scan_root, is_path_under_root, walk_tree, WalkOptions};

pub(crate) use walk::canonicalize_existing_prefix;
