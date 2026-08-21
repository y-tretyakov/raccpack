//! Git status abstraction: trait [`GitClient`], status enum, repo-root lookup.
//!
//! Pure contract layer — no subprocess logic here (see [`crate::git::process`])
//! and no app/report types; `app → git` is the only allowed direction.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::Result;

/// Lifecycle state of a file inside a git working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitFileStatus {
    /// Committed and clean.
    Tracked,
    /// Present on disk, never added to the index.
    Untracked,
    /// Matched by `.gitignore`.
    Ignored,
    /// Content differs from the index or HEAD.
    Modified,
    /// Newly added to the index.
    Staged,
    /// Deleted from the worktree or staged for deletion.
    Deleted,
    /// Status could not be classified.
    Unknown,
}

impl GitFileStatus {
    /// Stable snake_case name used in JSON reports (`as_str` == serde form).
    pub fn as_str(&self) -> &'static str {
        match self {
            GitFileStatus::Tracked => "tracked",
            GitFileStatus::Untracked => "untracked",
            GitFileStatus::Ignored => "ignored",
            GitFileStatus::Modified => "modified",
            GitFileStatus::Staged => "staged",
            GitFileStatus::Deleted => "deleted",
            GitFileStatus::Unknown => "unknown",
        }
    }
}

/// Coarse repository state for a path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitState {
    /// The path is inside a git working tree.
    pub is_repo: bool,
    /// The working tree has uncommitted changes.
    pub dirty: bool,
}

/// Source of git facts for facade use-cases.
///
/// Implementations must be usable from multiple threads; failures are returned
/// as [`Error`] so callers can soft-fail instead of aborting a run.
pub trait GitClient: Send + Sync {
    /// Whether `path` sits inside a git working tree.
    fn is_repo(&self, path: &Path) -> Result<bool>;

    /// Status of a single file inside `repo`.
    ///
    /// Default implementation batches through [`GitClient::files_status`].
    fn file_status(&self, repo: &Path, file: &Path) -> Result<GitFileStatus> {
        let statuses = self.files_status(repo, std::slice::from_ref(&file.to_path_buf()))?;
        Ok(statuses
            .get(file)
            .copied()
            .unwrap_or(GitFileStatus::Tracked))
    }

    /// Batch status lookup; result keys are exactly the input paths.
    fn files_status(
        &self,
        repo: &Path,
        files: &[PathBuf],
    ) -> Result<HashMap<PathBuf, GitFileStatus>>;
}

/// Walk up from `start` to the nearest directory containing `.git`
/// (either a directory or a worktree pointer file).
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_snake_case_for_every_variant() {
        let pairs = [
            (GitFileStatus::Tracked, "tracked"),
            (GitFileStatus::Untracked, "untracked"),
            (GitFileStatus::Ignored, "ignored"),
            (GitFileStatus::Modified, "modified"),
            (GitFileStatus::Staged, "staged"),
            (GitFileStatus::Deleted, "deleted"),
            (GitFileStatus::Unknown, "unknown"),
        ];
        for (status, expected) in pairs {
            assert_eq!(status.as_str(), expected);
        }
    }

    #[test]
    fn default_file_status_delegates_to_files_status() {
        struct OneTracked;
        impl GitClient for OneTracked {
            fn is_repo(&self, _path: &Path) -> Result<bool> {
                Ok(true)
            }
            fn files_status(
                &self,
                _repo: &Path,
                files: &[PathBuf],
            ) -> Result<HashMap<PathBuf, GitFileStatus>> {
                Ok(files
                    .iter()
                    .map(|f| (f.clone(), GitFileStatus::Untracked))
                    .collect())
            }
        }

        let client = OneTracked;
        assert_eq!(
            client.file_status(Path::new("/r"), Path::new("/r/f")).ok(),
            Some(GitFileStatus::Untracked)
        );
    }
}
