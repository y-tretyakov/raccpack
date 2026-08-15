use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use raccpack_core::{SecretExitPolicy, SensitiveRisk};

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
    /// Find and classify sensitive files
    Dig(DigArgs),
    /// Archive a project tree into the den
    Pack(PackArgs),
    /// Stash sensitive files into an age-encrypted archive in the den
    Stash(StashArgs),
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

/// Options specific to `racc dig`.
#[derive(Debug, Args, Default)]
pub struct DigArgs {
    /// Limit dig to one project directory
    #[arg(long, value_name = "PATH")]
    pub project: Option<PathBuf>,

    /// Disable content scan (filename only)
    #[arg(long)]
    pub no_content: bool,

    /// Find repeated secrets across files
    #[arg(long)]
    pub repeated: bool,

    /// Exit policy for sensitive findings
    #[arg(long, value_name = "POLICY", value_enum)]
    pub fail_on: Option<FailOnPolicy>,

    /// Override max depth
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,
}

/// Options specific to `racc pack`.
#[derive(Debug, Args, Default)]
pub struct PackArgs {
    /// Project directory to pack (required)
    #[arg(long, value_name = "PATH")]
    pub project: PathBuf,

    /// Commit mode: write the archive into the den
    #[arg(long)]
    pub yes: bool,

    /// Force dry-run even when --yes is also given
    #[arg(long)]
    pub dry_run: bool,

    /// Disable content-based secret deny (name deny stays on)
    #[arg(long)]
    pub no_content_deny: bool,

    /// Zstd compression level (crate default when absent)
    #[arg(long, value_name = "N")]
    pub zstd_level: Option<u32>,

    /// Override the artifact file name (without .tar.zst)
    #[arg(long, value_name = "NAME")]
    pub output_name: Option<String>,
}

/// Options specific to `racc stash`.
#[derive(Debug, Args, Default)]
pub struct StashArgs {
    /// Project or subtree to stash (required)
    #[arg(long, value_name = "PATH")]
    pub project: PathBuf,

    /// Commit mode: write the encrypted archive into the den
    #[arg(long)]
    pub yes: bool,

    /// Force dry-run even when --yes is also given
    #[arg(long)]
    pub dry_run: bool,

    /// Delete the original files after a successful commit
    #[arg(long)]
    pub remove_sources: bool,

    /// Minimum risk level to include
    #[arg(long, value_name = "LEVEL", value_enum, default_value = "high")]
    pub min_risk: RiskLevel,

    /// Only these files (repeatable, must be under the project)
    #[arg(long, value_name = "PATH")]
    pub only: Vec<PathBuf>,

    /// Optional name fragment replacing the timestamp in the artifact name
    #[arg(long, value_name = "ID")]
    pub batch_id: Option<String>,
}

/// Minimum risk level selected via `--min-risk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RiskLevel {
    /// Include LOW and above
    #[value(name = "low")]
    Low,
    /// Include MEDIUM and above
    #[value(name = "medium")]
    Medium,
    /// Include HIGH and above (default)
    #[value(name = "high")]
    High,
    /// Include CRITICAL only
    #[value(name = "critical")]
    Critical,
}

impl Default for RiskLevel {
    /// Default to HIGH, matching the core `StashOptions` default.
    fn default() -> Self {
        Self::High
    }
}

impl RiskLevel {
    /// Map the CLI risk level to the core sensitive risk.
    pub fn to_risk(self) -> SensitiveRisk {
        match self {
            Self::Low => SensitiveRisk::Low,
            Self::Medium => SensitiveRisk::Medium,
            Self::High => SensitiveRisk::High,
            Self::Critical => SensitiveRisk::Critical,
        }
    }
}

/// Exit policy selected via `--fail-on`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FailOnPolicy {
    /// Never fail the run because of sensitive findings
    Ignore,
    /// Fail on CRITICAL findings only (default)
    Critical,
    /// Fail on HIGH or above
    High,
}

impl FailOnPolicy {
    /// Map the CLI value to the core exit policy.
    pub fn to_exit_policy(self) -> SecretExitPolicy {
        match self {
            Self::Ignore => SecretExitPolicy::Ignore,
            Self::Critical => SecretExitPolicy::FailOnCritical,
            Self::High => SecretExitPolicy::FailOnHighOrAbove,
        }
    }
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
            _ => panic!("expected sniff command"),
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

