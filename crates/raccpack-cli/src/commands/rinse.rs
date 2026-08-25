//! The `rinse` subcommand: remove build-artifact directories from a project.

use std::process::ExitCode;

use raccpack_core::{
    resolve_stack_tree, rinse, AppContext, NullProgress, RinseOptions, RunMode, SecretExitPolicy,
    SniffOptions, WorkspacePaths,
};

use crate::cli::{GlobalOpts, RinseArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_rinse;

/// Load the config, apply CLI overrides, build the mode and options, run the
/// `rinse` facade, and print the result. `--yes` commits; `--dry-run` wins
/// over `--yes`. Unknown `--strategy` ids fail with exit code 1 via the
/// facade. Exit code is always 0 on success and 1 on error.
pub fn run_rinse(global: GlobalOpts, args: RinseArgs) -> Result<ExitCode, CliError> {
    let RinseArgs {
        project,
        yes,
        dry_run,
        strategy,
    } = args;

    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);

    let project = resolve_project_path(project, global.root.as_deref());
    let mode = if yes && !dry_run {
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

    // Resolve DAG stack_tree via targeted sniff (no-op in priority_table mode).
    let stack_tree = resolve_stack_tree(&ctx, &project, &SniffOptions::default());

    let opts = RinseOptions {
        target: project.clone(),
        strategies: if strategy.is_empty() {
            None
        } else {
            Some(strategy)
        },
        include_custom_patterns: false,
        collect_only: false,
        stack_tree,
    };
    let mut progress = NullProgress;
    let result = rinse(&ctx, &opts, &mut progress)?;

    output_rinse::print_rinse(&result, &project, global.json)?;
    Ok(ExitCode::SUCCESS)
}
