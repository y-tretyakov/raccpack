//! VCS directory marker: `.git`.

use super::types::{MarkerDef, MarkerKind};

/// Git repository marker (directory named `.git`, no language hint).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::DirName,
    name: ".git",
    language_hint: None,
}];
