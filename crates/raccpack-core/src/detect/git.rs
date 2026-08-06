//! Git/VCS detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::StackDetector;

/// Detector for git repositories (carries no language and no frameworks).
pub struct GitDetector;

impl StackDetector for GitDetector {
    fn id(&self) -> &'static str {
        "git"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == ".git")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        Ok(Stack::default())
    }
}
