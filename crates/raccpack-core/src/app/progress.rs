use serde::{Deserialize, Serialize};

/// Logical operation a [`ProgressEvent`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    /// Workspace sniffing: discover projects under `scan_root`.
    Sniff,
    /// Secret dig: locate and classify sensitive values.
    Dig,
    /// Secret stash: encrypt secrets into the den.
    Stash,
    /// Cleanup of build artifacts.
    Rinse,
    /// Packaging a project into `tar.zst`.
    Pack,
    /// Orchestrated end-to-end run.
    Raid,
}

/// One progress notification emitted by a facade use-case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// The operation this event belongs to.
    pub operation: OperationKind,
    /// Phase name within the operation (e.g. `"scan"`).
    pub phase: String,
    /// Zero-based index of the current phase within the operation.
    pub phase_index: u32,
    /// Total number of phases in the operation.
    pub phase_count: u32,
    /// Progress within the current phase, 0–100.
    pub percent: u8,
    /// Overall progress of the operation, 0–100.
    pub overall_percent: u8,
    /// Human-readable status message.
    pub message: String,
    /// Whether the current phase has completed.
    pub phase_complete: bool,
}

/// Consumer of [`ProgressEvent`]s.
///
/// Implementors must be `Send` so use-cases can emit progress from worker
/// threads. Rendering (spinner, bars, logs) is the caller's responsibility.
pub trait ProgressSink: Send {
    /// Deliver one progress event.
    fn emit(&mut self, event: ProgressEvent);
}

/// Progress sink that discards every event.
///
/// Useful for tests, batch runs, and any caller that does not care about
/// progress reporting.
#[derive(Debug, Default)]
pub struct NullProgress;

impl ProgressSink for NullProgress {
    fn emit(&mut self, _event: ProgressEvent) {}
}
