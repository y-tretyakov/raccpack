//! Generic Makefile detector (language-agnostic).

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::StackDetector;

/// Detector for projects signaled by a `Makefile` (no language hint, no
/// filename-based frameworks in the MVP set).
pub struct MakeDetector;

impl StackDetector for MakeDetector {
    fn id(&self) -> &'static str {
        "make"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "Makefile")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        Ok(Stack::default())
    }
}
