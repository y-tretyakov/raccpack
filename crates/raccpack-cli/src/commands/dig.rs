//! The `dig` subcommand: find and classify sensitive files.

use std::process::ExitCode;

use raccpack_core::{
    dig, exit_code_for_secrets, AppContext, DigOptions, NullProgress, RunMode, SecretExitPolicy,
};

use crate::cli::{DigArgs, GlobalOpts};
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output;

/// Load the config, apply CLI overrides, run the `dig` facade, and map the
/// exit policy to the process exit code (0, 1 or 2).
pub fn run_dig(global: GlobalOpts, args: DigArgs) -> Result<ExitCode, CliError> {
    let mut config = load_config(global.config.as_deref())?;
    config = apply_overrides(config, &global);
    if let Some(depth) = args.max_depth {
        config.scanner.max_depth = depth;
    }
    let ctx = AppContext::from_config(config, RunMode::DryRun)?;

    let policy = args
        .fail_on
        .map_or(SecretExitPolicy::FailOnCritical, |p| p.to_exit_policy());

    let opts = DigOptions {
        project: args.project,
        find_repeated: args.repeated,
        scan_content: !args.no_content,
        use_heuristics: None,
    };
    let mut progress = NullProgress;
    let result = dig(&ctx, &opts, &mut progress)?;

    output::print_dig(&result, global.json)?;

    let code = exit_code_for_secrets(&result.files, policy);
    if code != 0 && !global.json {
        eprintln!("Sensitive findings triggered exit policy ({policy:?})");
    }
    Ok(ExitCode::from(code as u8))
}
