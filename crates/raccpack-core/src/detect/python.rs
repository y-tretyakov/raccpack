//! Python ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::traits::StackDetector;
use super::types::{has_name, read_dir_names};

/// Detector for the Python ecosystem.
pub struct PythonDetector;

impl StackDetector for PythonDetector {
    fn id(&self) -> &'static str {
        "python"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| {
            matches!(
                hit.name.as_str(),
                "pyproject.toml" | "setup.py" | "requirements.txt"
            )
        })
    }

    fn detect(&self, _hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error> {
        let names = read_dir_names(project_dir)?;
        let mut frameworks = Vec::new();
        if has_name(&names, "manage.py") {
            frameworks.push("Django".to_string());
        }
        Ok(Stack {
            language: None,
            frameworks,
            markers: Vec::new(),
        })
    }
}
