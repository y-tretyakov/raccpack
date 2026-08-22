//! The `sniff` subcommand: discover projects under a scan root.

use std::path::Path;

use raccpack_core::{sniff, AppContext, NullProgress, RaccConfig, RunMode, SniffOptions};

use crate::cli::{DetectModeArg, GlobalOpts, SniffArgs};
use crate::error::CliError;
use crate::output;

/// Load the config, apply CLI overrides, run the `sniff` facade, and print.
pub fn run_sniff(global: GlobalOpts, args: SniffArgs) -> Result<(), CliError> {
    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);
    let ctx = AppContext::from_config(config, RunMode::DryRun)?;

    let opts = SniffOptions {
        force_refresh: args.force_refresh,
        max_depth: args.max_depth,
        detect_mode: args.detect_mode.map(DetectModeArg::to_detect_mode),
    };
    let mut progress = NullProgress;
    let result = sniff(&ctx, &opts, &mut progress)?;

    output::print_sniff(&result, global.json)
}

/// Load the config from an explicit path or from the default resolution.
pub(crate) fn load_config(path: Option<&Path>) -> Result<RaccConfig, CliError> {
    match path {
        Some(path) => Ok(RaccConfig::load_from_path(path)?),
        None => Ok(RaccConfig::load()?),
    }
}

/// Apply `--root` and `--den` overrides to the loaded config.
pub(crate) fn apply_overrides(mut config: RaccConfig, global: &GlobalOpts) -> RaccConfig {
    if let Some(root) = &global.root {
        config = config.with_scan_root(root.clone());
    }
    if let Some(den) = &global.den {
        config = config.with_den_dir(den.clone());
    }
    config
}
