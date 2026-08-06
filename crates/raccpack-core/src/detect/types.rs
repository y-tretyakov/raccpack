//! Shared types for the detect subsystem: the [`StackDetector`] trait, the
//! §4.1 language-priority table, and small deterministic helpers.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

/// A detector contributing framework knowledge for exactly one ecosystem.
///
/// Detectors never walk or scan; they read at most one level of the project
/// directory (see `read_dir_names`, plus a single `config/` peek in the
/// Ruby detector) to look for framework marker files. The orchestrator
/// ([`super::detect_stack`]) collects their `frameworks`, resolves `language`
/// centrally by priority and sets `markers` from the hits, so detectors must
/// leave both fields empty/unset.
///
/// `detect` returns a [`Result`] so shallow-read IO failures surface as
/// [`Error::Io`] (spec §5), which a bare `-> Stack` signature cannot express.
pub trait StackDetector: Send + Sync {
    /// Stable identifier used for diagnostics and future configuration.
    fn id(&self) -> &'static str;

    /// Whether this detector applies to the given marker hits.
    ///
    /// A detector matches when at least one of its ecosystem markers appears
    /// in `hits`. The orchestrator additionally probes *all* detectors when
    /// `hits` is empty (path-only detection), so this method only gates the
    /// marker-driven case.
    fn matches(&self, hits: &[MarkerHit]) -> bool;

    /// Produce a [`Stack`] contribution for the project at `project_dir`.
    ///
    /// Only the returned `frameworks` are used by the orchestrator. A detector
    /// does its shallow read here and only when the orchestrator decided to
    /// consult it. Errors from reading the directory map to [`Error::Io`].
    fn detect(&self, hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error>;
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
