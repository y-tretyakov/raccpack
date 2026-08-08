use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod error;
mod output;

use crate::cli::{Cli, Commands};
use crate::commands::run_sniff;
use crate::error::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            err.report();
            err.exit_code()
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    let Cli { global, command } = cli;
    match command {
        Commands::Sniff(args) => run_sniff(global, args),
    }
}
