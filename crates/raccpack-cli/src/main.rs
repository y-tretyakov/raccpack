use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod error;
mod output;
mod output_pack;
mod output_stash;
mod passphrase;

use crate::cli::{Cli, Commands};
use crate::commands::run_dig;
use crate::commands::run_pack;
use crate::commands::run_sniff;
use crate::commands::run_stash;
use crate::error::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            err.report();
            err.exit_code()
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let Cli { global, command } = cli;
    match command {
        Commands::Sniff(args) => {
            run_sniff(global, args)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Dig(args) => run_dig(global, args),
        Commands::Pack(args) => {
            run_pack(global, args)?;
            Ok(ExitCode::SUCCESS)
        }
        Commands::Stash(args) => {
            run_stash(global, args)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
