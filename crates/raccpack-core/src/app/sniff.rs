use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::cache::{store_sniff_cache, try_load_sniff_cache};
use crate::detect::{candidate_to_project, detect_stack, DetectMode, StackNode, WorkspaceDetector};
use crate::domain::{Result, ScanReport};
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
/// The detect mode resolves as CLI override → config → default. The resolved
/// [`DetectMode::CompositeDag`] additionally fills
/// [`Project::stack_tree`](crate::Project::stack_tree) per candidate via the
/// composite pipeline (experimental), while the flat stack stays filled in
/// both modes.
pub fn sniff(
    ctx: &AppContext,
    opts: &SniffOptions,
    progress: &mut dyn ProgressSink,
) -> Result<SniffResult> {
    let t0 = Instant::now();

    let detect_mode = resolve_detect_mode(ctx, opts);

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
        // The flat stack is always filled, in both pipelines.
        let stack = detect_stack(&candidate.path, &candidate.markers)?;
        let size = project_size_bytes(&candidate.path, &policy, max_depth).unwrap_or_default();
        total_size = total_size.saturating_add(size);
        let mut project = candidate_to_project(candidate, stack, size);
        if detect_mode == DetectMode::CompositeDag {
            project.stack_tree = Some(composite_stack_tree(&project.path, max_depth, &policy)?);
        }
        projects.push(project);
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

/// Build the composite stack tree for one candidate project.
///
/// Reuses [`find_candidates`] over the candidate directory (same `max_depth`
/// and skip policy as the outer scan) — it inspects the root itself, never
/// follows symlinks and returns paths sorted ascending. The resulting
/// `(path, markers)` pairs feed [`WorkspaceDetector`]; no extra walker is
/// introduced here.
fn composite_stack_tree(
    project_root: &Path,
    max_depth: usize,
    policy: &SkipPolicy,
) -> Result<StackNode> {
    let subs = find_candidates(
        project_root,
        &CandidateOptions {
            max_depth,
            policy: policy.clone(),
            ..CandidateOptions::default()
        },
    )?;
    let pairs = subs
        .into_iter()
        .map(|candidate| (candidate.path, candidate.markers))
        .collect::<Vec<_>>();
    WorkspaceDetector::new().detect_tree(project_root, &pairs)
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
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use super::*;
    use crate::app::{NullProgress, RunMode, SecretExitPolicy, WorkspacePaths};
    use crate::config::RaccConfig;
    use serial_test::serial;

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

    /// Restores the previous `XDG_CACHE_HOME` value on drop, even on panic.
    struct CacheEnvGuard {
        previous: Option<OsString>,
    }

    impl CacheEnvGuard {
        /// Point `XDG_CACHE_HOME` at an empty directory inside `work`.
        fn set(work: &tempfile::TempDir) -> Self {
            let previous = std::env::var_os("XDG_CACHE_HOME");
            let dir = work.path().join("xdg-cache");
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_var("XDG_CACHE_HOME", &dir);
            Self { previous }
        }
    }

    impl Drop for CacheEnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
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
    #[serial]
    fn composite_dag_sniff_builds_stack_tree_with_nested_ecosystems() {
        let work = tempfile::tempdir().unwrap();
        let _cache = CacheEnvGuard::set(&work);

        // Monorepo fixture: rust root + nested node package.
        let scan_root = work.path().join("projects");
        let repo = scan_root.join("monorepo");
        std::fs::create_dir_all(repo.join("web")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"mono\"\n").unwrap();
        std::fs::write(repo.join("web").join("package.json"), "{}").unwrap();

        let ctx = AppContext {
            config: RaccConfig::default(),
            paths: WorkspacePaths {
                scan_root,
                den_dir: work.path().join("den"),
            },
            mode: RunMode::DryRun,
            exit_policy: SecretExitPolicy::FailOnCritical,
        };
        let opts = SniffOptions {
            detect_mode: Some(DetectMode::CompositeDag),
            ..SniffOptions::default()
        };

        let result = sniff(&ctx, &opts, &mut NullProgress).expect("composite_dag must run");

        // Nested projects are not collapsed: monorepo and monorepo/web are
        // both candidates; nested ones are not collapsed by design.
        let project = result
            .report
            .projects
            .iter()
            .find(|p| p.name == "monorepo")
            .expect("monorepo must be discovered");
        // Flat-stack invariant holds in composite mode too.
        assert_eq!(project.stack.language.as_deref(), Some("Rust"));

        let tree = project.stack_tree.as_ref().expect("stack_tree is filled");
        assert_eq!(tree.detection.ecosystem, "rust");
        assert_eq!(tree.detection.confidence, 1.0);
        assert_eq!(tree.detection.language.as_deref(), Some("Rust"));
        assert_eq!(tree.children.len(), 1);
        let web = &tree.children[0];
        assert_eq!(web.detection.ecosystem, "node");
        assert_eq!(web.detection.language.as_deref(), Some("JavaScript"));
        assert_eq!(
            web.detection.scope.file_name().and_then(OsStr::to_str),
            Some("web")
        );
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
