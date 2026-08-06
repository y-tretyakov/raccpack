//! Go ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::StackDetector;

/// Detector for the Go ecosystem.
pub struct GoDetector;

impl StackDetector for GoDetector {
    fn id(&self) -> &'static str {
        "go"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "go.mod")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        // No filename-based framework rules in the MVP set.
        Ok(Stack::default())
    }
}
