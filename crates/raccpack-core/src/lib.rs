//! raccpack-core — domain and use-cases. No CLI/TUI/Desktop dependencies.

pub mod app;
pub mod archive;
pub mod cache;
pub mod clean;
pub mod config;
pub mod den;
pub mod detect;
pub mod domain;
pub mod git;
pub mod scan;
pub mod secrets;

pub use app::{
    dig, dig_with_git, exit_code_for_secrets, pack, raid, rinse, sniff, stash, AgeIdentity,
    AppContext, DigOptions, DigResult, NullProgress, OperationKind, OrchestrationMode, PackOptions,
    PackPhaseOpts, PackResult, ProgressEvent, ProgressSink, RaidOptions, RaidResult,
    RaidStageResult, RepeatedSecret, RinseOptions, RinsePhaseOpts, RinseResult, RunMode,
    SecretExitPolicy, SensitiveFile, SniffOptions, SniffResult, StashOptions, StashPhaseOpts,
    StashResult, WorkspacePaths,
};
pub use archive::{
    pack_tree, should_deny_file_in_pack, ContentDenyOptions, PackTreeOptions, PackTreeResult,
};
pub use cache::{store_sniff_cache, try_load_sniff_cache};
pub use clean::{
    find_trash_dirs, remove_trash_dir, DetectTrashOptions, StrategyDef, StrategyId, TrashDir,
    TrashMatchKind, TrashPattern, DEFAULT_STRATEGIES,
};
pub use config::{
    default_config_path, default_config_version, default_toml, init_config, migrate_to_current,
    CleanupConfig, ConfigError, InitOptions, InitResult, PathsConfig, RaccConfig, ScannerConfig,
    CURRENT_CONFIG_VERSION, DEFAULT_DEN_DIR,
};
pub use den::{
    ensure_den, manifest_relative_path, pack_relative_path, place_pack, place_secrets_archive,
    project_slug, secrets_relative_path, short_id, staging_pack_path, utc_timestamp_now,
    write_manifest, DenManifest, DenPaths, ManifestArtifacts, ManifestStage, PlacePackRequest,
    PlacePackResult, PlaceSecretsRequest, PlaceSecretsResult, DEN_VERSION, MANIFEST_SCHEMA_VERSION,
};
pub use detect::{
    candidate_to_project, clamp_confidence, detect_stack, detect_stacks, detector_registry,
    stack_from_candidate, Detection, StackNode,
};
pub use domain::{Error, Project, Result, ScanReport, SensitiveRisk, Stack};
pub use git::{
    find_repo_root, GitClient, GitFileStatus, GitState, MockGitClient, ProcessGitClient,
};
pub use scan::{
    default_markers, ensure_scan_root, find_candidates, project_size_bytes, skip::SkipPolicy,
    skip::SkipReason, walk::WalkOptions, walk_tree, CandidateOptions, MarkerDef, MarkerHit,
    MarkerKind, ProjectCandidate,
};
pub use secrets::{
    fingerprint_secret, mask_secret, match_filename, match_filename_all, scan_file_content,
    scan_filenames, scan_secrets, upgrade_risk, ContentHit, ContentMarker, ContentMatchKind,
    ContentScanLimits, FilenameMatch, FilenamePattern, FilenameScanOptions, FindingSource,
    MaskedValue, NameMatchKind, SecretScanOptions, SensitiveFinding, DEFAULT_CONTENT_MARKERS,
    DEFAULT_FILENAME_PATTERNS,
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
