//! Rust ecosystem marker: `Cargo.toml`.

use super::types::{MarkerDef, MarkerKind};

/// Rust markers (`Cargo.toml`).
pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "Cargo.toml",
    language_hint: Some("Rust"),
}];
