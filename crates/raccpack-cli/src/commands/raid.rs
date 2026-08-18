//! The `raid` subcommand: orchestrated stash → rinse → pack → move.
//!
//! Loads the config, applies CLI overrides, resolves the project path, and
//! runs the `raid` facade with all phases enabled (stash min-risk High,
//! remove_sources, content deny). `--yes` commits; `--dry-run` wins over
//! `--yes`. In dry-run mode no passphrase is read (placeholder identity);
//! Commit reads it once via [`read_passphrase`].
//!
//! Exit code is **0** when the facade returns `Ok`, including phase-failure
//! runs (`RaidResult { success: false }`) — a distinct exit code for
//! `!success` is deferred to A3.4. Exit code **1** covers config, path, and
//! facade precondition errors via [`CliError`].

use std::process::ExitCode;

use raccpack_core::{
    raid, AgeIdentity, AppContext, NullProgress, ProgressSink, RaidOptions, RunMode,
    SecretExitPolicy, WorkspacePaths,
};
use zeroize::Zeroizing;

use crate::cli::{GlobalOpts, RaidArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_raid;
use crate::passphrase::read_passphrase;
use crate::progress::CliProgress;

/// Dry-run placeholder passphrase (same contract as `racc stash`): never used
/// for encryption because the raid facade returns before encrypting in DryRun.
const DRY_RUN_PASSPHRASE: &str = "unused-dry-run-passphrase";

/// Run the raid facade for a project and print the result.
pub fn run_raid(global: GlobalOpts, args: RaidArgs) -> Result<ExitCode, CliError> {
    let RaidArgs {
        project,
        yes,
        dry_run,
    } = args;

    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);

    let project = resolve_project_path(project, global.root.as_deref());
    let commit = yes && !dry_run;
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
        ..RaidOptions::default()
    };

    // DryRun never needs a passphrase, so read it only for Commit.
    let identity = if commit {
        AgeIdentity::Passphrase(read_passphrase()?)
    } else {
        AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE.to_string()))
    };

    // Progress lines render only in human mode; --json stays silent.
    let mut progress: Box<dyn ProgressSink> = if global.json {
        Box::new(NullProgress)
    } else {
        Box::new(CliProgress)
    };
    let result = raid(&ctx, &opts, Some(&identity), &mut *progress)?;

    output_raid::print_raid(&result, global.json)?;
    Ok(ExitCode::SUCCESS)
}
