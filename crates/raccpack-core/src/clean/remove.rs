//! Removal of trash directories (A2.2).
//!
//! [`remove_trash_dir`] deletes one matched trash directory tree after
//! recomputing its byte size, and never follows or deletes symlinks. Path
//! containment is enforced by the caller (`app::rinse`) before any call.

use std::path::Path;

use crate::domain::{Error, Result};

use super::detect::dir_size_bytes;

/// Remove the directory tree at `path`, returning the bytes that were under it.
///
/// The size is recomputed immediately before deletion with the same restricted
/// walk as discovery ([`dir_size_bytes`], never following symlinks), so the
/// returned value reflects the actual content at removal time.
///
/// A symlink at `path` is never followed or removed: it yields `Ok(0)` and the
/// target directory is left untouched (MVP safety rule for symlink dirs).
pub fn remove_trash_dir(path: &Path) -> Result<u64> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }

    let bytes = dir_size_bytes(path)?;
    std::fs::remove_dir_all(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(bytes)
}
