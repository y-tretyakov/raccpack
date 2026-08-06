//! raccpack-core — domain and use-cases. No CLI/TUI/Desktop dependencies.

pub mod app;
pub mod cache;
pub mod config;
pub mod detect;
pub mod domain;
pub mod scan;

pub use app::{
    sniff, AppContext, NullProgress, OperationKind, ProgressEvent, ProgressSink, RunMode,
    SecretExitPolicy, SniffOptions, SniffResult, WorkspacePaths,
};
pub use cache::{store_sniff_cache, try_load_sniff_cache};
pub use config::{ConfigError, PathsConfig, RaccConfig, ScannerConfig};
pub use detect::{candidate_to_project, detect_stack, detect_stacks, stack_from_candidate};
pub use domain::{Error, Project, Result, ScanReport, SensitiveRisk, Stack};
pub use scan::{
    default_markers, ensure_scan_root, find_candidates, project_size_bytes, skip::SkipPolicy,
    skip::SkipReason, walk::WalkOptions, walk_tree, CandidateOptions, MarkerDef, MarkerHit,
    MarkerKind, ProjectCandidate,
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
