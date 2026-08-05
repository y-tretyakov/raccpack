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
//! `is_under_root` / path-containment checks are NOT implemented here; they are
//! a required follow-up before `pack` / `stash` can rely on entries staying
//! within the scan root.

use std::path::Path;

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
