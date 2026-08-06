//! Ruby ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::{has_name, read_dir_names, StackDetector};

/// Detector for the Ruby ecosystem.
pub struct RubyDetector;

impl StackDetector for RubyDetector {
    fn id(&self) -> &'static str {
        "ruby"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "Gemfile")
    }

    fn detect(&self, _hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error> {
        let names = read_dir_names(project_dir)?;
        if !has_name(&names, "Gemfile") {
            return Ok(Stack::default());
        }
        let mut frameworks = Vec::new();
        // A symlinked `config` directory is not followed: the Rails check must
        // not read outside the project root.
        let config = project_dir.join("config");
        let is_config_dir = std::fs::symlink_metadata(&config)
            .map(|meta| meta.is_dir())
            .unwrap_or(false);
        if is_config_dir {
            let config_names = read_dir_names(&config)?;
            if has_name(&config_names, "application.rb") {
                frameworks.push("Rails".to_string());
            }
        }
        Ok(Stack {
            language: None,
            frameworks,
            markers: Vec::new(),
        })
    }
}