    #[test]
    fn dig_args_default_to_false_and_none() {
        let args = DigArgs::default();
        assert!(args.project.is_none());
        assert!(!args.no_content);
        assert!(!args.repeated);
        assert!(args.fail_on.is_none());
        assert!(args.max_depth.is_none());
    }

    #[test]
    fn clap_parse_dig_with_all_flags() {
        let cli = Cli::try_parse_from([
            "racc",
            "dig",
            "--project",
            "/tmp/app",
            "--no-content",
            "--repeated",
            "--fail-on",
            "high",
            "--max-depth",
            "4",
        ])
        .expect("parse should succeed");
        match cli.command {
            Commands::Dig(args) => {
                assert_eq!(args.project, Some(PathBuf::from("/tmp/app")));
                assert!(args.no_content);
                assert!(args.repeated);
                assert_eq!(args.fail_on, Some(FailOnPolicy::High));
                assert_eq!(args.max_depth, Some(4));
            }
            _ => panic!("expected dig command"),
        }
    }

    #[test]
    fn clap_parse_dig_without_fail_on_defaults_to_none() {
        let cli =
            Cli::try_parse_from(["racc", "dig", "--root", "/tmp"]).expect("parse should succeed");
        match cli.command {
            Commands::Dig(args) => {
                assert!(args.fail_on.is_none(), "default fail_on is None");
                assert!(!args.no_content, "content scan enabled by default");
                assert!(!args.repeated, "repeated disabled by default");
            }
            _ => panic!("expected dig command"),
        }
    }

    #[test]
    fn clap_rejects_unknown_fail_on_policy() {
        let result = Cli::try_parse_from(["racc", "dig", "--fail-on", "bogus"]);
        assert!(result.is_err(), "unknown --fail-on value must be rejected");
    }

    #[test]
    fn fail_on_policy_maps_to_core_exit_policy() {
        assert_eq!(
            FailOnPolicy::Ignore.to_exit_policy(),
            SecretExitPolicy::Ignore
        );
        assert_eq!(
            FailOnPolicy::Critical.to_exit_policy(),
            SecretExitPolicy::FailOnCritical
        );
        assert_eq!(
            FailOnPolicy::High.to_exit_policy(),
            SecretExitPolicy::FailOnHighOrAbove
        );
    }

    #[test]
    fn pack_args_default_to_false_and_none() {
        let args = PackArgs::default();
        assert!(args.project.as_os_str().is_empty());
        assert!(!args.yes);
        assert!(!args.dry_run);
        assert!(!args.no_content_deny);
        assert!(args.zstd_level.is_none());
        assert!(args.output_name.is_none());
    }

    #[test]
    fn clap_parse_pack_with_all_flags() {
        let cli = Cli::try_parse_from([
            "racc",
            "pack",
            "--project",
            "/tmp/app",
            "--den",
            "/tmp/den",
            "--yes",
            "--no-content-deny",
            "--zstd-level",
            "19",
            "--output-name",
            "snapshot",
        ])
        .expect("parse should succeed");
        match cli.command {
            Commands::Pack(args) => {
                assert_eq!(args.project, PathBuf::from("/tmp/app"));
                assert!(args.yes);
                assert!(!args.dry_run);
                assert!(args.no_content_deny);
                assert_eq!(args.zstd_level, Some(19));
                assert_eq!(args.output_name.as_deref(), Some("snapshot"));
            }
            _ => panic!("expected pack command"),
        }
    }

    #[test]
    fn clap_parse_pack_dry_run_and_global_den() {
        let cli = Cli::try_parse_from([
            "racc",
            "--den",
            "/tmp/den",
            "pack",
            "--project",
            "/tmp/app",
            "--dry-run",
        ])
        .expect("parse should succeed");
        assert_eq!(cli.global.den, Some(PathBuf::from("/tmp/den")));
        match cli.command {
            Commands::Pack(args) => {
                assert!(args.dry_run);
                assert!(!args.yes);
                assert!(
                    args.zstd_level.is_none(),
                    "zstd_level stays None by default"
                );
            }
            _ => panic!("expected pack command"),
        }
    }

    #[test]
    fn clap_rejects_pack_without_project() {
        let result = Cli::try_parse_from(["racc", "pack", "--dry-run"]);
        assert!(result.is_err(), "missing --project must be rejected");
    }

    #[test]
    fn stash_args_default_to_dry_run_high() {
        let args = StashArgs::default();
        assert!(args.project.as_os_str().is_empty());
        assert!(!args.yes);
        assert!(!args.dry_run);
        assert!(!args.remove_sources);
        assert_eq!(args.min_risk, RiskLevel::High);
        assert!(args.only.is_empty());
        assert!(args.batch_id.is_none());
    }

