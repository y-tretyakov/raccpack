//! Atomic raid runner — shared staging + deferred destructive ops (A3.3).
//!
//! [`atomic_raid`] runs the enabled phases in fixed order
//! (**stash → rinse → pack → move**) with every intermediate artifact written
//! under one shared `den/staging/{raid_id}/` directory and every destructive
//! op (stash `remove_sources`, rinse deletes) deferred to the `move` (commit)
//! phase. A mid-raid failure therefore leaves nothing in the den (ORPHAN-1):
//! staging is cleaned best-effort and the run reports `success: false` with
//! empty `den_artifacts`.
//!
//! The commit records every forward effect in `staging/wal.jsonl` **before**
//! applying it; a mid-commit failure rolls those effects back via
//! [`super::rollback::rollback_from_wal`], reports `rolled_back: true`, and
//! clears `den_artifacts`.
//!
//! A dry run delegates to [`super::fail_fast::fail_fast_raid`]: zero FS
//! writes, identical events (ORPHAN-3). On a successful atomic commit the
//! result matches the fail-fast commit result (same sub-results,
//! `den_artifacts` in stash→pack order, identical stages).

use std::path::{Path, PathBuf};

use crate::app::context::AppContext;
use crate::app::pack::{pack, PackOptions, PackResult};
use crate::app::progress::ProgressSink;
use crate::app::rinse::{remove_trash_dirs, rinse, RinseOptions, RinseResult};
use crate::app::stash::{stash, AgeIdentity, StashOptions, StashResult};
use crate::den::{create_dir_all, ensure_den, move_archive, short_id};
use crate::domain::{Error, Result};
use crate::secrets::remove_stash_sources;

use super::fail_fast::{enabled_phase_count, fail_fast_raid, resolve_stash_identity};
use super::progress::{emit_phase_event, emit_rollback_event, plan_phases};
use super::rollback::rollback_from_wal;
use super::stages::{
    disabled_stage, failed_stage, ok_stage, pack_message, rinse_message, skipped_stage,
    stash_message,
};
use super::staging::{raid_staging_path, remove_raid_staging};
use super::wal::{Wal, WalOp};
use super::{RaidOptions, RaidResult, SKIPPED_MESSAGE};

