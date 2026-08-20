//! Raid-wide staging helpers (A3.3 PR2).
//!
//! The atomic raid keeps every intermediate artifact under one shared
//! `den/staging/{raid_id}/` directory. Stash and pack create the directory
//! themselves when writing (`create_dir_all` on the artifact parent), so the
//! raid runner never creates it in advance — that keeps a failed stash from
//! bootstrapping the den. [`remove_raid_staging`] is the best-effort cleanup
//! used on phase failures and after a commit.

use std::fs;
use std::path::{Path, PathBuf};

/// The shared raid staging directory: `den/staging/{raid_id}`.
pub(super) fn raid_staging_path(den_dir: &Path, raid_id: &str) -> PathBuf {
    den_dir.join("staging").join(raid_id)
}

/// Best-effort removal of the whole raid staging directory tree.
///
/// Errors are ignored: this is a cleanup helper, and the caller's original
/// result/error is authoritative. A non-existent path is a no-op.
pub(super) fn remove_raid_staging(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
