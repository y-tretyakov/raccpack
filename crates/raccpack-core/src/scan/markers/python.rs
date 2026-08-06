//! Python ecosystem markers: `pyproject.toml`, `setup.py`, `requirements.txt`.

use super::types::{MarkerDef, MarkerKind};

/// Python markers (`pyproject.toml`, `setup.py`, `requirements.txt`).
pub static MARKERS: &[MarkerDef] = &[
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
];
