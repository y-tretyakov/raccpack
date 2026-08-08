//! Facade use-cases and application context for raccpack-core.
//!
//! [`AppContext`] holds the shared workspace context; [`sniff`] is the first
//! public use-case (project discovery over `scan_root`). Progress reporting is
//! provided by [`ProgressSink`] / [`ProgressEvent`].

mod context;
mod dig;
mod progress;
mod sniff;

pub use context::{AppContext, RunMode, SecretExitPolicy, WorkspacePaths};
pub use dig::{dig, exit_code_for_secrets, DigOptions, DigResult, RepeatedSecret, SensitiveFile};
pub use progress::{NullProgress, OperationKind, ProgressEvent, ProgressSink};
pub use sniff::{sniff, SniffOptions, SniffResult};
