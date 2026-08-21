//! Directory traversal honoring a [`SkipPolicy`] and a max depth.
//!
//! INVARIANT: symlinks are never followed. Every walk is built with
//! `follow_links(false)`, so a symlinked directory or file is yielded (when
//! `include_root` allows) but never traversed.
//!
//! Callers run [`ensure_scan_root`] before [`walk_tree`], then map the yielded
//! [`walkdir::Error`] items to the domain `Error` type:
//!
//! ```no_run
//! # use raccpack_core::{Error, WalkOptions, ensure_scan_root, walk_tree};
//! # fn main() -> Result<(), Error> {
//! # let root = std::path::Path::new("/tmp/example");
//! let opts = WalkOptions::default();
//! ensure_scan_root(root)?;
//! for item in walk_tree(root, &opts) {
//!     let _entry = item.map_err(|e| Error::Other { message: e.to_string() })?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Path-containment checks live here: [`is_path_under_root`] canonicalizes both
//! sides and guarantees an entry stays within the scan root, which `pack` /
//! `stash` rely on. [`canonicalize_existing_prefix`] resolves a path that may
//! not exist yet (canonicalizing its deepest existing ancestor) so the same
//! check can run before any directory is created.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::domain::Error;

use super::skip::SkipPolicy;

/// Options controlling a [`walk_tree`] traversal.
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Maximum directory depth to descend (0 walks only the root).
    pub max_depth: usize,
    /// Policy deciding which directories are skipped.
    pub policy: SkipPolicy,
    /// Whether the root entry itself is yielded (it is always the first item).
    pub include_root: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            max_depth: crate::config::default_max_depth(),
            policy: SkipPolicy::default_scan(),
            include_root: false,
        }
    }
}

/// Walk `root`, skipping directories per `opts.policy`.
///
/// INVARIANT: symlinks are never followed. The underlying [`walkdir::WalkDir`]
/// is always built with `follow_links(false)` and `max_depth(opts.max_depth)`.
///
/// Directories matching the policy are filtered out (and not descended into);
/// all other entries pass through, including symlinks. When
/// [`WalkOptions::include_root`] is false the root entry (always the first
/// item) is not yielded; error items from walkdir are never dropped.
///
/// # Contract
///
/// Call [`ensure_scan_root`] first so `root` is an existing directory. The
/// yielded error type is [`walkdir::Error`]; map it to the domain `Error`
/// downstream, e.g. `item.map_err(|e| Error::Other { message: e.to_string() })`.
pub fn walk_tree<'a>(
    root: &'a Path,
    opts: &'a WalkOptions,
) -> impl Iterator<Item = Result<walkdir::DirEntry, walkdir::Error>> + 'a {
    let mut skip_root = !opts.include_root;
    WalkDir::new(root)
        .follow_links(false)
        .max_depth(opts.max_depth)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                !opts.policy.should_skip_dir(entry.path())
            } else {
                true
            }
        })
        .filter(move |item| {
            if skip_root {
                skip_root = false;
                if let Ok(entry) = item {
                    if entry.depth() == 0 {
                        return false;
                    }
                }
            }
            true
        })
}

/// Map a [`walkdir::Error`] to the domain [`Error`] type.
///
/// IO errors map to [`Error::Io`] with the offending path (falling back to the
/// scan root); walkdir errors without an IO source (e.g. loop detection) map to
/// [`Error::Other`].
pub(crate) fn map_walk_error(err: walkdir::Error, root: &Path) -> Error {
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

/// Validate that `root` exists and is a directory.
///
/// Returns [`Error::PathNotFound`] when `root` does not exist and
/// [`Error::NotADirectory`] when it exists but is not a directory.
pub fn ensure_scan_root(root: &Path) -> Result<(), Error> {
    if !root.exists() {
        return Err(Error::PathNotFound {
            path: root.to_path_buf(),
        });
    }
    if !root.is_dir() {
        return Err(Error::NotADirectory {
            path: root.to_path_buf(),
        });
    }
    Ok(())
}

/// Whether `path` is contained under `root`, after canonicalizing both sides.
///
/// Both paths must exist (`fs::canonicalize`); a missing path yields
/// [`Error::Io`] with the offending path. The check is component-wise on the
/// canonical paths, so a sibling like `/a/bc` is NOT under `/a/b`. Because both
/// sides are canonicalized, a `..` path that resolves inside `root` (e.g.
/// `/a/b/c/../file`) is accepted.
pub fn is_path_under_root(path: &Path, root: &Path) -> Result<bool, Error> {
    let canonical_path = fs::canonicalize(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let canonical_root = fs::canonicalize(root).map_err(|source| Error::Io {
        path: root.to_path_buf(),
        source,
    })?;
    Ok(canonical_path.starts_with(canonical_root))
}

/// Canonicalize the deepest existing ancestor of `path`, then re-append the
/// remaining components lexically.
///
/// Unlike [`is_path_under_root`], `path` itself does not need to exist — the
/// `stash` F-PATH-3 guard runs before its `staging/…` directories are created.
/// `..` components in the existing prefix are resolved by the canonicalization;
/// components below the resolved ancestor are appended verbatim, so callers
/// must not reintroduce `..` or symlinks there. Relative paths with no existing
/// ancestor resolve against the working directory.
pub(crate) fn canonicalize_existing_prefix(path: &Path) -> Result<PathBuf, Error> {
    if path.as_os_str().is_empty() {
        return Err(Error::Other {
            message: "cannot resolve an empty path".to_string(),
        });
    }
    let mut tail: Vec<OsString> = Vec::new();
    let mut prefix = path;
    loop {
        if let Ok(real) = fs::canonicalize(prefix) {
            let mut resolved = real;
            for component in tail.iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        match prefix.file_name() {
            Some(name) => {
                if let Some(parent) = prefix.parent() {
                    if !parent.as_os_str().is_empty() {
                        tail.push(name.to_os_string());
                        prefix = parent;
                        continue;
                    }
                }
                tail.push(name.to_os_string());
            }
            None => {
                return Err(Error::Other {
                    message: format!("cannot resolve {}: no existing ancestor", path.display()),
                });
            }
        }
        let working_dir = fs::canonicalize(Path::new(".")).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut resolved = working_dir;
        for component in tail.iter().rev() {
            resolved.push(component);
        }
        return Ok(resolved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn is_path_under_root_accepts_nested_file() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("proj");
        fs::create_dir_all(root.join("src")).unwrap();
        let inside = root.join("src/main.rs");
        fs::write(&inside, b"fn main() {}\n").unwrap();

        assert!(is_path_under_root(&inside, &root).unwrap());
        assert!(is_path_under_root(&root, &root).unwrap());
    }

    #[test]
    fn is_path_under_root_rejects_sibling_prefix() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("b");
        fs::create_dir(&root).unwrap();
        let sibling = dir.path().join("bc");
        fs::write(&sibling, b"not under b\n").unwrap();

        assert!(!is_path_under_root(&sibling, &root).unwrap());
    }

    #[test]
    fn is_path_under_root_resolves_dotdot_inside() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("a/b");
        fs::create_dir_all(root.join("c")).unwrap();
        let real = root.join("file.txt");
        fs::write(&real, b"x\n").unwrap();

        let via_dotdot = root.join("c/../file.txt");
        assert!(is_path_under_root(&via_dotdot, &root).unwrap());
    }

    #[test]
    fn is_path_under_root_missing_path_is_err() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("proj");
        fs::create_dir(&root).unwrap();
        let missing = root.join("nope.txt");

        assert!(is_path_under_root(&missing, &root).is_err());
    }
}
