//! Node/JavaScript/TypeScript ecosystem detector.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::types::{has_name, has_prefix, has_prefix_ext, read_dir_names, StackDetector};

/// Detector for the Node/JavaScript/TypeScript ecosystem.
pub struct NodeDetector;

impl StackDetector for NodeDetector {
    fn id(&self) -> &'static str {
        "node"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| hit.name == "package.json")
    }

    fn detect(&self, _hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error> {
        let names = read_dir_names(project_dir)?;
        let mut frameworks = Vec::new();
        if has_prefix_ext(&names, "next.config.", &["js", "mjs", "ts"]) {
            frameworks.push("Next.js".to_string());
        }
        if has_prefix(&names, "nuxt.config.") {
            frameworks.push("Nuxt".to_string());
        }
        if has_name(&names, "angular.json") {
            frameworks.push("Angular".to_string());
        }
        if has_prefix(&names, "vite.config.") {
            frameworks.push("Vite".to_string());
        }
        if has_name(&names, "deno.json") {
            frameworks.push("Deno".to_string());
        }
        Ok(Stack {
            language: None,
            frameworks,
            markers: Vec::new(),
        })
    }
}
