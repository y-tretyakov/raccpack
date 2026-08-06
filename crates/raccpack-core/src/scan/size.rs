//! Project byte-size accounting.
//!
//! [`project_size_bytes`] sums the size of regular files under a project root
//! while honoring a [`SkipPolicy`] and the project-wide invariant that
//! symlinks are never followed.

use std::path::Path;

use crate::domain::Error;

use super::skip::SkipPolicy;
use super::walk::{ensure_scan_root, walk_tree, WalkOptions};

/// Sum the byte size of regular files under `path`.
///
/// `path` must be an existing directory; a missing path maps to
/// [`Error::PathNotFound`] and a non-directory to [`Error::NotADirectory`].
/// Directories matching `policy` are never descended into; `max_depth` limits
/// traversal depth (0 walks only the root, which is itself never counted).
///
/// Symlinks are skipped entirely — never followed, never counted: their targets
/// may live outside the project root, so counting them would either require
/// following links (forbidden) or misreport size (a link is not a payload).
/// Unreadable files are skipped and the scan continues (spec §6: skip +
/// continue is safer for UX). Walk errors map to [`Error::Io`], or
/// [`Error::Other`] when walkdir reports a non-IO failure such as loop
/// detection.
pub fn project_size_bytes(
    path: &Path,
    policy: &SkipPolicy,
    max_depth: usize,
) -> Result<u64, Error> {
    ensure_scan_root(path)?;
    let opts = WalkOptions {
        max_depth,
        policy: policy.clone(),
        include_root: false,
    };
    let mut total: u64 = 0;
    for item in walk_tree(path, &opts) {
        let entry = item.map_err(|err| map_walk_error(err, path))?;
        let file_type = entry.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

/// Map a [`walkdir::Error`] to the domain [`Error`] type.
///
/// IO errors map to [`Error::Io`] with the offending path (falling back to the
/// project root); walkdir errors without an IO source (e.g. loop detection)
/// map to [`Error::Other`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_files_and_skips_policy_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(proj.join("src")).unwrap();
        std::fs::create_dir_all(proj.join("node_modules/lodash")).unwrap();
        std::fs::create_dir_all(proj.join("target/debug")).unwrap();
        std::fs::write(proj.join("src/main.rs"), "abc").unwrap();
        std::fs::write(proj.join("README.md"), "defg").unwrap();
        std::fs::write(proj.join("node_modules/lodash/index.js"), "vendor").unwrap();
        std::fs::write(proj.join("target/debug/app"), "binary").unwrap();

        let size = project_size_bytes(&proj, &SkipPolicy::default_scan(), 6).expect("size ok");
        assert_eq!(size, 7);
    }

    #[test]
    fn empty_dir_is_zero() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let size = project_size_bytes(&empty, &SkipPolicy::default_scan(), 6).expect("size ok");
        assert_eq!(size, 0);
    }

    #[test]
    fn missing_path_is_path_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let err = project_size_bytes(&missing, &SkipPolicy::default_scan(), 6).unwrap_err();
        assert!(matches!(err, Error::PathNotFound { .. }));
    }

    #[test]
    fn file_path_is_not_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let err = project_size_bytes(&file, &SkipPolicy::default_scan(), 6).unwrap_err();
        assert!(matches!(err, Error::NotADirectory { .. }));
    }

    #[test]
    fn symlink_files_are_not_counted() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("real.txt"), "1234567890").unwrap();
        std::os::unix::fs::symlink(proj.join("real.txt"), proj.join("link.txt")).unwrap();

        let size = project_size_bytes(&proj, &SkipPolicy::default_scan(), 6).expect("size ok");
        assert_eq!(size, 10, "symlink itself must not be counted");
    }
}
