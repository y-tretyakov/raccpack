//! Cleanup (rinse) support: named trash-dir strategies, discovery, and removal
//! (A2.1 / A2.2).
//!
//! [`find_trash_dirs`] reports trash-directory candidates under a project
//! root; [`remove_trash_dir`] deletes a single such directory tree after
//! recomputing its size (path containment is enforced by the caller).

pub mod detect;
pub mod remove;
pub mod strategy;

pub use detect::{
    find_trash_dirs, find_trash_dirs_scoped, DetectTrashOptions, ScopeEntry, TrashDir,
};
pub use remove::remove_trash_dir;
pub use strategy::{StrategyDef, StrategyId, TrashMatchKind, TrashPattern, DEFAULT_STRATEGIES};
