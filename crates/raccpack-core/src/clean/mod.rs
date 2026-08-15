//! Cleanup (rinse) support: named trash-dir strategies and discovery (A2.1).
//!
//! [`find_trash_dirs`] reports trash-directory candidates under a project
//! root; nothing is deleted at this stage (deletion arrives in A2.2).

pub mod detect;
pub mod strategy;

pub use detect::{find_trash_dirs, DetectTrashOptions, TrashDir};
pub use strategy::{StrategyDef, StrategyId, TrashMatchKind, TrashPattern, DEFAULT_STRATEGIES};
