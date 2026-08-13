//! C/C++ ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::StackDetector;

/// Detector for the C/C++ ecosystem.
pub struct CppDetector;

impl StackDetector for CppDetector {
    fn id(&self) -> &'static str {
        "cpp"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "CMakeLists.txt")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        // No filename-based framework rules in the MVP set.
        Ok(Stack::default())
    }
}
