//! Select sensitive files for stashing.
//!
//! [`select_files_for_stash`] turns either an explicit file list
//! ([`StashSelectOptions::only_files`]) or a full [`crate::secrets::scan`] into
//! [`StashFileEntry`]s: absolute path, a `relative_path` guaranteed to contain
//! no `..` (safe for tar), the folded [`SensitiveRisk`], and the file size.
//!
//! Selected paths must be contained under the target root (F-PATH-1); a path
//! escaping it is an error, never a silently-broadened archive. Files that do
//! not meet the minimum risk are simply not included (`Ok(vec![])` when nothing
//! survives).

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::domain::{Error, SensitiveRisk};
use crate::scan::is_path_under_root;
use crate::scan::walk::ensure_scan_root;

use super::content::{scan_file_content, ContentScanLimits};
use super::filename::match_filename_all;
use super::risk::upgrade_risk;
use super::scan::{scan_secrets, SecretScanOptions};

/// One file selected for stashing.
#[derive(Debug, Clone)]
pub struct StashFileEntry {
    /// Absolute path of the source file on disk.
    pub path: PathBuf,
    /// Path relative to the target root (for tar); never contains `..`.
    pub relative_path: PathBuf,
    /// Folded severity of all sources that met the minimum risk.
    pub risk: SensitiveRisk,
    /// Size of the source file in bytes.
    pub size_bytes: u64,
}

/// Options controlling [`select_files_for_stash`].
#[derive(Debug, Clone)]
pub struct StashSelectOptions {
    /// Project or subtree root that every selected file must live under.
    pub target: PathBuf,
    /// Explicit file list. When `None`, the whole target tree is re-scanned.
    pub only_files: Option<Vec<PathBuf>>,
    /// Minimum risk to include; sources below this are dropped per path.
    pub min_risk: SensitiveRisk,
    /// Default true. When false, only filename patterns decide inclusion.
    pub scan_content: bool,
}

impl Default for StashSelectOptions {
    fn default() -> Self {
        Self {
            target: PathBuf::new(),
            only_files: None,
            min_risk: SensitiveRisk::High,
            scan_content: true,
        }
    }
}

/// Select sensitive files under `opts.target` for stashing.
///
/// # Algorithm
///
/// - `opts.target` is validated with [`ensure_scan_root`] first.
/// - With `only_files`: every path must (a) exist — else
///   [`Error::PathNotFound`], (b) be a regular file — else
///   [`Error::Other`], and (c) be contained under `target` via
///   [`is_path_under_root`] — else [`Error::Other`]. Inclusion is decided by
///   filename matches and (when `scan_content`) content hits filtered against
///   `min_risk`; a file with no qualifying source is skipped, not an error.
///   Duplicates (same canonical path) collapse into one entry.
/// - Without `only_files`: [`scan_secrets`] runs with the same `min_risk` /
///   `scan_content`, and each finding maps to one entry.
/// - `relative_path` is `path` stripped of `target` with every component
///   validated (mirroring `archive::pack::relative_posix_name`); `..`, `/`, and
///   drive-prefix components are rejected with [`Error::Other`].
/// - `size_bytes` comes from `fs::metadata` (IO failures → [`Error::Io`]).
/// - Results are sorted by `path` ascending; an empty selection is
///   `Ok(vec![])` (the "nothing to stash" error belongs to
///   [`crate::secrets::stash_batch::write_stash_age`]).
pub fn select_files_for_stash(opts: &StashSelectOptions) -> Result<Vec<StashFileEntry>, Error> {
    ensure_scan_root(&opts.target)?;

    let mut entries: Vec<StashFileEntry> = Vec::new();
    if let Some(files) = &opts.only_files {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for file in files {
            if !file.exists() {
                return Err(Error::PathNotFound { path: file.clone() });
            }
            if file.is_dir() {
                return Err(Error::Other {
                    message: format!("not a file: {}", file.display()),
                });
            }
            if !is_path_under_root(file, &opts.target)? {
                return Err(Error::Other {
                    message: format!("path outside stash target: {}", file.display()),
                });
            }
            let canonical = fs::canonicalize(file).map_err(|source| Error::Io {
                path: file.clone(),
                source,
            })?;
            if !seen.insert(canonical) {
                continue;
            }

            let Some(risk) = qualifying_risk(file, opts)? else {
                continue;
            };
            let relative_path = relative_stash_path(&opts.target, file)?;
            let size_bytes = metadata_len(file)?;
            entries.push(StashFileEntry {
                path: file.clone(),
                relative_path,
                risk,
                size_bytes,
            });
        }
    } else {
        let scan_opts = SecretScanOptions {
            min_risk: opts.min_risk,
            scan_content: opts.scan_content,
            ..SecretScanOptions::default()
        };
        for finding in scan_secrets(&opts.target, &scan_opts)? {
            let relative_path = relative_stash_path(&opts.target, &finding.path)?;
            let size_bytes = metadata_len(&finding.path)?;
            entries.push(StashFileEntry {
                path: finding.path,
                relative_path,
                risk: finding.risk,
                size_bytes,
            });
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Fold the risk of the filename + content sources that meet `opts.min_risk`.
///
/// Returns `Ok(None)` when no source qualifies (the file is then skipped, not
/// an error). IO errors from [`scan_file_content`] propagate as
/// [`Error::Io`] — they are never treated as "no content hits".
fn qualifying_risk(file: &Path, opts: &StashSelectOptions) -> Result<Option<SensitiveRisk>, Error> {
    let mut risk = SensitiveRisk::Low;
    let mut has_source = false;
    for matched in match_filename_all(file) {
        if matched.risk.at_least(opts.min_risk) {
            risk = upgrade_risk(risk, matched.risk);
            has_source = true;
        }
    }
    if opts.scan_content {
        for hit in scan_file_content(file, &ContentScanLimits::default())? {
            if hit.risk.at_least(opts.min_risk) {
                risk = upgrade_risk(risk, hit.risk);
                has_source = true;
            }
        }
    }
    Ok(has_source.then_some(risk))
}

/// `path` made relative to `target`, guaranteed free of `..` / absolute roots.
///
/// Components are re-joined into a `PathBuf` (mirrors
/// `archive::pack::relative_posix_name`); `ParentDir`, `RootDir`, and drive
/// prefixes are rejected so the resulting path can never escape `target`.
fn relative_stash_path(target: &Path, path: &Path) -> Result<PathBuf, Error> {
    let relative = path.strip_prefix(target).map_err(|_| Error::Other {
        message: format!("path escapes stash target: {}", path.display()),
    })?;
    let mut result = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::Other {
                    message: format!("path escapes stash target: {}", path.display()),
                });
            }
        }
    }
    Ok(result)
}

