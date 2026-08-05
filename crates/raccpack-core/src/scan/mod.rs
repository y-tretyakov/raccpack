//! Directory scanning primitives: skip policy, traversal, and candidates.
//!
//! [`SkipPolicy`] decides which directories a walk never descends into;
//! [`walk_tree`] walks a tree honoring that policy, a max depth, and the
//! invariant that symlinks are never followed. [`ensure_scan_root`] validates
//! the scan root with the domain `Error` type. [`find_candidates`] discovers
//! candidate project roots by matching [`MarkerDef`] markers.

pub mod candidates;
pub mod markers;
pub mod skip;
pub mod walk;

pub use candidates::{find_candidates, CandidateOptions, ProjectCandidate};
pub use markers::{MarkerDef, MarkerHit, MarkerKind, DEFAULT_MARKERS};
pub use skip::{SkipPolicy, SkipReason};
pub use walk::{ensure_scan_root, walk_tree, WalkOptions};
