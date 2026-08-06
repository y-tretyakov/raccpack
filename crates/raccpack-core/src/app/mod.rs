//! Facade use-cases and application context for raccpack-core.
//!
//! [`AppContext`] holds the shared workspace context; [`sniff`] is the first
//! public use-case (project discovery over `scan_root`). Progress reporting is
//! provided by [`ProgressSink`] / [`ProgressEvent`].

mod context;
mod progress;
mod sniff;

pub use context::{AppContext, RunMode, SecretExitPolicy, WorkspacePaths};
pub use progress::{NullProgress, OperationKind, ProgressEvent, ProgressSink};
pub use sniff::{sniff, SniffOptions, SniffResult};
