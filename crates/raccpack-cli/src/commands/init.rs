//! Implementation of the `racc init` subcommand.

use raccpack_core::{default_config_path, init_config, InitOptions, InitResult};

use crate::cli::{GlobalOpts, InitArgs};
use crate::error::CliError;

/// Run the `init` subcommand.
pub fn run_init(global: GlobalOpts, args: InitArgs) -> Result<(), CliError> {
    let config_path = match global.config {
        Some(path) => path,
        None => default_config_path()?,
    };

    let scan_root = args.scan_root.or(global.root);
    let den_dir = global.den;

    let opts = InitOptions {
        config_path,
        force: args.force,
        scan_root,
        den_dir,
        ensure_den: args.ensure_den,
    };

    let result = init_config(&opts)?;

    if global.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_human_init(&result);
    }

    Ok(())
}

/// Print human-readable output after successful initialization.
fn print_human_init(result: &InitResult) {
    println!("Created config file: {}", result.config_path.display());
    if let Some(den) = &result.den_dir {
        println!("Initialized den vault: {}", den.display());
    }
}
