//! Launch-argument contract for `racc-tui` (and, later, the Desktop app).
//!
//! Parsed *before* any terminal init (raw mode / alternate screen / ratatui),
//! so `--version` / `--help` and non-interactive refusal work cleanly.
//!
//! See `docs/launch-contract.md` for the shared semantic summary shared with
//! the future Desktop surface.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::app::ViewId;

/// Terminal UI for raccpack.
#[derive(Parser, Debug)]
#[command(
    name = "racc-tui",
    version,
    about = "Terminal UI for raccpack: sniff, dig, stash, pack",
    long_about = None
)]
pub struct Cli {
    /// Project scan root (default: ~/DEV/PROJS if it exists, else cwd).
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Den directory (overrides RACCPACK_DEN env var).
    #[arg(long, value_name = "PATH")]
    pub den: Option<PathBuf>,

    /// Config file (stored for later; not yet wired into worker).
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Initial view.
    #[arg(long, value_enum, default_value_t = ViewArg::Overview)]
    pub view: ViewArg,

    /// Trigger a sniff refresh on startup when landing on Projects.
    #[arg(long)]
    pub refresh: bool,

    /// Increase logging verbosity on stderr (-v: info, -vv: debug, -vvv: trace).
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Command-line view selector.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewArg {
    Overview,
    Projects,
    Findings,
    Operations,
}

impl ViewArg {
    /// Map the CLI variant to the corresponding app `ViewId`.
    pub fn to_view_id(self) -> ViewId {
        match self {
            Self::Overview => ViewId::Overview,
            Self::Projects => ViewId::Projects,
            Self::Findings => ViewId::Findings,
            Self::Operations => ViewId::Operations,
        }
    }
}

impl From<ViewId> for ViewArg {
    #[inline]
    fn from(view: ViewId) -> Self {
        match view {
            ViewId::Overview => Self::Overview,
            ViewId::Projects => Self::Projects,
            ViewId::Findings => Self::Findings,
            ViewId::Operations => Self::Operations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_arg_matches_view_id_bijection() {
        for arg in [
            ViewArg::Overview,
            ViewArg::Projects,
            ViewArg::Findings,
            ViewArg::Operations,
        ] {
            let view = arg.to_view_id();
            assert_eq!(ViewArg::from(view), arg, "ViewArg -> ViewId -> ViewArg");
            assert_eq!(ViewArg::from(view).to_view_id(), view, "round-trip");
        }
    }

    #[test]
    fn lower_names_map_to_expected_views() {
        // `--view <value>` accepts kebab/lower names matching the ViewId set.
        for (name, expected) in [
            ("overview", ViewId::Overview),
            ("projects", ViewId::Projects),
            ("findings", ViewId::Findings),
            ("operations", ViewId::Operations),
        ] {
            let cli = Cli::try_parse_from(["racc-tui", "--view", name]).expect("parses");
            assert_eq!(cli.view.to_view_id(), expected);
        }
    }

    #[test]
    fn version_and_help_flags_short_circuit() {
        // clap treats `--version`/`-V` (DisplayVersion) and `--help`/`-h`
        // (DisplayHelp) as short-circuits that print and exit — neither reaches
        // the app, so parsing yields the corresponding "display" error kinds.
        use clap::error::ErrorKind::{DisplayHelp, DisplayVersion};
        for (args, kind) in [
            (["racc-tui", "--version"], DisplayVersion),
            (["racc-tui", "-V"], DisplayVersion),
            (["racc-tui", "--help"], DisplayHelp),
            (["racc-tui", "-h"], DisplayHelp),
        ] {
            let err = Cli::try_parse_from(args).expect_err("must short-circuit");
            assert_eq!(err.kind(), kind, "{args:?} should yield {kind:?}");
        }
    }

    #[test]
    fn default_view_is_overview() {
        let cli = Cli::try_parse_from(["racc-tui"]).expect("parses");
        assert_eq!(cli.view.to_view_id(), ViewId::Overview);
    }

    #[test]
    fn no_args_produce_default_field_values() {
        let cli = Cli::try_parse_from(["racc-tui"]).expect("parses");
        assert_eq!(cli.root, None);
        assert_eq!(cli.den, None);
        assert_eq!(cli.config, None);
        assert_eq!(cli.view.to_view_id(), ViewId::Overview);
        assert!(!cli.refresh, "refresh must default to false");
        assert_eq!(cli.verbose, 0, "verbose must default to 0");
    }

    #[test]
    fn root_flag_is_captured() {
        let cli = Cli::try_parse_from(["racc-tui", "--root", "/tmp/foo"]).expect("parses");
        assert_eq!(cli.root, Some(std::path::PathBuf::from("/tmp/foo")));
    }

    #[test]
    fn den_flag_is_captured() {
        let cli = Cli::try_parse_from(["racc-tui", "--den", "/tmp/den"]).expect("parses");
        assert_eq!(cli.den, Some(std::path::PathBuf::from("/tmp/den")));
    }

    #[test]
    fn config_flag_is_captured() {
        let cli = Cli::try_parse_from(["racc-tui", "--config", "/tmp/racc.toml"]).expect("parses");
        assert_eq!(cli.config, Some(std::path::PathBuf::from("/tmp/racc.toml")));
    }

    #[test]
    fn view_projects_maps_to_viewid_projects() {
        let cli = Cli::try_parse_from(["racc-tui", "--view", "projects"]).expect("parses");
        assert_eq!(cli.view, ViewArg::Projects);
        assert_eq!(cli.view.to_view_id(), ViewId::Projects);
    }

    #[test]
    fn invalid_view_value_is_rejected() {
        let result = Cli::try_parse_from(["racc-tui", "--view", "not-a-view"]);
        assert!(result.is_err(), "--view not-a-view must not parse");
    }

    #[test]
    fn refresh_flag_is_captured() {
        let cli = Cli::try_parse_from(["racc-tui", "--refresh"]).expect("parses");
        assert!(cli.refresh, "--refresh must set refresh to true");
    }

    #[test]
    fn verbose_count_captures_repetitions() {
        let cli = Cli::try_parse_from(["racc-tui", "-v"]).expect("parses");
        assert_eq!(cli.verbose, 1);
        let cli = Cli::try_parse_from(["racc-tui", "-vvv"]).expect("parses");
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn value_enum_exposes_exactly_the_four_view_names() {
        use clap::ValueEnum;
        let names: Vec<String> = ViewArg::value_variants()
            .iter()
            .map(|v| {
                v.to_possible_value()
                    .expect("possible value")
                    .get_name()
                    .to_owned()
            })
            .collect();
        assert_eq!(
            names,
            ["overview", "projects", "findings", "operations"],
            "CLI view names must match the four ViewId variants"
        );
    }
}
