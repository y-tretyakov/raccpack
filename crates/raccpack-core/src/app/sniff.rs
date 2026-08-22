use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::cache::{store_sniff_cache, try_load_sniff_cache};
use crate::detect::{candidate_to_project, detect_stack, DetectMode};
use crate::domain::{Error, Result, ScanReport};
use crate::scan::{
    ensure_scan_root, find_candidates, project_size_bytes, CandidateOptions, SkipPolicy,
};

use super::context::AppContext;
use super::progress::{OperationKind, ProgressEvent, ProgressSink};

/// Fingerprint of the [`SkipPolicy::default_scan`] policy used as part of the
/// cache key. Bump this when the default skip policy changes.
const POLICY_FINGERPRINT: &str = "default_scan_v1";

/// Options controlling [`sniff`].
#[derive(Debug, Clone, Default)]
pub struct SniffOptions {
    /// Force a full rescan and overwrite the cache when true.
    pub force_refresh: bool,
    /// Override `config.scanner.max_depth` when set.
    pub max_depth: Option<usize>,
    /// Override `config.detect.mode` when set.
    ///
    /// Resolution order: CLI override → config `detect.mode` →
    /// [`DetectMode::PriorityTable`] (the config field default).
    pub detect_mode: Option<DetectMode>,
}

/// Outcome of a [`sniff`] run.
// `Eq` was dropped alongside `ScanReport`'s: `Project.stack_tree` carries an
// `f32` confidence (`PartialEq` still holds).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SniffResult {
    /// The produced scan report.
    pub report: ScanReport,
    /// Whether the report was served from the versioned cache.
    pub from_cache: bool,
    /// Wall-clock duration of the run in milliseconds.
    pub duration_ms: u64,
}

/// Discover projects under `ctx.paths.scan_root`.
///
/// Never reads file contents for secrets and never writes to the den. Results
/// are cached at `$XDG_CACHE_HOME/raccpack/sniff/{hash}.json` (falling back to
/// `~/.cache/raccpack/sniff/{hash}.json`) keyed by absolute root, `max_depth`,
/// the scan policy fingerprint (see [`cache_fingerprint`]), the resolved detect
/// mode and the crate version. A fresh cache short-circuits the walk and sets
/// [`SniffResult::from_cache`]; setting [`SniffOptions::force_refresh`] forces
/// a rescan and rewrites the cache. Cache read failures are treated as a miss
/// and cache write failures never fail the run.
///
/// The detect mode resolves as CLI override → config → default; a resolved
/// [`DetectMode::CompositeDag`] fails with [`Error::DetectPipelineUnavailable`]
/// until Detect v2 (`0.4.x`) ships the pipeline.
pub fn sniff(
    ctx: &AppContext,
    opts: &SniffOptions,
    progress: &mut dyn ProgressSink,
) -> Result<SniffResult> {
    let t0 = Instant::now();

    let detect_mode = resolve_detect_mode(ctx, opts);
    if detect_mode != DetectMode::PriorityTable {
        return Err(Error::DetectPipelineUnavailable {
            mode: detect_mode.to_string(),
        });
    }

    let root = ctx.paths.scan_root.clone();
    ensure_scan_root(&root)?;

    let max_depth = opts.max_depth.unwrap_or(ctx.config.scanner.max_depth);
    let policy = SkipPolicy::default_scan();
    let fingerprint = cache_fingerprint(detect_mode);

    progress.emit(scan_event(0, "Scanning…", false));

    if !opts.force_refresh {
        if let Some(report) = try_load_sniff_cache(&root, max_depth, &fingerprint)? {
            let duration_ms = elapsed_ms(t0);
            progress.emit(scan_event(100, "Done (from cache)", true));
            info!(
                target: "raccpack_core",
                root = %root.display(),
                projects = report.projects.len(),
                from_cache = true,
                duration_ms,
                "sniff complete"
            );
            return Ok(SniffResult {
                report,
                from_cache: true,
                duration_ms,
            });
        }
    }

    let candidates = find_candidates(
        &root,
        &CandidateOptions {
            max_depth,
            policy: policy.clone(),
            ..CandidateOptions::default()
        },
    )?;
    let count = candidates.len();
    progress.emit(scan_event(40, format!("Found {count} candidates"), false));

    let mut projects = Vec::with_capacity(candidates.len());
    let mut total_size: u64 = 0;
    for candidate in candidates {
        let stack = detect_stack(&candidate.path, &candidate.markers)?;
        let size = project_size_bytes(&candidate.path, &policy, max_depth).unwrap_or_default();
        total_size = total_size.saturating_add(size);
        projects.push(candidate_to_project(candidate, stack, size));
    }

    progress.emit(scan_event(90, "Building report", false));

    let report = ScanReport {
        root: root.clone(),
        projects,
        total_size_bytes: total_size,
        schema_version: 1,
    };

    let _ = store_sniff_cache(&root, max_depth, &fingerprint, &report);

    let duration_ms = elapsed_ms(t0);
    progress.emit(scan_event(100, "Done", true));
    info!(
        target: "raccpack_core",
        root = %root.display(),
        projects = report.projects.len(),
        from_cache = false,
        duration_ms,
        "sniff complete"
    );

    Ok(SniffResult {
        report,
        from_cache: false,
        duration_ms,
    })
}

