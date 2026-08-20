//! Fail-fast raid runner — the A3.1 legacy orchestration.
//!
//! [`fail_fast_raid`] runs the enabled phases in fixed order
//! (**stash → rinse → pack → move**) and stops at the first failing enabled
//! phase. Artifacts already placed into the den stay; no rollback is
//! attempted. [`OrchestrationMode::FailFast`] selects this runner; it is also
//! the PR1 green-bridge delegate of the atomic runner.

use crate::app::context::AppContext;
use crate::app::pack::{pack, PackOptions, PackResult};
use crate::app::progress::ProgressSink;
use crate::app::rinse::{rinse, RinseOptions, RinseResult};
use crate::app::stash::{stash, AgeIdentity, StashOptions, StashResult};
use crate::domain::{Error, Result};

use super::progress::{emit_phase_event, plan_phases};
use super::stages::{
    disabled_stage, failed_stage, ok_stage, pack_message, rinse_message, skipped_stage,
    stash_message,
};
use super::{RaidOptions, RaidResult, SKIPPED_MESSAGE};

/// Fail-fast orchestration of `stash → rinse → pack → move` for
/// `opts.project`.
///
/// A failing enabled phase short-circuits the rest (fail-fast); the partial
/// run returns `Ok(RaidResult { success: false, .. })`. Artifacts already
/// placed into the den stay (paths in [`RaidResult::den_artifacts`]).
///
/// # Errors (preconditions only)
///
/// - Empty project path → [`Error::Other`].
/// - `opts.stash.enabled` without an identity → [`Error::Other`].
/// - `opts.stash.enabled` with [`AgeIdentity::Recipients`] → [`Error::Unsupported`].
pub(super) fn fail_fast_raid(
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
    let planned = plan_phases(opts);
    let phase_count = enabled_phase_count(opts) as u32 + 1;

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
                let message = stash_message(&result, dry_run);
                stages.push(ok_stage("stash", message.clone()));
                emit_phase_event(progress, &planned, "stash", phase_count, message);
            }
            // No secrets matched → successful no-op ("nothing to stash").
            Err(Error::StashEmpty { .. }) => {
                stages.push(ok_stage("stash", "nothing to stash"));
                emit_phase_event(progress, &planned, "stash", phase_count, "nothing to stash");
            }
            Err(err) => {
                overall_ok = false;
                let message = err.to_string();
                stages.push(failed_stage("stash", message.clone()));
                emit_phase_event(progress, &planned, "stash", phase_count, message);
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
                    let message = rinse_message(&result, dry_run);
                    stages.push(ok_stage("rinse", message.clone()));
                    emit_phase_event(progress, &planned, "rinse", phase_count, message);
                }
                Err(err) => {
                    overall_ok = false;
                    let message = err.to_string();
                    stages.push(failed_stage("rinse", message.clone()));
                    emit_phase_event(progress, &planned, "rinse", phase_count, message);
                }
            }
        } else {
            stages.push(skipped_stage("rinse"));
            emit_phase_event(progress, &planned, "rinse", phase_count, SKIPPED_MESSAGE);
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
                    let message = pack_message(&result, dry_run);
                    stages.push(ok_stage("pack", message.clone()));
                    emit_phase_event(progress, &planned, "pack", phase_count, message);
                }
                Err(err) => {
                    overall_ok = false;
                    let message = err.to_string();
                    stages.push(failed_stage("pack", message.clone()));
                    emit_phase_event(progress, &planned, "pack", phase_count, message);
                }
            }
        } else {
            stages.push(skipped_stage("pack"));
            emit_phase_event(progress, &planned, "pack", phase_count, SKIPPED_MESSAGE);
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
        emit_phase_event(progress, &planned, "move", phase_count, message);
    } else {
        stages.push(skipped_stage("move"));
        emit_phase_event(progress, &planned, "move", phase_count, SKIPPED_MESSAGE);
    }

    Ok(RaidResult {
        project_path: opts.project.clone(),
        stages,
        stash: stash_result,
        rinse: rinse_result,
        pack: pack_result,
        den_artifacts,
        success: overall_ok,
        dry_run,
        rolled_back: false,
        rollback_warnings: Vec::new(),
    })
}

/// Validate the stash identity precondition (ignored when stash is disabled).
pub(super) fn resolve_stash_identity<'a>(
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
        staging_dir: None,
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
        collect_only: false,
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
        staging_dir: None,
        exclude_files: Vec::new(),
    };
    pack(ctx, &pack_opts, progress)
}

/// Number of enabled phases (`stash`/`rinse`/`pack`); `move` is implicit and
/// `+1` equals `plan_phases(opts).len()`.
pub(super) fn enabled_phase_count(opts: &RaidOptions) -> usize {
    usize::from(opts.stash.enabled)
        + usize::from(opts.rinse.enabled)
        + usize::from(opts.pack.enabled)
}
