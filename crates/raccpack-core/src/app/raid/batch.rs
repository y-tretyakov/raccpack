//! Batch raid: discover projects under a root and raid each one.

use std::path::PathBuf;

use crate::app::context::{AppContext, WorkspacePaths};
use crate::app::progress::ProgressSink;
use crate::app::raid::{raid, RaidResult};
use crate::domain::Result;
use crate::scan::candidates::{find_candidates, CandidateOptions};
use crate::RunMode;

use super::RaidOptions;

use crate::app::stash::AgeIdentity;

/// Options for a batch raid across multiple projects.
pub struct RaidBatchOptions {
    /// Root directory containing projects to discover.
    pub root: PathBuf,
    /// Shared per-project raid config. The `project` field is overwritten per
    /// project; other fields (mode, stash/rinse/pack opts) apply to all.
    pub raid: RaidOptions,
    /// Optional filters: substring match on project name or path.
    pub only: Vec<String>,
    /// Optional cap on the number of projects to raid.
    pub limit: Option<usize>,
    /// Stop the batch after the first project failure.
    pub stop_on_project_failure: bool,
}

/// Result of a batch raid across multiple projects.
pub struct RaidBatchResult {
    pub root: PathBuf,
    pub dry_run: bool,
    pub projects_total: usize,
    pub projects_run: usize,
    pub results: Vec<RaidBatchItem>,
    pub success: bool,
}

/// A single project's outcome within a batch raid.
pub struct RaidBatchItem {
    pub project_path: PathBuf,
    pub project_name: String,
    pub outcome: RaidBatchOutcome,
}

/// Outcome for one project in a batch.
pub enum RaidBatchOutcome {
    Raided(Box<RaidResult>),
    Skipped { reason: String },
    Error { message: String },
}

/// Run a raid on every project discovered under `opts.root`.
///
/// Projects are discovered via [`find_candidates`]. Each project gets its own
/// [`raid`] call with the shared [`RaidBatchOptions::raid`] config (the
/// `project` field is overwritten per candidate).
///
/// # Errors
///
/// Returns an error if candidate discovery fails. Per-project raid errors are
/// captured in [`RaidBatchOutcome::Error`] and do not abort the batch unless
/// [`RaidBatchOptions::stop_on_project_failure`] is set.
pub fn raid_batch(
    ctx: &AppContext,
    opts: &RaidBatchOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidBatchResult> {
    let candidates = find_candidates(
        &opts.root,
        &CandidateOptions {
            max_depth: ctx.config.scanner.max_depth,
            ..CandidateOptions::default()
        },
    )?;

    let projects_total = candidates.len();

    if projects_total == 0 {
        return Ok(RaidBatchResult {
            root: opts.root.clone(),
            dry_run: ctx.mode == RunMode::DryRun,
            projects_total: 0,
            projects_run: 0,
            results: Vec::new(),
            success: true,
        });
    }

    let filtered: Vec<_> = candidates
        .into_iter()
        .filter(|c| {
            opts.only.is_empty()
                || opts
                    .only
                    .iter()
                    .any(|f| c.name.contains(f) || c.path.to_string_lossy().contains(f))
        })
        .collect();

    let limited: Vec<_> = match opts.limit {
        Some(n) => filtered.into_iter().take(n).collect(),
        None => filtered,
    };

    let projects_run = limited.len();
    let mut results = Vec::with_capacity(projects_run);
    let mut overall_success = true;

    for (i, candidate) in limited.iter().enumerate() {
        progress.emit(crate::app::progress::ProgressEvent {
            operation: crate::app::progress::OperationKind::Raid,
            phase: format!("project {}/{}: {}", i + 1, projects_run, candidate.name),
            phase_index: i as u32,
            phase_count: projects_run as u32,
            percent: 0,
            overall_percent: ((i as u32 * 100) / projects_run as u32) as u8,
            message: format!("Raiding {}", candidate.name),
            phase_complete: false,
        });

        let project_ctx = AppContext {
            config: ctx.config.clone(),
            paths: WorkspacePaths {
                scan_root: candidate.path.clone(),
                den_dir: ctx.paths.den_dir.clone(),
            },
            mode: ctx.mode,
            exit_policy: ctx.exit_policy,
        };

        let project_opts = RaidOptions {
            project: candidate.path.clone(),
            ..opts.raid.clone()
        };

        let outcome = match raid(&project_ctx, &project_opts, identity, progress) {
            Ok(result) => {
                if !result.success {
                    overall_success = false;
                }
                RaidBatchOutcome::Raided(Box::new(result))
            }
            Err(err) => {
                overall_success = false;
                RaidBatchOutcome::Error {
                    message: err.to_string(),
                }
            }
        };

        results.push(RaidBatchItem {
            project_path: candidate.path.clone(),
            project_name: candidate.name.clone(),
            outcome,
        });

        if !overall_success && opts.stop_on_project_failure {
            break;
        }
    }

    Ok(RaidBatchResult {
        root: opts.root.clone(),
        dry_run: ctx.mode == RunMode::DryRun,
        projects_total,
        projects_run: results.len(),
        results,
        success: overall_success,
    })
}