/// Atomic orchestration of `stash → rinse → pack → move` for `opts.project`.
///
/// # Errors (preconditions only)
///
/// - Empty project path → [`Error::Other`].
/// - `opts.stash.enabled` without an identity → [`Error::Other`].
/// - `opts.stash.enabled` with [`AgeIdentity::Recipients`] → [`Error::Unsupported`].
///
/// Phase and commit failures return `Ok(RaidResult { success: false, .. })`.
pub(super) fn atomic_raid(
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

    if ctx.mode.is_dry_run() {
        return fail_fast_raid(ctx, opts, identity, progress);
    }

    let raid_id = short_id();
    let raid_staging = raid_staging_path(&ctx.paths.den_dir, &raid_id);
    let wal_path = raid_staging.join("wal.jsonl");

    let planned = plan_phases(opts);
    let phase_count = enabled_phase_count(opts) as u32 + 1;

    let mut stages = Vec::new();
    let mut stash_result = None;
    let mut rinse_result = None;
    let mut pack_result = None;
    let mut overall_ok = true;

    if let Some(identity) = stash_identity {
        match run_atomic_stash_phase(ctx, opts, identity, &raid_staging, progress) {
            Ok(result) => {
                stash_result = Some(result.clone());
                let message = stash_message(&result, false);
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
            match run_atomic_rinse_phase(ctx, opts, progress) {
                Ok(result) => {
                    rinse_result = Some(result.clone());
                    let message = rinse_message(&result, false);
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

    let excluded = stash_result
        .as_ref()
        .map(|r| {
            r.manifest
                .iter()
                .map(|entry| entry.original_path.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if opts.pack.enabled {
        if overall_ok {
            match run_atomic_pack_phase(ctx, opts, &raid_staging, &excluded, progress) {
                Ok(result) => {
                    pack_result = Some(result.clone());
                    let message = pack_message(&result, false);
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

    if !overall_ok {
        // A phase failure leaves only staging (never the den): clean it up and
        // report no artifacts (ORPHAN-1). The WAL was never created, so no
        // rollback is needed.
        remove_raid_staging(&raid_staging);
        stages.push(skipped_stage("move"));
        emit_phase_event(progress, &planned, "move", phase_count, SKIPPED_MESSAGE);
        return Ok(RaidResult {
            project_path: opts.project.clone(),
            stages,
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: Vec::new(),
            success: false,
            dry_run: false,
            rolled_back: false,
            rollback_warnings: Vec::new(),
        });
    }

    let mut rolled_back = false;
    let mut rollback_warnings = Vec::new();
    let (den_artifacts, success) = match commit(
        ctx,
        opts,
        &raid_staging,
        &wal_path,
        &mut stash_result,
        &mut rinse_result,
        &mut pack_result,
    ) {
        Ok(artifacts) => {
            let message = if artifacts.is_empty() {
                "nothing to finalize"
            } else {
                "finalized staged artifacts"
            };
            stages.push(ok_stage("move", message));
            emit_phase_event(progress, &planned, "move", phase_count, message);
            (artifacts, true)
        }
        Err(err) => {
            // Mid-commit failure: reverse-WAL the recorded effects, drop
            // staging and the placed-artifact list, then emit failed move
            // followed by the rollback event.
            let report = rollback_from_wal(&wal_path);
            rolled_back = report.applied;
            rollback_warnings = report.warnings;
            remove_raid_staging(&raid_staging);
            let message = format!("commit failed, rolled back: {err}");
            stages.push(failed_stage("move", message.clone()));
            emit_phase_event(progress, &planned, "move", phase_count, message);
            let rollback_message = format!("rolled back ({} warnings)", rollback_warnings.len());
            emit_rollback_event(progress, phase_count, rollback_message);
            (Vec::new(), false)
        }
    };

    Ok(RaidResult {
        project_path: opts.project.clone(),
        stages,
        stash: stash_result,
        rinse: rinse_result,
        pack: pack_result,
        den_artifacts,
        success,
        dry_run: false,
        rolled_back,
        rollback_warnings,
    })
}

/// Run the stash phase into the shared raid staging, deferring placement and
/// source removal to the commit.
fn run_atomic_stash_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: &AgeIdentity,
    raid_staging: &Path,
    progress: &mut dyn ProgressSink,
) -> Result<StashResult> {
    let stash_opts = StashOptions {
        target: opts.project.clone(),
        only_files: None,
        min_risk: opts.stash.min_risk,
        remove_sources: false,
        batch_id: None,
        staging_dir: Some(raid_staging.to_path_buf()),
    };
    stash(ctx, &stash_opts, identity, progress)
}

/// Run the rinse phase in scan-only mode; deletion happens in the commit.
fn run_atomic_rinse_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    progress: &mut dyn ProgressSink,
) -> Result<RinseResult> {
    let rinse_opts = RinseOptions {
        target: opts.project.clone(),
        strategies: None,
        include_custom_patterns: false,
        collect_only: true,
    };
    rinse(ctx, &rinse_opts, progress)
}

/// Run the pack phase into the shared raid staging, deferring placement to the
/// commit.
///
/// Files already selected by the stash phase (`exclude_files`) are omitted from
/// the archive entirely — mirroring the fail-fast commit where stash removed
/// them before pack ran — so a stashed secret never re-enters the pack tree.
fn run_atomic_pack_phase(
    ctx: &AppContext,
    opts: &RaidOptions,
    raid_staging: &Path,
    exclude_files: &[PathBuf],
    progress: &mut dyn ProgressSink,
) -> Result<PackResult> {
    let pack_opts = PackOptions {
        project: opts.project.clone(),
        output_name: None,
        deny_content_secrets: opts.pack.deny_content_secrets,
        zstd_level: None,
        staging_dir: Some(raid_staging.to_path_buf()),
        exclude_files: exclude_files.to_vec(),
    };
    pack(ctx, &pack_opts, progress)
}

/// Place the staged artifacts into the den and apply the deferred destructive
/// ops, returning the paths placed into the den.
///
/// Order (PR3 policy): WAL record before every effect → stash placement → pack
/// placement → stash `remove_sources` → rinse deletes → staging cleanup. A
/// failure mid-commit triggers [`rollback_from_wal`] in the caller, so no
/// placed artifact is reported on a failed commit.
fn commit(
    ctx: &AppContext,
    opts: &RaidOptions,
    raid_staging: &Path,
    wal_path: &Path,
    stash_result: &mut Option<StashResult>,
    rinse_result: &mut Option<RinseResult>,
    pack_result: &mut Option<PackResult>,
) -> Result<Vec<PathBuf>> {
    // With nothing staged and no deferred deletes there is no effect to
    // record — skip the WAL entirely so an all-disabled commit still reports
    // a clean "move".
    let needs_wal = stash_result.is_some()
        || pack_result.is_some()
        || (opts.rinse.enabled && rinse_result.as_ref().is_some_and(|r| !r.removed.is_empty()));
    if !needs_wal {
        remove_raid_staging(raid_staging);
        return Ok(Vec::new());
    }

    // The WAL is created before any effect (including `ensure_den`): every
    // forward effect is durably recorded before it is applied.
    create_dir_all(raid_staging)?;
    let mut wal = Wal::new(wal_path)?;

    let mut placed = Vec::new();

    if stash_result.is_some() || pack_result.is_some() {
        ensure_den(&ctx.paths.den_dir)?;
    }

    if let Some(stash) = stash_result.as_ref() {
        place_staged_artifact(
            &mut wal,
            &raid_staging.join("secrets.age"),
            &stash.archive_path,
        )?;
        placed.push(stash.archive_path.clone());
    }

    if let Some(pack) = pack_result.as_ref() {
        place_staged_artifact(&mut wal, &raid_staging.join("pack.tar.zst"), &pack.output)?;
        placed.push(pack.output.clone());
    }

    if opts.stash.remove_sources {
        if let Some(stash) = stash_result.as_mut() {
            for entry in &stash.manifest {
                wal.record(&WalOp::DeleteFile {
                    path: entry.original_path.clone(),
                })?;
            }
            let removed = remove_stash_sources(&stash.manifest)?;
            stash.removed_sources = removed;
        }
    }

    if opts.rinse.enabled {
        if let Some(rinse) = rinse_result.as_mut() {
            for dir in &rinse.removed {
                wal.record(&WalOp::DeleteDir {
                    path: dir.path.clone(),
                })?;
            }
            let (removed, bytes_freed) = remove_trash_dirs(&opts.project, &rinse.removed)?;
            rinse.removed = removed;
            rinse.bytes_freed = bytes_freed;
            rinse.dry_run = false;
        }
    }

    remove_raid_staging(raid_staging);

    Ok(placed)
}

/// Move one staged artifact into the den, recording the create-dir and the
/// rename in the WAL before applying them (append+fsync precedes each effect).
fn place_staged_artifact(wal: &mut Wal, from: &Path, to: &Path) -> Result<()> {
    let parent = to.parent().ok_or_else(|| Error::Other {
        message: format!("invalid artifact path: {}", to.display()),
    })?;
    wal.record(&WalOp::CreateDir {
        path: parent.to_path_buf(),
    })?;
    create_dir_all(parent)?;
    wal.record(&WalOp::Rename {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
    })?;
    move_archive(from, to)
}
