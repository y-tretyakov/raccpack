//! The `raid` subcommand: orchestrated stash → rinse → pack → move.
//!
//! Supports two modes:
//! - **Single**: `--project <PATH>` — raid one project (original behavior).
//! - **Batch**: `--root <PATH>` — discover projects under root, raid each.
//!
//! `--project` and `--root` are mutually exclusive; exactly one must be
//! provided (enforced by clap `conflicts_with`).
//!
//! `--yes` commits; `--dry-run` wins over `--yes`. The passphrase is read via
//! [`read_passphrase`] only when the stash phase is enabled **and** the run
//! commits; otherwise a placeholder identity is used.
//!
//! Exit code (contract change A3.2): **0** when the facade returns `Ok` and
//! `result.success == true`; **1** when the facade errors or the run finished
//! with `success == false`.

use std::process::ExitCode;

use raccpack_core::{
    raid, raid_batch, AgeIdentity, AppContext, NullProgress, OrchestrationMode, PackPhaseOpts,
    ProgressSink, RaidBatchOptions, RaidOptions, RinsePhaseOpts, RunMode, SecretExitPolicy,
    StashPhaseOpts, WorkspacePaths,
};
use zeroize::Zeroizing;

use crate::cli::{GlobalOpts, RaidArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_raid;
use crate::output_raid_batch;
use crate::passphrase::read_passphrase;
use crate::progress::CliProgress;

/// Dry-run placeholder passphrase (same contract as `racc stash`): never used
/// for encryption because the raid facade returns before encrypting in DryRun.
const DRY_RUN_PASSPHRASE: &str = "unused-dry-run-passphrase";

/// Run the raid facade for a project and print the result.
pub fn run_raid(global: GlobalOpts, args: RaidArgs) -> Result<ExitCode, CliError> {
    let RaidArgs {
        project,
        root,
        yes,
        dry_run,
        no_stash,
        no_rinse,
        no_pack,
        min_risk,
        keep_sources,
        no_content_deny,
        fail_fast,
        only,
        limit,
        stop_on_error,
    } = args;

    let commit = yes && !dry_run;
    let stash_enabled = !no_stash;

    let identity = if commit && stash_enabled {
        AgeIdentity::Passphrase(read_passphrase()?)
    } else {
        AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE.to_string()))
    };

    let mut progress: Box<dyn ProgressSink> = if global.json {
        Box::new(NullProgress)
    } else {
        Box::new(CliProgress)
    };

    match (project, root) {
        (Some(project), None) => run_single_raid(
            global,
            project,
            commit,
            no_stash,
            no_rinse,
            no_pack,
            min_risk,
            keep_sources,
            no_content_deny,
            fail_fast,
            &identity,
            &mut *progress,
        ),
        (None, Some(root)) => run_batch_raid(
            global,
            root,
            commit,
            no_stash,
            no_rinse,
            no_pack,
            min_risk,
            keep_sources,
            no_content_deny,
            fail_fast,
            only,
            limit,
            stop_on_error,
            &identity,
            &mut *progress,
        ),
        (None, None) => Err(CliError::Core(raccpack_core::Error::Other {
            message: "exactly one of --project or --root is required".into(),
        })),
        (Some(_), Some(_)) => {
            // clap prevents this, but handle gracefully
            Err(CliError::Core(raccpack_core::Error::Other {
                message: "--project and --root are mutually exclusive".into(),
            }))
        }
    }
}

/// Single-project raid (original behavior).
#[allow(clippy::too_many_arguments)]
fn run_single_raid(
    global: GlobalOpts,
    project: std::path::PathBuf,
    commit: bool,
    no_stash: bool,
    no_rinse: bool,
    no_pack: bool,
    min_risk: crate::cli::RiskLevel,
    keep_sources: bool,
    no_content_deny: bool,
    fail_fast: bool,
    identity: &AgeIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<ExitCode, CliError> {
    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);

    let project = resolve_project_path(project, global.root.as_deref());
    let mode = if commit {
        RunMode::Commit
    } else {
        RunMode::DryRun
    };

    let ctx = AppContext {
        config: config.clone(),
        paths: WorkspacePaths {
            scan_root: project.clone(),
            den_dir: config.den_dir()?,
        },
        mode,
        exit_policy: SecretExitPolicy::FailOnCritical,
    };

    let opts = RaidOptions {
        project,
        mode: if fail_fast {
            OrchestrationMode::FailFast
        } else {
            OrchestrationMode::Atomic
        },
        stash: StashPhaseOpts {
            enabled: !no_stash,
            min_risk: min_risk.to_risk(),
            remove_sources: !keep_sources,
        },
        rinse: RinsePhaseOpts { enabled: !no_rinse },
        pack: PackPhaseOpts {
            enabled: !no_pack,
            deny_content_secrets: !no_content_deny,
        },
    };

    let result = raid(&ctx, &opts, Some(identity), progress)?;

    output_raid::print_raid(&result, global.json)?;
    if result.success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

/// Batch raid: discover projects under root and raid each.
#[allow(clippy::too_many_arguments)]
fn run_batch_raid(
    global: GlobalOpts,
    root: std::path::PathBuf,
    commit: bool,
    no_stash: bool,
    no_rinse: bool,
    no_pack: bool,
    min_risk: crate::cli::RiskLevel,
    keep_sources: bool,
    no_content_deny: bool,
    fail_fast: bool,
    only: Vec<String>,
    limit: Option<usize>,
    stop_on_error: bool,
    identity: &AgeIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<ExitCode, CliError> {
    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);

    let resolved_root = resolve_project_path(root, global.root.as_deref());
    let mode = if commit {
        RunMode::Commit
    } else {
        RunMode::DryRun
    };

    let ctx = AppContext {
        config: config.clone(),
        paths: WorkspacePaths {
            scan_root: resolved_root.clone(),
            den_dir: config.den_dir()?,
        },
        mode,
        exit_policy: SecretExitPolicy::FailOnCritical,
    };

    let opts = RaidBatchOptions {
        root: resolved_root,
        raid: RaidOptions {
            project: std::path::PathBuf::new(), // overwritten per candidate
            mode: if fail_fast {
                OrchestrationMode::FailFast
            } else {
                OrchestrationMode::Atomic
            },
            stash: StashPhaseOpts {
                enabled: !no_stash,
                min_risk: min_risk.to_risk(),
                remove_sources: !keep_sources,
            },
            rinse: RinsePhaseOpts { enabled: !no_rinse },
            pack: PackPhaseOpts {
                enabled: !no_pack,
                deny_content_secrets: !no_content_deny,
            },
        },
        only,
        limit,
        stop_on_project_failure: stop_on_error,
    };

    let result = raid_batch(&ctx, &opts, Some(identity), progress)?;

    output_raid_batch::print_raid_batch(&result, global.json)?;
    if result.success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
