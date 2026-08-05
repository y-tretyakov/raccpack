//! Candidate project discovery over a scan root.
//!
//! [`find_candidates`] walks the scan root with the M1.4 walk helper and, for
//! every visited directory plus the root itself, reads that directory's
//! entries once and matches them against the marker table. Directories that
//! contain at least one marker become [`ProjectCandidate`]s.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::domain::Error;

use super::markers::{MarkerDef, MarkerHit, MarkerKind, DEFAULT_MARKERS};
use super::walk::{ensure_scan_root, walk_tree, WalkOptions};

/// A directory that looks like a project root because it contains markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCandidate {
    /// Directory that contains the marker(s).
    pub path: PathBuf,
    /// Display name (usually `file_name` of `path`).
    pub name: String,
    /// Markers that matched inside this directory.
    pub markers: Vec<MarkerHit>,
    /// True if a `.git` directory is among the matched markers.
    pub is_git_repo: bool,
}

/// Options controlling [`find_candidates`].
#[derive(Debug, Clone)]
pub struct CandidateOptions {
    /// Maximum directory depth to descend (0 walks only the root).
    pub max_depth: usize,
    /// Policy deciding which directories are skipped.
    pub policy: crate::scan::SkipPolicy,
    /// Additional markers beyond [`DEFAULT_MARKERS`] (optional).
    pub extra_markers: Vec<MarkerDef>,
    /// If true, a directory with only `.git` and no other markers still
    /// becomes a candidate.
    pub accept_git_only: bool,
}

impl Default for CandidateOptions {
    fn default() -> Self {
        Self {
            max_depth: 6,
            policy: crate::scan::SkipPolicy::default_scan(),
            extra_markers: Vec::new(),
            accept_git_only: true,
        }
    }
}

/// Discover candidate project roots under `root`.
///
/// The scan root is validated with [`ensure_scan_root`] and walked with
/// [`walk_tree`] (symlinks never followed, `max_depth` and `policy` taken from
/// `opts`). For every visited directory plus the root itself, the directory's
/// entries are read once and matched against [`DEFAULT_MARKERS`] plus
/// `opts.extra_markers` by exact, case-sensitive `file_name()`. Every
/// directory with at least one matching marker becomes a [`ProjectCandidate`].
///
/// `.git` is in [`crate::scan::SkipPolicy::default_scan()`], so the walker
/// never descends into it — that is fine and desired. `.git` is recognized as
/// a marker by reading the *parent* directory's entries, not by the walker
/// yielding `.git`, so a repo root is still detected. Symlinked directories
/// are never read and produce no candidates. Nested projects are not
/// collapsed: every directory with markers is its own candidate.
///
/// Returns the candidates sorted stably by `path`.
pub fn find_candidates(
    root: &Path,
    opts: &CandidateOptions,
) -> Result<Vec<ProjectCandidate>, Error> {
    ensure_scan_root(root)?;

    let markers: Vec<MarkerDef> = DEFAULT_MARKERS
        .iter()
        .chain(opts.extra_markers.iter())
        .cloned()
        .collect();

    let walk_opts = WalkOptions {
        max_depth: opts.max_depth,
        policy: opts.policy.clone(),
        include_root: false,
    };

    let mut candidates: Vec<ProjectCandidate> = Vec::new();

    if let Some(candidate) = inspect_dir(root, &markers, opts.accept_git_only)? {
        candidates.push(candidate);
    }

    for item in walk_tree(root, &walk_opts) {
        let entry = item.map_err(|err| map_walk_error(err, root))?;
        if !entry.file_type().is_dir() {
            continue;
        }
        if let Some(candidate) = inspect_dir(entry.path(), &markers, opts.accept_git_only)? {
            candidates.push(candidate);
        }
    }

    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(candidates)
}

/// Read one directory's entries and build a candidate when a marker matches.
///
/// Directory names are collected once into a set, then the marker table is
/// iterated in its fixed order so [`MarkerHit`] ordering is deterministic and
/// independent of the filesystem's `read_dir` order. A directory that is a
/// symlink is never read.
fn inspect_dir(
    dir: &Path,
    markers: &[MarkerDef],
    accept_git_only: bool,
) -> Result<Option<ProjectCandidate>, Error> {
    let meta = std::fs::symlink_metadata(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    if meta.file_type().is_symlink() {
        return Ok(None);
    }

    let mut names: HashSet<OsString> = HashSet::new();
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        names.insert(entry.file_name());
    }

    let mut hits: Vec<MarkerHit> = Vec::new();
    let mut only_git = true;
    for marker in markers {
        if !names.contains(OsStr::new(marker.name)) {
            continue;
        }
        let is_git = marker.kind == MarkerKind::DirName && marker.name == ".git";
        hits.push(MarkerHit {
            name: marker.name.to_string(),
            kind: marker.kind,
            language_hint: marker.language_hint.map(str::to_string),
        });
        if !is_git {
            only_git = false;
        }
    }

    if hits.is_empty() {
        return Ok(None);
    }
    if only_git && !accept_git_only {
        return Ok(None);
    }

    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    let is_git_repo = hits.iter().any(|hit| hit.name == ".git");

    Ok(Some(ProjectCandidate {
        path: dir.to_path_buf(),
        name,
        markers: hits,
        is_git_repo,
    }))
}

/// Map a [`walkdir::Error`] to the domain [`Error`] type.
///
/// IO errors map to [`Error::Io`] with the offending path (falling back to the
/// scan root); walkdir errors without an IO source (e.g. loop detection) map
/// to [`Error::Other`].
fn map_walk_error(err: walkdir::Error, root: &Path) -> Error {
    let path = err
        .path()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf());
    let message = err.to_string();
    match err.into_io_error() {
        Some(source) => Error::Io { path, source },
        None => Error::Other { message },
    }
}
