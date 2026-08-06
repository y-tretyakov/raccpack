//! Marker type definitions shared by all ecosystem groups.
//!
//! [`MarkerKind`] and [`MarkerDef`] describe a filesystem entry that signals
//! "a project starts here"; [`MarkerHit`] is a marker that matched inside a
//! candidate directory.

/// What kind of filesystem entry a [`MarkerDef`] matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    /// Exact file name, e.g. `"Cargo.toml"`.
    FileName,
    /// Exact directory name, e.g. `".git"`.
    DirName,
}

/// A single marker that signals "project root here" when found in a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerDef {
    /// Whether the marker is a file or a directory.
    pub kind: MarkerKind,
    /// Exact name to match on `file_name()` (case-sensitive on Linux).
    pub name: &'static str,
    /// Optional language hint for M2.2 (e.g. `"Rust"`).
    pub language_hint: Option<&'static str>,
}

/// A marker that matched inside a candidate directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerHit {
    /// Matched marker name.
    pub name: String,
    /// Whether the marker is a file or a directory.
    pub kind: MarkerKind,
    /// Language hint copied from the matching [`MarkerDef`].
    pub language_hint: Option<String>,
}