    #[test]
    fn clap_parse_stash_with_all_flags() {
        let cli = Cli::try_parse_from([
            "racc",
            "stash",
            "--project",
            "/tmp/app",
            "--den",
            "/tmp/den",
            "--yes",
            "--remove-sources",
            "--min-risk",
            "critical",
            "--only",
            "/tmp/app/.env",
            "--only",
            "/tmp/app/id_rsa",
            "--batch-id",
            "release-42",
        ])
        .expect("parse should succeed");
        assert_eq!(cli.global.den, Some(PathBuf::from("/tmp/den")));
        match cli.command {
            Commands::Stash(args) => {
                assert_eq!(args.project, PathBuf::from("/tmp/app"));
                assert!(args.yes);
                assert!(!args.dry_run);
                assert!(args.remove_sources);
                assert_eq!(args.min_risk, RiskLevel::Critical);
                assert_eq!(
                    args.only,
                    vec![
                        PathBuf::from("/tmp/app/.env"),
                        PathBuf::from("/tmp/app/id_rsa")
                    ]
                );
                assert_eq!(args.batch_id.as_deref(), Some("release-42"));
            }
            _ => panic!("expected stash command"),
        }
    }

    #[test]
    fn clap_parse_stash_default_min_risk_is_high() {
        let cli = Cli::try_parse_from(["racc", "stash", "--project", "/tmp/app"])
            .expect("parse should succeed");
        match cli.command {
            Commands::Stash(args) => {
                assert_eq!(args.min_risk, RiskLevel::High);
                assert!(!args.yes);
                assert!(!args.dry_run);
                assert!(!args.remove_sources);
            }
            _ => panic!("expected stash command"),
        }
    }

    #[test]
    fn clap_parse_stash_min_risk_mapping() {
        for (input, expected) in [
            ("low", RiskLevel::Low),
            ("medium", RiskLevel::Medium),
            ("high", RiskLevel::High),
            ("critical", RiskLevel::Critical),
        ] {
            let cli = Cli::try_parse_from([
                "racc",
                "stash",
                "--project",
                "/tmp/app",
                "--min-risk",
                input,
            ])
            .expect("parse should succeed");
            match cli.command {
                Commands::Stash(args) => assert_eq!(args.min_risk, expected),
                _ => panic!("expected stash command"),
            }
        }
    }

    #[test]
    fn clap_parse_stash_only_repeatable() {
        let cli = Cli::try_parse_from([
            "racc",
            "stash",
            "--project",
            "/tmp/app",
            "--only",
            "a",
            "--only",
            "b",
            "--only",
            "c",
        ])
        .expect("parse should succeed");
        match cli.command {
            Commands::Stash(args) => {
                assert_eq!(args.only.len(), 3);
                assert_eq!(args.only[0], PathBuf::from("a"));
                assert_eq!(args.only[2], PathBuf::from("c"));
            }
            _ => panic!("expected stash command"),
        }
    }

    #[test]
    fn clap_parse_stash_dry_run_and_yes_both_accepted() {
        let cli = Cli::try_parse_from([
            "racc",
            "stash",
            "--project",
            "/tmp/app",
            "--yes",
            "--dry-run",
        ])
        .expect("parse should succeed");
        match cli.command {
            Commands::Stash(args) => {
                assert!(args.yes);
                assert!(args.dry_run);
            }
            _ => panic!("expected stash command"),
        }
    }

    #[test]
    fn clap_rejects_stash_without_project() {
        let result = Cli::try_parse_from(["racc", "stash", "--yes"]);
        assert!(result.is_err(), "missing --project must be rejected");
    }

    #[test]
    fn clap_rejects_unknown_min_risk_level() {
        let result = Cli::try_parse_from([
            "racc",
            "stash",
            "--project",
            "/tmp/app",
            "--min-risk",
            "bogus",
        ]);
        assert!(result.is_err(), "unknown --min-risk value must be rejected");
    }

    #[test]
    fn risk_level_maps_to_core_risk() {
        assert_eq!(RiskLevel::Low.to_risk(), SensitiveRisk::Low);
        assert_eq!(RiskLevel::Medium.to_risk(), SensitiveRisk::Medium);
        assert_eq!(RiskLevel::High.to_risk(), SensitiveRisk::High);
        assert_eq!(RiskLevel::Critical.to_risk(), SensitiveRisk::Critical);
    }
}
