//! Shared types and helpers for the detect subsystem: the [`Detection`] /
//! [`StackNode`] composite-tree DTOs, the confidence normalizer
//! ([`clamp_confidence`]), the §4.1 language-priority table and small
//! deterministic directory-read/match helpers. The detector contract itself
//! lives in [`super::traits`].

use std::path::{Path, PathBuf};

use crate::domain::Error;
use crate::scan::MarkerHit;

/// One ecosystem-level detection result for a subtree root.
///
/// Produced by the composite stack detection pipeline; until that lands
/// (D2.x) no producer exists and this type fixes only the data contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Detection {
    /// Ecosystem identifier ("rust", "node", …).
    pub ecosystem: String,
    /// Primary language, if resolved for this subtree.
    pub language: Option<String>,
    /// Framework / runtime hints contributed by detectors.
    pub frameworks: Vec<String>,
    /// Confidence in `0.0..=1.0`; producers must normalize through
    /// [`clamp_confidence`].
    pub confidence: f32,
    /// Subtree root this detection applies to. Filled by the composite_dag
    /// pipeline in D2.x; currently a passthrough `PathBuf`.
    pub scope: PathBuf,
    /// Marker names that contributed to this detection.
    pub markers: Vec<String>,
}

/// A recursive node of the composite stack tree: one [`Detection`] plus the
/// nested subtrees below it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct StackNode {
    /// Detection for this node's subtree root.
    pub detection: Detection,
    /// Nested subtree nodes (recursion terminates on an empty vec).
    pub children: Vec<StackNode>,
}

/// Clamp a detector confidence value into `[0.0, 1.0]`.
///
/// Non-finite inputs (`NaN`, `+inf`, `-inf`) map to `0.0`: JSON has no
/// representation for them (`serde_json` would emit `null`), so mapping them
/// deterministically keeps serialized output valid and round-trip stable
/// across platforms and producers.
pub fn clamp_confidence(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Language priority groups from spec §4.1, highest priority first.
///
/// Markers within one group share the same priority; ties are broken by the
/// first hit in `hits` order. `.git` is deliberately absent: it carries no
/// language signal. `Makefile` is present but has no `language_hint`, so a
/// table hit with a `None` hint yields `None` (see [`resolve_language`]).
const LANGUAGE_PRIORITY_GROUPS: &[&[&str]] = &[
    &["Cargo.toml"],
    &["go.mod"],
    &["pom.xml", "build.gradle", "build.gradle.kts"],
    &["package.json"],
    &["pyproject.toml"],
    &["setup.py"],
    &["requirements.txt"],
    &["Gemfile"],
    &["composer.json"],
    &["CMakeLists.txt"],
    &["Makefile"],
];

/// Resolve the primary language from marker hits (spec §4.1).
///
/// The language is the `language_hint` of the highest-priority hit present in
/// [`LANGUAGE_PRIORITY_GROUPS`]; when several hits share the winning priority,
/// the first hit in `hits` order wins. If no hit is in the table, the first
/// hit's hint is used (covers user-supplied `extra_markers`); if that is also
/// absent the result is `None`. A table hit with a `None` hint (e.g.
/// `Makefile`) therefore yields `None` even when another hit carries a hint.
///
/// Deterministic: depends only on `hits` order, never on filesystem state.
pub fn resolve_language(hits: &[MarkerHit]) -> Option<String> {
    let mut best: Option<(usize, &MarkerHit)> = None;
    for hit in hits {
        let Some(level) = priority_level(&hit.name) else {
            continue;
        };
        let is_better = match best {
            Some((best_level, _)) => level < best_level,
            None => true,
        };
        if is_better {
            best = Some((level, hit));
        }
    }
    match best {
        Some((_, hit)) => hit.language_hint.clone(),
        None => hits.first().and_then(|hit| hit.language_hint.clone()),
    }
}

/// Priority level (lower = higher priority) for a marker name, if any.
fn priority_level(name: &str) -> Option<usize> {
    LANGUAGE_PRIORITY_GROUPS
        .iter()
        .position(|group| group.contains(&name))
}

/// Read and sort the entry names of one directory level.
///
/// No recursion and no symlink dereferencing: `read_dir` yields a symlink
/// under its own name, so a symlink named `next.config.js` still matches the
/// filename rule without its target ever being touched. Sorting makes
/// framework detection independent of the filesystem's `read_dir` order.
/// Errors map to [`Error::Io`].
pub fn read_dir_names(dir: &Path) -> Result<Vec<String>, Error> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

/// Whether `names` contains an entry whose name equals `name`.
pub fn has_name(names: &[String], name: &str) -> bool {
    names.iter().any(|candidate| candidate == name)
}

/// Whether `names` contains an entry starting with `prefix` plus at least one
/// more character.
pub fn has_prefix(names: &[String], prefix: &str) -> bool {
    names
        .iter()
        .any(|candidate| candidate.starts_with(prefix) && candidate.len() > prefix.len())
}

/// Whether `names` contains an entry named `{prefix}{ext}` with `ext` in `exts`.
pub fn has_prefix_ext(names: &[String], prefix: &str, exts: &[&str]) -> bool {
    names.iter().any(|candidate| {
        candidate
            .strip_prefix(prefix)
            .is_some_and(|ext| exts.contains(&ext))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_detection() -> Detection {
        Detection {
            ecosystem: "rust".to_string(),
            language: Some("Rust".to_string()),
            frameworks: vec!["Axum".to_string()],
            confidence: 0.9,
            scope: PathBuf::from("/tmp/fixture"),
            markers: vec!["Cargo.toml".to_string()],
        }
    }

    #[test]
    fn clamp_confidence_keeps_in_range_values() {
        assert_eq!(clamp_confidence(0.0), 0.0);
        assert_eq!(clamp_confidence(1.0), 1.0);
        assert_eq!(clamp_confidence(0.42), 0.42);
    }

    #[test]
    fn clamp_confidence_pulls_out_of_range_values_to_bounds() {
        assert_eq!(clamp_confidence(-0.5), 0.0);
        assert_eq!(clamp_confidence(1.5), 1.0);
    }

    #[test]
    fn clamp_confidence_maps_non_finite_to_zero() {
        assert_eq!(clamp_confidence(f32::NAN), 0.0);
        assert_eq!(clamp_confidence(f32::INFINITY), 0.0);
        assert_eq!(clamp_confidence(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn detection_serde_roundtrip() {
        let json = serde_json::to_string(&sample_detection()).unwrap();
        let back: Detection = serde_json::from_str(&json).unwrap();
        assert_eq!(sample_detection(), back);
    }

    #[test]
    fn stack_node_recursive_serde_roundtrip() {
        let node = StackNode {
            detection: sample_detection(),
            children: vec![StackNode {
                detection: Detection {
                    ecosystem: "node".to_string(),
                    language: Some("TypeScript".to_string()),
                    frameworks: Vec::new(),
                    confidence: clamp_confidence(2.5),
                    scope: PathBuf::from("/tmp/fixture/web"),
                    markers: vec!["package.json".to_string()],
                },
                children: Vec::new(),
            }],
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: StackNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }
}
