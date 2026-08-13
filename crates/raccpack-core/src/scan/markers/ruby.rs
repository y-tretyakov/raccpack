//! Ruby ecosystem marker: `Gemfile`.

use super::types::{MarkerDef, MarkerKind};

/// Ruby markers (`Gemfile`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "Gemfile",
    language_hint: Some("Ruby"),
}];
