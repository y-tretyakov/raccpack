//! Marker registry: one ecosystem group per module, aggregated here.
//!
//! Each group file (`rust.rs`, `node.rs`, …) defines a [`MarkerDef`] table for
//! exactly one ecosystem. This module is the single enumeration point:
//! [`default_markers()`] concatenates the groups in a **stable, fixed order**
//! (rust, node, go, python, jvm, ruby, php, cpp, make, git) that reproduces the
//! effective M2.1 hit ordering exactly. Adding a new language = a new group
//! file + one line in the registry below; `candidates.rs` stays
//! ecosystem-agnostic.

mod cpp;
mod git;
mod go;
mod jvm;
mod make;
mod node;
mod php;
mod python;
mod ruby;
mod rust;
mod types;

pub use types::{MarkerDef, MarkerHit, MarkerKind};

use std::sync::OnceLock;

/// All ecosystem groups in registry (aggregation) order.
///
/// Order is intentional and stable: it must preserve the M2.1 marker order
/// (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`,
/// `requirements.txt`, `pom.xml`, `build.gradle`, `build.gradle.kts`,
/// `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`) so
/// [`crate::scan::find_candidates`] hit ordering does not regress.
const GROUPS: &[&[MarkerDef]] = &[
    rust::MARKERS,
    node::MARKERS,
    go::MARKERS,
    python::MARKERS,
    jvm::MARKERS,
    ruby::MARKERS,
    php::MARKERS,
    cpp::MARKERS,
    make::MARKERS,
    git::MARKERS,
];

/// Lazily built aggregate marker table, populated once on first access.
static DEFAULT_MARKERS: OnceLock<&'static [MarkerDef]> = OnceLock::new();

/// The default marker set used by [`crate::scan::find_candidates`].
///
/// Concatenation of every ecosystem group in the stable registry order defined
/// by [`GROUPS`]. Same 14 markers, names, kinds, `language_hint`s and ordering
/// as the former M2.1 `DEFAULT_MARKERS` table.
pub fn default_markers() -> &'static [MarkerDef] {
    DEFAULT_MARKERS.get_or_init(|| {
        let total: usize = GROUPS.iter().map(|group| group.len()).sum();
        let mut markers: Vec<MarkerDef> = Vec::with_capacity(total);
        for group in GROUPS {
            markers.extend_from_slice(group);
        }
        Box::leak(markers.into_boxed_slice())
    })
}
