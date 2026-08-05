//! Marker definitions used to recognize candidate project roots.
//!
//! A marker is a filesystem entry (file or directory) whose exact name signals
//! "a project starts here". The table [`DEFAULT_MARKERS`] is the default set;
//! adding a marker means appending one row to it, nothing else changes.

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

/// Default marker set used by [`crate::scan::find_candidates`].
pub static DEFAULT_MARKERS: &[MarkerDef] = &[
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "Cargo.toml",
        language_hint: Some("Rust"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "package.json",
        language_hint: Some("JavaScript"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "go.mod",
        language_hint: Some("Go"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "pyproject.toml",
        language_hint: Some("Python"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "setup.py",
        language_hint: Some("Python"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "requirements.txt",
        language_hint: Some("Python"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "pom.xml",
        language_hint: Some("Java"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "build.gradle",
        language_hint: Some("Java"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "build.gradle.kts",
        language_hint: Some("Kotlin"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "Gemfile",
        language_hint: Some("Ruby"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "composer.json",
        language_hint: Some("PHP"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "CMakeLists.txt",
        language_hint: Some("C++"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "Makefile",
        language_hint: None,
    },
    MarkerDef {
        kind: MarkerKind::DirName,
        name: ".git",
        language_hint: None,
    },
];

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
