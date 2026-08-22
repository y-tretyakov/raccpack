//! In-memory [`GitClient`] for tests and previews — always compiled.
//!
//! Programmed via builders; every trait method consults the same programmed
//! error first, so a failing mock degrades all calls uniformly.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};
use crate::git::client::{GitClient, GitFileStatus};

/// Scripted client: no subprocess, fully deterministic.
#[derive(Debug, Default)]
pub struct MockGitClient {
    is_repo: bool,
    statuses: HashMap<PathBuf, GitFileStatus>,
    error: Option<Error>,
}

impl MockGitClient {
    /// Empty mock: not a repo, no statuses, no error.
    pub fn new() -> Self {
        Self::default()
    }

    /// Program the `is_repo` answer.
    pub fn with_is_repo(mut self, is_repo: bool) -> Self {
        self.is_repo = is_repo;
        self
    }

    /// Program per-file answers; unlisted files report [`GitFileStatus::Tracked`].
    pub fn with_statuses(mut self, statuses: HashMap<PathBuf, GitFileStatus>) -> Self {
        self.statuses = statuses;
        self
    }

    /// Make every trait method return this error (cloned per call).
    pub fn with_error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }
}

impl GitClient for MockGitClient {
    fn is_repo(&self, _path: &Path) -> Result<bool> {
        match &self.error {
            Some(error) => Err(clone_error(error)),
            None => Ok(self.is_repo),
        }
    }

    fn files_status(
        &self,
        _repo: &Path,
        files: &[PathBuf],
    ) -> Result<HashMap<PathBuf, GitFileStatus>> {
        if let Some(error) = &self.error {
            return Err(clone_error(error));
        }
        Ok(files
            .iter()
            .map(|file| {
                let status = self
                    .statuses
                    .get(file)
                    .copied()
                    .unwrap_or(GitFileStatus::Tracked);
                (file.clone(), status)
            })
            .collect())
    }
}

/// Rebuild an owned [`Error`] from a reference.
///
/// `Error` is not `Clone` (the IO variant holds a non-cloneable source), so the
/// mock reconstructs variants explicitly; the IO source degrades to its
/// Display text.
fn clone_error(error: &Error) -> Error {
    match error {
        Error::PathNotFound { path } => Error::PathNotFound { path: path.clone() },
        Error::NotADirectory { path } => Error::NotADirectory { path: path.clone() },
        Error::Io { path, source } => Error::Io {
            path: path.clone(),
            source: std::io::Error::other(source.to_string()),
        },
        Error::Encrypt { message } => Error::Encrypt {
            message: message.clone(),
        },
        Error::Config { message } => Error::Config {
            message: message.clone(),
        },
        Error::DenVersion { found, expected } => Error::DenVersion {
            found: found.clone(),
            expected,
        },
        Error::StashEmpty { message } => Error::StashEmpty {
            message: message.clone(),
        },
        Error::PathOutsideTarget { path } => Error::PathOutsideTarget { path: path.clone() },
        Error::NotAFile { path } => Error::NotAFile { path: path.clone() },
        Error::Unsupported { feature } => Error::Unsupported {
            feature: feature.clone(),
        },
        Error::Git { message } => Error::Git {
            message: message.clone(),
        },
        Error::Other { message } => Error::Other {
            message: message.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlisted_files_default_to_tracked() {
        let known = PathBuf::from("/r/.env");
        let other = PathBuf::from("/r/other");
        let git = MockGitClient::new()
            .with_is_repo(true)
            .with_statuses(HashMap::from([(known.clone(), GitFileStatus::Ignored)]));

        let statuses = git
            .files_status(Path::new("/r"), &[known.clone(), other.clone()])
            .expect("mock files_status");

        assert_eq!(statuses.get(&known), Some(&GitFileStatus::Ignored));
        assert_eq!(statuses.get(&other), Some(&GitFileStatus::Tracked));
    }

    #[test]
    fn programmed_error_is_returned_from_every_method() {
        let git = MockGitClient::new().with_error(Error::Git {
            message: "boom".to_string(),
        });

        assert!(git.is_repo(Path::new("/r")).is_err());
        assert!(git
            .files_status(Path::new("/r"), &[PathBuf::from("/r/f")])
            .is_err());
    }

    #[test]
    fn default_mock_reports_no_repo_and_empty_statuses() {
        let git = MockGitClient::new();
        assert_eq!(git.is_repo(Path::new("/r")).ok(), Some(false));
        assert!(git
            .files_status(Path::new("/r"), &[])
            .ok()
            .is_some_and(|m| m.is_empty()));
    }
}
