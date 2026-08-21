//! The `stash` subcommand: encrypt sensitive files into an age archive in the den.

use std::process::ExitCode;

use raccpack_core::{
    stash, AgeIdentity, AppContext, NullProgress, RunMode, SecretExitPolicy, StashOptions,
    WorkspacePaths,
};
use zeroize::Zeroizing;

use crate::cli::{GlobalOpts, StashArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_stash;
use crate::passphrase::read_passphrase;

/// Dry-run placeholder passphrase. It is never used for encryption: the
/// `stash` facade returns before encrypting in `RunMode::DryRun`.
const DRY_RUN_PASSPHRASE: &str = "unused-dry-run-passphrase";

/// Load the config, apply CLI overrides, build the mode and options, read the
/// passphrase (Commit only), run the `stash` facade, and print the result.
/// `--yes` commits; `--dry-run` wins over `--yes`. Exit code is always 0 on
/// success and 1 on error.
pub fn run_stash(global: GlobalOpts, args: StashArgs) -> Result<ExitCode, CliError> {
    let StashArgs {
        project,
        yes,
        dry_run,
        remove_sources,
        min_risk,
        only,
        batch_id,
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

    let opts = StashOptions {
        target: project,
        only_files: if only.is_empty() { None } else { Some(only) },
        min_risk: min_risk.to_risk(),
        remove_sources,
        batch_id,
        staging_dir: None,
    };

    // DryRun never needs a passphrase, so read it only for Commit.
    let identity = if commit {
        AgeIdentity::Passphrase(read_passphrase()?)
    } else {
        AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE.to_string()))
    };

    let mut progress = NullProgress;
    let result = stash(&ctx, &opts, &identity, &mut progress)?;

    output_stash::print_stash(&result, remove_sources, global.json)?;
    Ok(ExitCode::SUCCESS)
}
