//! raccpack-core — domain and use-cases. No CLI/TUI/Desktop dependencies.

pub mod config;
pub mod domain;
pub mod scan;

pub use config::{ConfigError, PathsConfig, RaccConfig, ScannerConfig};
pub use domain::{Error, Project, Result, ScanReport, SensitiveRisk, Stack};
pub use scan::{
    ensure_scan_root, skip::SkipPolicy, skip::SkipReason, walk::WalkOptions, walk_tree,
};

/// Placeholder to keep the crate non-empty and testable.
pub fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        assert!(!core_version().is_empty());
    }
}
