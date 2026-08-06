//! PHP ecosystem marker: `composer.json`.

use super::types::{MarkerDef, MarkerKind};

/// PHP markers (`composer.json`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "composer.json",
    language_hint: Some("PHP"),
}];
