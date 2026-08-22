//! JVM ecosystem detector (Java, Kotlin, Scala/sbt).

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

use super::traits::StackDetector;
use super::types::{has_name, read_dir_names};

/// Detector for the JVM ecosystem.
pub struct JvmDetector;

impl StackDetector for JvmDetector {
    fn id(&self) -> &'static str {
        "jvm"
    }

    fn matches(&self, hits: &[MarkerHit]) -> bool {
        hits.iter().any(|hit| {
            matches!(
                hit.name.as_str(),
                "pom.xml" | "build.gradle" | "build.gradle.kts"
            )
        })
    }

    fn detect(&self, _hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error> {
        let names = read_dir_names(project_dir)?;
        let mut frameworks = Vec::new();
        if has_name(&names, "build.sbt") {
            frameworks.push("Scala/sbt".to_string());
        }
        Ok(Stack {
            language: None,
            frameworks,
            markers: Vec::new(),
        })
    }
}
