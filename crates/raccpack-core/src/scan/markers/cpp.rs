//! C/C++ ecosystem marker: `CMakeLists.txt`.

use super::types::{MarkerDef, MarkerKind};

/// C/C++ markers (`CMakeLists.txt`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "CMakeLists.txt",
    language_hint: Some("C++"),
}];
