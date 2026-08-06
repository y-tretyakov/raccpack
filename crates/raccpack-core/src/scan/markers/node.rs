//! Node/JavaScript ecosystem marker: `package.json`.

use super::types::{MarkerDef, MarkerKind};

/// Node/JavaScript markers (`package.json`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "package.json",
    language_hint: Some("JavaScript"),
}];
