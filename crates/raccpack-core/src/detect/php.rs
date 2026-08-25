//! PHP ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::traits::StackDetector;

/// Detector for the PHP ecosystem.
pub struct PhpDetector;

impl StackDetector for PhpDetector {
    fn id(&self) -> &'static str {
        "php"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "composer.json")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        // No filename-based framework rules in the MVP set.
        Ok(Stack::default())
    }
}
