//! Facade use-case `raid`: orchestrate the project lifecycle.
//!
//! [`raid`] runs the enabled lifecycle phases in a fixed order —
//! **stash → rinse → pack → move** — and stops at the first failing enabled
//! phase (fail-fast). It never re-implements encrypt/clean/pack logic: each
//! phase delegates to its own facade ([`crate::app::stash`],
//! [`crate::app::rinse`], [`crate::app::pack`]) and only collects the outcome
//! into a [`RaidStageResult`].
//!
//! INVARIANTS:
//!
//! - **Fail-fast**: an `Err` from an enabled phase aborts the run; following
//!   enabled phases are recorded as `skipped` ("not run due to prior failure").
//!   Artifacts already created by earlier phases stay in the den and their
//!   paths are reported in [`RaidResult::den_artifacts`] — never rolled back.
//! - **Phase failure → `Ok(RaidResult { success: false, .. })`**, not `Err`:
//!   the partial run is a valid result for UX/JSON. Only **precondition**
//!   errors return `Err` (missing project path, missing identity when stash is
//!   enabled, unsupported recipients identity).
//! - **DryRun safe**: every sub-call runs under `ctx.mode`; nothing is created
//!   under the den and `remove_sources` never deletes. Stage success means the
//!   plan was built without an error.
//! - **Identity**: required only when `opts.stash.enabled`; when stash is
//!   disabled the identity argument is ignored entirely.
//! - **Raw-free**: stage messages carry summaries only (counts, "disabled",
//!   "not run…"), never raw secret material.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result, SensitiveRisk};

use super::context::AppContext;
use super::pack::{pack, PackOptions, PackResult};
use super::progress::{OperationKind, ProgressEvent, ProgressSink};
use super::rinse::{rinse, RinseOptions, RinseResult};
use super::stash::{stash, AgeIdentity, StashOptions, StashResult};

/// Phase-level options for the stash part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashPhaseOpts {
    /// Whether the stash phase runs.
    pub enabled: bool,
    /// Minimum risk to include (default [`SensitiveRisk::High`]).
    pub min_risk: SensitiveRisk,
    /// Delete source files after a successful Commit.
    pub remove_sources: bool,
}

/// Phase-level options for the rinse part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RinsePhaseOpts {
    /// Whether the rinse phase runs.
    pub enabled: bool,
}

/// Phase-level options for the pack part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPhaseOpts {
    /// Whether the pack phase runs.
    pub enabled: bool,
    /// Skip files whose content is a Critical secret (default true).
    pub deny_content_secrets: bool,
}

/// Options controlling a full [`raid`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidOptions {
    /// Project to process; all phases operate on this tree.
    pub project: PathBuf,
    /// Stash phase options.
    pub stash: StashPhaseOpts,
    /// Rinse phase options.
    pub rinse: RinsePhaseOpts,
    /// Pack phase options.
    pub pack: PackPhaseOpts,
}

impl Default for RaidOptions {
    fn default() -> Self {
        Self {
            project: PathBuf::new(), // caller must set
            stash: StashPhaseOpts {
                enabled: true,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        }
    }
}

/// Outcome of a single logical phase within a [`raid`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidStageResult {
    /// Phase name: `"stash"` | `"rinse"` | `"pack"` | `"move"`.
    pub name: String,
    /// Whether the phase succeeded (`false` for disabled-ok and skipped).
    pub success: bool,
    /// Human-readable summary, never containing secret material.
    pub message: String,
    /// Whether the phase did not run (disabled or short-circuited).
    pub skipped: bool,
}

/// Full outcome of a [`raid`] orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidResult {
    /// The project path the run operated on.
    pub project_path: PathBuf,
    /// Stage outcomes in run order (successful / failed / skipped).
    pub stages: Vec<RaidStageResult>,
    /// Stash sub-result when the stash phase ran.
    pub stash: Option<StashResult>,
    /// Rinse sub-result when the rinse phase ran.
    pub rinse: Option<RinseResult>,
    /// Pack sub-result when the pack phase ran.
    pub pack: Option<PackResult>,
    /// Paths of artifacts placed into the den (empty in a dry run).
    pub den_artifacts: Vec<PathBuf>,
    /// `true` iff every enabled phase succeeded.
    pub success: bool,
    /// Whether the run was a dry run (nothing written).
    pub dry_run: bool,
}

