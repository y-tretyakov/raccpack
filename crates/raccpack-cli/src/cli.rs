use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Command-line interface for the `racc` binary.
#[derive(Debug, Parser)]
#[command(
    name = "racc",
    version,
    about = "Scan projects, find secrets, pack into den"
)]
pub struct Cli {
    /// Options shared by every subcommand.
    #[command(flatten)]
    pub global: GlobalOpts,
    /// The operation to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Global options accepted before or after the subcommand.
#[derive(Debug, Args, Default)]
pub struct GlobalOpts {
    /// Config file (overrides RACCPACK_CONFIG)
    #[arg(short, long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,

    /// Override scan_root (also per-command)
    #[arg(long, value_name = "PATH", global = true)]
    pub root: Option<PathBuf>,

    /// Override den_dir (optional for sniff)
    #[arg(long, value_name = "PATH", global = true)]
    pub den: Option<PathBuf>,

    /// Emit machine-readable JSON instead of a human table
    #[arg(long, global = true)]
    pub json: bool,
}

/// The operation to run.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Discover projects under scan root
    Sniff(SniffArgs),
}

/// Options specific to `racc sniff`.
#[derive(Debug, Args, Default)]
pub struct SniffArgs {
    /// Ignore the sniff cache and rescan from scratch
    #[arg(long)]
    pub force_refresh: bool,

    /// Override scanner.max_depth
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn sniff_args_default_to_false_and_none() {
        let args = SniffArgs::default();
        assert!(!args.force_refresh);
        assert_eq!(args.max_depth, None);
    }

    #[test]
    fn global_opts_default_to_none() {
        let opts = GlobalOpts::default();
        assert!(opts.config.is_none());
        assert!(opts.root.is_none());
        assert!(opts.den.is_none());
        assert!(!opts.json);
    }

    #[test]
    fn clap_parse_sniff_with_global_json() {
        let cli = Cli::try_parse_from([
            "racc",
            "--json",
            "sniff",
            "--root",
            "/tmp",
            "--force-refresh",
            "--max-depth",
            "3",
        ])
        .expect("parse should succeed");
        assert!(cli.global.json);
        assert_eq!(cli.global.root, Some(PathBuf::from("/tmp")));
        match cli.command {
            Commands::Sniff(args) => {
                assert!(args.force_refresh);
                assert_eq!(args.max_depth, Some(3));
            }
        }
    }

    #[test]
    fn clap_parse_sniff_root_before_subcommand() {
        let cli = Cli::try_parse_from(["racc", "--root", "/tmp", "sniff"])
            .expect("global root before subcommand should parse");
        assert_eq!(cli.global.root, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn command_line_is_valid_clap_definition() {
        Cli::command().debug_assert();
    }
}
