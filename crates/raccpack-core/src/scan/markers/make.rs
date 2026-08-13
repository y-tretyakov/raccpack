//! Generic build-system marker: `Makefile` (no language hint).

use super::types::{MarkerDef, MarkerKind};

/// Generic Makefile marker (language-agnostic, no hint).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "Makefile",
    language_hint: None,
}];