/// Orchestrate `stash → rinse → pack → move` for `opts.project`.
///
/// Runs the enabled phases in the fixed order above. A failing enabled phase
/// short-circuits the remaining enabled phases (fail-fast); the partial result
/// is returned as `Ok(RaidResult { success: false, .. })` with the failure and
/// the short-circuited stages recorded. Artifacts already placed into the den
/// by earlier successful phases are kept (their paths appear in
/// [`RaidResult::den_artifacts`]).
///
/// # Errors (preconditions only)
///
/// - Empty project path → [`Error::Other`].
/// - `opts.stash.enabled` without an identity → [`Error::Other`].
/// - `opts.stash.enabled` with [`AgeIdentity::Recipients`] → [`Error::Unsupported`].
///
/// Phase failures are **not** errors: they surface as `RaidResult.success == false`.
pub fn raid(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidResult> {
    if opts.project.as_os_str().is_empty() {
        return Err(Error::Other {
            message: "raid requires a project path".to_string(),
        });
    }

    let stash_identity = resolve_stash_identity(opts, identity)?;

    let dry_run = ctx.mode.is_dry_run();
    let phase_count = enabled_phase_count(opts) + 1;

    progress.emit(raid_event(
        0,
        phase_count as u32,
        0,
        "Starting raid…",
        false,
    ));

    let mut stages = Vec::new();
    let mut den_artifacts = Vec::new();
    let mut stash_result = None;
    let mut rinse_result = None;
    let mut pack_result = None;
    let mut overall_ok = true;

    if let Some(identity) = stash_identity {
        match run_stash_phase(ctx, opts, identity, progress) {
            Ok(result) => {
                stash_result = Some(result.clone());
                if !dry_run {
                    den_artifacts.push(result.archive_path.clone());
                }
                stages.push(ok_stage("stash", stash_message(&result, dry_run)));
            }
            Err(err) => {
                overall_ok = false;
                stages.push(failed_stage("stash", err.to_string()));
            }
        }
    } else {
        stages.push(disabled_stage("stash"));
    }

    if opts.rinse.enabled {
        if overall_ok {
            match run_rinse_phase(ctx, opts, progress) {
                Ok(result) => {
                    rinse_result = Some(result.clone());
                    stages.push(ok_stage("rinse", rinse_message(&result, dry_run)));
                }
                Err(err) => {
                    overall_ok = false;
                    stages.push(failed_stage("rinse", err.to_string()));
                }
            }
        } else {
            stages.push(skipped_stage("rinse"));
        }
    } else {
        stages.push(disabled_stage("rinse"));
    }

    if opts.pack.enabled {
        if overall_ok {
            match run_pack_phase(ctx, opts, progress) {
                Ok(result) => {
                    pack_result = Some(result.clone());
                    if !dry_run {
                        den_artifacts.push(result.output.clone());
                    }
                    stages.push(ok_stage("pack", pack_message(&result, dry_run)));
                }
                Err(err) => {
                    overall_ok = false;
                    stages.push(failed_stage("pack", err.to_string()));
                }
            }
        } else {
            stages.push(skipped_stage("pack"));
        }
    } else {
        stages.push(disabled_stage("pack"));
    }

    if overall_ok {
        let message = if den_artifacts.is_empty() {
            "nothing to finalize"
        } else {
            "finalized staged artifacts"
        };
        stages.push(ok_stage("move", message));
    } else {
        stages.push(skipped_stage("move"));
    }

    progress.emit(raid_event(
        phase_count as u32 - 1,
        phase_count as u32,
        100,
        if overall_ok {
            "Raid completed"
        } else {
            "Raid stopped after a failed phase"
        },
        true,
    ));

    Ok(RaidResult {
        project_path: opts.project.clone(),
        stages,
        stash: stash_result,
        rinse: rinse_result,
        pack: pack_result,
        den_artifacts,
        success: overall_ok,
        dry_run,
    })
}

/// Validate the identity precondition for the stash phase.
///
/// When stash is disabled the identity is ignored (`Ok(None)`), even a
/// recipients identity — spec §4. When enabled, a missing identity or an
/// unsupported recipients identity is a precondition error.
fn resolve_stash_identity<'a>(
    opts: &RaidOptions,
    identity: Option<&'a AgeIdentity>,
) -> Result<Option<&'a AgeIdentity>> {
    if !opts.stash.enabled {
        return Ok(None);
    }
    match identity {
        Some(id @ AgeIdentity::Passphrase(_)) => Ok(Some(id)),
        Some(AgeIdentity::Recipients(_)) => Err(Error::Unsupported {
            feature: "age recipient identities".to_string(),
        }),
        None => Err(Error::Other {
            message: "stash phase requires an age identity (passphrase)".to_string(),
        }),
    }
}

/// Run the stash phase, building its options from the raid options.
fn run_stash_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: &AgeIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<StashResult> {
    let stash_opts = StashOptions {
        target: opts.project.clone(),
        only_files: None,
        min_risk: opts.stash.min_risk,
        remove_sources: opts.stash.remove_sources,
        batch_id: None,
    };
    stash(ctx, &stash_opts, identity, progress)
}

/// Run the rinse phase, building its options from the raid options.
fn run_rinse_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    progress: &mut dyn ProgressSink,
) -> Result<RinseResult> {
    let rinse_opts = RinseOptions {
        target: opts.project.clone(),
        strategies: None,
        include_custom_patterns: false,
    };
    rinse(ctx, &rinse_opts, progress)
}

