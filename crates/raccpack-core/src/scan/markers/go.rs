//! Go ecosystem marker: `go.mod`.

use super::types::{MarkerDef, MarkerKind};

/// Go markers (`go.mod`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "go.mod",
    language_hint: Some("Go"),
}];
