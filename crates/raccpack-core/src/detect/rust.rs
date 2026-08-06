//! Rust ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::StackDetector;

/// Detector for the Rust ecosystem.
pub struct RustDetector;

impl StackDetector for RustDetector {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "Cargo.toml")
    }

    fn detect(&self, _hits: &[MarkerHit], _project_dir: &Path) -> Result<Stack, Error> {
        // No filename-based framework rules in the MVP set (Axum needs
        // Cargo.toml parsing, deferred to a later phase).
        Ok(Stack::default())
    }
}
