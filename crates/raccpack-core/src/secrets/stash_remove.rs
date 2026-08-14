//! Remove stash source files after a successful encrypt (Commit semantics).
//!
//! [`remove_stash_sources`] is an explicit, opt-in call: the stash pipeline
//! never deletes sources as a side effect of encryption. It is fail-fast —
//! the first unexpected path state aborts the whole batch — and directories are
//! skipped (only regular files count). Removing a symlink deletes the link
//! itself, never its target.

use std::fs;

use crate::domain::Error;

use super::stash_batch::StashManifestEntry;

/// Delete every manifest entry's source file, returning the number removed.
///
/// # Contract
///
/// - Directories are skipped and not counted.
/// - Missing paths (including a file already removed by a previous call) and
///   any other removal failure abort immediately with [`Error::Io`]
///   (fail-fast; no partial-success bookkeeping beyond the counter).
/// - Symlinks are removed with `fs::remove_file`, which deletes the link, not
///   the target.
///
/// # Errors
///
/// Any failure (missing file, permission denied, removal error) → [`Error::Io`]
/// on the offending path.
pub fn remove_stash_sources(entries: &[StashManifestEntry]) -> Result<usize, Error> {
    let mut removed = 0usize;
    for entry in entries {
        let path = &entry.original_path;
        let metadata = fs::metadata(path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() {
            continue;
        }
        fs::remove_file(path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::domain::SensitiveRisk;
    use tempfile::TempDir;

    fn entry(path: std::path::PathBuf) -> StashManifestEntry {
        StashManifestEntry {
            original_path: path,
            risk: SensitiveRisk::High,
            size_bytes: 8,
        }
    }

    #[test]
    fn removes_files_and_returns_count() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a.env");
        let b = dir.path().join("b.env");
        fs::write(&a, b"TOKEN=1\n").unwrap();
        fs::write(&b, b"TOKEN=2\n").unwrap();

        let entries = vec![entry(a.clone()), entry(b.clone())];
        let removed = remove_stash_sources(&entries).unwrap();

        assert_eq!(removed, 2);
        assert!(!a.exists());
        assert!(!b.exists());
    }

    #[test]
    fn second_call_fails_fast_on_missing_file() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a.env");
        fs::write(&a, b"TOKEN=1\n").unwrap();
        let entries = vec![entry(a.clone())];

        assert_eq!(remove_stash_sources(&entries).unwrap(), 1);
        let err = remove_stash_sources(&entries).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn directories_are_skipped_and_not_counted() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();

        let entries = vec![entry(sub.clone())];
        let removed = remove_stash_sources(&entries).unwrap();

        assert_eq!(removed, 0);
        assert!(sub.exists(), "directories must not be removed");
    }
}