/// Resolve the detect mode: CLI override → config → enum default.
///
/// The config field itself already defaults to [`DetectMode::PriorityTable`],
/// so `unwrap_or` on the option covers the whole precedence chain.
fn resolve_detect_mode(ctx: &AppContext, opts: &SniffOptions) -> DetectMode {
    opts.detect_mode.unwrap_or(ctx.config.detect.mode)
}

/// Cache-key fingerprint for `(default scan policy, detect mode)`.
///
/// The default mode keeps the bare [`POLICY_FINGERPRINT`] string so cache file
/// paths of existing users are byte-for-byte unchanged. Non-default modes get
/// a `+detect_mode={mode}` segment so reports produced by different pipelines
/// never share a cache slot.
fn cache_fingerprint(mode: DetectMode) -> String {
    match mode {
        DetectMode::PriorityTable => POLICY_FINGERPRINT.to_string(),
        other => format!("{POLICY_FINGERPRINT}+detect_mode={}", other.as_str()),
    }
}

/// Build a progress event for the single `"scan"` phase.
fn scan_event(percent: u8, message: impl Into<String>, phase_complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Sniff,
        phase: "scan".to_string(),
        phase_index: 0,
        phase_count: 1,
        percent,
        overall_percent: percent,
        message: message.into(),
        phase_complete,
    }
}

/// Wall-clock milliseconds since `t0`, saturating at `u64::MAX`.
fn elapsed_ms(t0: Instant) -> u64 {
    u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::{NullProgress, RunMode, SecretExitPolicy, WorkspacePaths};
    use crate::config::RaccConfig;

    fn ctx_with_mode(mode: DetectMode) -> AppContext {
        let mut config = RaccConfig::default();
        config.detect.mode = mode;
        AppContext {
            config,
            paths: WorkspacePaths {
                scan_root: PathBuf::from("/tmp/scan"),
                den_dir: PathBuf::from("/tmp/den"),
            },
            mode: RunMode::DryRun,
            exit_policy: SecretExitPolicy::FailOnCritical,
        }
    }

    #[test]
    fn detect_mode_resolution_follows_precedence() {
        let default_ctx = ctx_with_mode(DetectMode::PriorityTable);
        let dag_ctx = ctx_with_mode(DetectMode::CompositeDag);

        // No override → config value.
        assert_eq!(
            resolve_detect_mode(&default_ctx, &SniffOptions::default()),
            DetectMode::PriorityTable
        );
        assert_eq!(
            resolve_detect_mode(&dag_ctx, &SniffOptions::default()),
            DetectMode::CompositeDag
        );
        // CLI override wins over the config section.
        let opts = SniffOptions {
            detect_mode: Some(DetectMode::PriorityTable),
            ..SniffOptions::default()
        };
        assert_eq!(
            resolve_detect_mode(&dag_ctx, &opts),
            DetectMode::PriorityTable
        );
    }

    #[test]
    fn composite_dag_fails_with_unavailable_error_before_any_io() {
        let missing_root = PathBuf::from("/definitely/not/a/real/scan/root");
        let mut config = RaccConfig::default();
        config.paths.scan_root = Some(missing_root.to_string_lossy().into_owned());
        let ctx = AppContext {
            paths: WorkspacePaths {
                scan_root: missing_root,
                den_dir: PathBuf::from("/tmp/den"),
            },
            config,
            mode: RunMode::DryRun,
            exit_policy: SecretExitPolicy::FailOnCritical,
        };
        let opts = SniffOptions {
            detect_mode: Some(DetectMode::CompositeDag),
            ..SniffOptions::default()
        };
        let err = sniff(&ctx, &opts, &mut NullProgress).unwrap_err();
        assert!(
            matches!(err, Error::DetectPipelineUnavailable { ref mode } if mode == "composite_dag")
        );
        assert!(err.to_string().contains("composite_dag"));
    }

    #[test]
    fn cache_fingerprint_keeps_default_path_and_splits_modes() {
        assert_eq!(
            cache_fingerprint(DetectMode::PriorityTable),
            POLICY_FINGERPRINT
        );
        assert_ne!(
            cache_fingerprint(DetectMode::CompositeDag),
            POLICY_FINGERPRINT
        );
        assert_eq!(
            cache_fingerprint(DetectMode::CompositeDag),
            "default_scan_v1+detect_mode=composite_dag"
        );
    }
}
