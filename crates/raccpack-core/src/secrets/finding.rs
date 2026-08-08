//! Finding types produced by secret scans.
//!
//! [`SensitiveFinding`] is the internal representation of a discovered
//! sensitive file: its path, computed [`SensitiveRisk`], the source(s) that
//! triggered it, and human labels. It carries **no secret values** — every
//! value is masked via [`super::mask::MaskedValue`] (`masked` preview +
//! `value_hash` + `original_len`), so nothing here can leak raw credentials.
//!
//! A finding can be triggered by a filename pattern, a content hit, or both
//! (merged per path). Invariant: `source == sources[0]` and
//! `label == labels[0]`, so old readers of `source` / `label` keep working.

use std::path::PathBuf;

use crate::domain::SensitiveRisk;

use super::mask::MaskedValue;

/// Where a [`SensitiveFinding`] came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingSource {
    /// Matched a filename pattern from the
    /// [`crate::secrets::filename`] pattern table.
    Filename { pattern_id: String },
    /// Matched a content marker from the
    /// [`crate::secrets::content`] table.
    Content {
        /// Marker id from [`crate::secrets::content::ContentMarker::id`].
        marker_id: String,
        /// Masked preview of the matched value; never the raw value.
        masked: MaskedValue,
        /// 1-based line number, if known.
        line: Option<u32>,
    },
}

/// A sensitive file discovered by a secrets scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveFinding {
    /// Path of the sensitive file.
    pub path: PathBuf,
    /// Computed severity of the finding (max over all sources).
    pub risk: SensitiveRisk,
    /// Primary source (== `sources[0]`). Kept for M3.1 compatibility.
    pub source: FindingSource,
    /// Primary label (== `labels[0]`).
    pub label: String,
    /// All triggers in deterministic order: filename matches (table order),
    /// then content hits (line/marker order).
    pub sources: Vec<FindingSource>,
    /// Labels aligned with `sources`.
    pub labels: Vec<String>,
    /// Masked preview of the highest-risk content hit, if any.
    pub content_match: Option<MaskedValue>,
}