/// Length of `path` in bytes, mapping IO failures to [`Error::Io`].
fn metadata_len(path: &Path) -> Result<u64, Error> {
    fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Component;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
        fs::write(&path, content).expect("write fixture file");
    }

    #[test]
    fn select_picks_env_but_not_plain_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, ".env", b"TOKEN=secret\n");
        write(&target, "notes.txt", b"nothing sensitive\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_path, PathBuf::from(".env"));
        assert_eq!(entries[0].risk, SensitiveRisk::High);
        assert_eq!(entries[0].size_bytes, 13);
    }

    #[test]
    fn min_risk_critical_filters_high_only_files() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, ".env", b"TOKEN=secret\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            min_risk: SensitiveRisk::Critical,
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn only_files_limits_the_set() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, ".env", b"TOKEN=secret\n");
        write(&target, ".env.production", b"TOKEN=prod\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            only_files: Some(vec![target.join(".env")]),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_path, PathBuf::from(".env"));
    }

    #[test]
    fn only_files_dedupes_by_canonical_path() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        let env = target.join(".env");
        write(&target, ".env", b"TOKEN=secret\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            only_files: Some(vec![env.clone(), env.clone()]),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn only_files_path_outside_target_is_error() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, ".env", b"TOKEN=secret\n");
        let outside = dir.path().join("other/secret.txt");
        fs::create_dir_all(outside.parent().unwrap()).unwrap();
        fs::write(&outside, b"AKIAABCDEFGHIJKLMNOPQRST\n").unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            only_files: Some(vec![outside]),
            ..StashSelectOptions::default()
        };
        let err = select_files_for_stash(&opts).unwrap_err();
        assert!(
            err.to_string().contains("outside"),
            "expected outside-target error, got: {err}"
        );
    }

    #[test]
    fn only_files_missing_path_is_path_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            only_files: Some(vec![target.join("nope.env")]),
            ..StashSelectOptions::default()
        };
        let err = select_files_for_stash(&opts).unwrap_err();
        assert!(matches!(err, Error::PathNotFound { .. }));
    }

    #[test]
    fn only_files_directory_is_error() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(target.join("sub")).unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            only_files: Some(vec![target.join("sub")]),
            ..StashSelectOptions::default()
        };
        let err = select_files_for_stash(&opts).unwrap_err();
        assert!(err.to_string().contains("not a file"));
    }

    #[test]
    fn relative_path_never_contains_parent_dir() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(target.join("deep/sub")).unwrap();
        write(&target, "deep/sub/.env", b"TOKEN=secret\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_path, PathBuf::from("deep/sub/.env"));
        assert!(!entries[0]
            .relative_path
            .components()
            .any(|c| matches!(c, Component::ParentDir)));
    }

    #[test]
    fn empty_selection_is_ok_not_error() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, "notes.txt", b"nothing sensitive\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn scan_content_false_is_filename_only() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        write(&target, "notes.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");

        let opts = StashSelectOptions {
            target: target.clone(),
            scan_content: false,
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn missing_target_is_path_not_found() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("does-not-exist");

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let err = select_files_for_stash(&opts).unwrap_err();
        assert!(matches!(err, Error::PathNotFound { .. }));
    }
}
