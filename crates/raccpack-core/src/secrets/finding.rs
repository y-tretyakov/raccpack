//! Finding types produced by secret scans.
//!
//! [`SensitiveFinding`] is the internal representation of a discovered
//! sensitive file: its path, computed [`SensitiveRisk`], the source that
//! triggered it, and a human label. It carries no secret values — masking and
//! hashing happen at later stages.

use std::path::PathBuf;

use crate::domain::SensitiveRisk;

/// Where a [`SensitiveFinding`] came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingSource {
    /// Matched a filename pattern from the
    /// [`crate::secrets::filename`] pattern table.
    Filename { pattern_id: String },
}

/// A sensitive file discovered by a secrets scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveFinding {
    /// Path of the sensitive file.
    pub path: PathBuf,
    /// Computed severity of the finding.
    pub risk: SensitiveRisk,
    /// What triggered this finding.
    pub source: FindingSource,
    /// Human label from the matched pattern.
    pub label: String,
}
