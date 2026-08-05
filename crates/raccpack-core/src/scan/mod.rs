//! Directory scanning primitives: skip policy and traversal.
//!
//! [`SkipPolicy`] decides which directories a walk never descends into;
//! [`walk_tree`] walks a tree honoring that policy, a max depth, and the
//! invariant that symlinks are never followed. [`ensure_scan_root`] validates
//! the scan root with the domain `Error` type.

pub mod skip;
pub mod walk;

pub use skip::{SkipPolicy, SkipReason};
pub use walk::{ensure_scan_root, walk_tree, WalkOptions};
