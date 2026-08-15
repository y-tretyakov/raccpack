//! The `pack` subcommand: archive a project tree into the den.

use std::process::ExitCode;

use raccpack_core::{
    pack, AppContext, NullProgress, PackOptions, RunMode, SecretExitPolicy, WorkspacePaths,
};

use crate::cli::{GlobalOpts, PackArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_pack;

/// Load the config, apply CLI overrides, build the mode and options, run the
/// `pack` facade, and print the result. `--yes` commits; `--dry-run` wins over
/// `--yes` (spec M4.4 §4). Exit code is always 0 on success and 1 on error.
pub fn run_pack(global: GlobalOpts, args: PackArgs) -> Result<ExitCode, CliError> {
    let PackArgs {
        project,
        yes,
        dry_run,
        no_content_deny,
        zstd_level,
        output_name,
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

    let opts = PackOptions {
        project,
        output_name,
        deny_content_secrets: !no_content_deny,
        zstd_level,
    };
    let mut progress = NullProgress;
    let result = pack(&ctx, &opts, &mut progress)?;

    output_pack::print_pack(&result, opts.deny_content_secrets, global.json)?;
    Ok(ExitCode::SUCCESS)
}