/// Run the pack phase, building its options from the raid options.
fn run_pack_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    progress: &mut dyn ProgressSink,
) -> Result<PackResult> {
    let pack_opts = PackOptions {
        project: opts.project.clone(),
        output_name: None,
        deny_content_secrets: opts.pack.deny_content_secrets,
        zstd_level: None,
    };
    pack(ctx, &pack_opts, progress)
}

/// Number of enabled phases (`stash` / `rinse` / `pack`); `move` is implicit.
fn enabled_phase_count(opts: &RaidOptions) -> usize {
    usize::from(opts.stash.enabled)
        + usize::from(opts.rinse.enabled)
        + usize::from(opts.pack.enabled)
}

/// Build a successful stage.
fn ok_stage(name: &str, message: impl Into<String>) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: true,
        message: message.into(),
        skipped: false,
    }
}

/// Build a failed stage (the phase ran and errored).
fn failed_stage(name: &str, message: impl Into<String>) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: false,
        message: message.into(),
        skipped: false,
    }
}

/// Build a stage for a phase short-circuited by an earlier failure.
fn skipped_stage(name: &str) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: false,
        message: "not run due to prior failure".to_string(),
        skipped: true,
    }
}

/// Build a stage for a disabled phase.
fn disabled_stage(name: &str) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: true,
        message: "disabled".to_string(),
        skipped: true,
    }
}

/// Stash stage summary, mode-aware ("would stash N files" / "stashed N files").
fn stash_message(result: &StashResult, dry_run: bool) -> String {
    if dry_run {
        format!("would stash {} files", result.files_archived)
    } else {
        format!("stashed {} files", result.files_archived)
    }
}

/// Rinse stage summary, mode-aware.
fn rinse_message(result: &RinseResult, dry_run: bool) -> String {
    if dry_run {
        format!("found {} directories", result.removed.len())
    } else {
        format!("removed {} directories", result.removed.len())
    }
}

/// Pack stage summary, mode-aware.
fn pack_message(result: &PackResult, dry_run: bool) -> String {
    if dry_run {
        "would pack project".to_string()
    } else {
        format!("packed {} files", result.file_count)
    }
}

/// Build a progress event for the `"raid"` phase.
fn raid_event(
    phase_index: u32,
    phase_count: u32,
    percent: u8,
    message: impl Into<String>,
    phase_complete: bool,
) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Raid,
        phase: "raid".to_string(),
        phase_index,
        phase_count,
        percent,
        overall_percent: percent,
        message: message.into(),
        phase_complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid_default_options_have_all_phases_enabled() {
        let opts = RaidOptions::default();
        assert!(opts.stash.enabled);
        assert!(opts.rinse.enabled);
        assert!(opts.pack.enabled);
        assert_eq!(opts.stash.min_risk, SensitiveRisk::High);
        assert!(opts.stash.remove_sources);
        assert!(opts.pack.deny_content_secrets);
        assert!(opts.project.as_os_str().is_empty());
    }

    #[test]
    fn enabled_phase_count_ignores_disabled() {
        let all = RaidOptions::default();
        assert_eq!(enabled_phase_count(&all), 3);

        let pack_only = RaidOptions {
            project: PathBuf::from("/tmp/p"),
            stash: StashPhaseOpts {
                enabled: false,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: false },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        };
        assert_eq!(enabled_phase_count(&pack_only), 1);
    }

    #[test]
    fn stage_helpers_produce_expected_shapes() {
        let ok = ok_stage("pack", "packed 3 files");
        assert!(ok.success);
        assert!(!ok.skipped);
        assert_eq!(ok.message, "packed 3 files");

        let failed = failed_stage("stash", "no files");
        assert!(!failed.success);
        assert!(!failed.skipped);
        assert_eq!(failed.message, "no files");

        let skipped = skipped_stage("rinse");
        assert!(!skipped.success);
        assert!(skipped.skipped);
        assert_eq!(skipped.message, "not run due to prior failure");

        let disabled = disabled_stage("pack");
        assert!(disabled.success);
        assert!(disabled.skipped);
        assert_eq!(disabled.message, "disabled");
    }

    #[test]
    fn raid_event_helper_shape() {
        let event = raid_event(0, 4, 0, "Starting raid…", false);
        assert_eq!(event.operation, OperationKind::Raid);
        assert_eq!(event.phase, "raid");
        assert_eq!(event.phase_index, 0);
        assert_eq!(event.phase_count, 4);
        assert_eq!(event.percent, 0);
        assert!(!event.phase_complete);

        let done = raid_event(3, 4, 100, "Raid completed", true);
        assert_eq!(done.phase_index, 3);
        assert!(done.phase_complete);
    }
}
