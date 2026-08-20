//! The `raid` subcommand: orchestrated stash → rinse → pack → move.
//!
//! Loads the config, applies CLI overrides, resolves the project path, and
//! runs the `raid` facade. `--no-stash` / `--no-rinse` / `--no-pack` disable
//! phases; `--min-risk` selects the stash floor; `--keep-sources` disables
//! stash source removal; `--no-content-deny` disables pack content deny;
//! `--fail-fast` selects [`OrchestrationMode::FailFast`] instead of the
//! default atomic mode.
//!
//! `--yes` commits; `--dry-run` wins over `--yes`. The passphrase is read via
//! [`read_passphrase`] only when the stash phase is enabled **and** the run
//! commits; otherwise a placeholder identity is used, so `--no-stash --yes`
//! never prompts.
//!
//! Exit code (contract change A3.2): **0** when the facade returns `Ok` and
//! `RaidResult.success == true`; **1** when the facade errors or the run
//! finished with `success == false` (including a rolled-back commit failure).

use std::process::ExitCode;

use raccpack_core::{
    raid, AgeIdentity, AppContext, NullProgress, OrchestrationMode, PackPhaseOpts, ProgressSink,
    RaidOptions, RinsePhaseOpts, RunMode, SecretExitPolicy, StashPhaseOpts, WorkspacePaths,
};
use zeroize::Zeroizing;

use crate::cli::{GlobalOpts, RaidArgs};
use crate::commands::paths::resolve_project_path;
use crate::commands::sniff::{apply_overrides, load_config};
use crate::error::CliError;
use crate::output_raid;
use crate::passphrase::read_passphrase;
use crate::progress::CliProgress;

/// Dry-run placeholder passphrase (same contract as `racc stash`): never used
/// for encryption because the raid facade returns before encrypting in DryRun.
const DRY_RUN_PASSPHRASE: &str = "unused-dry-run-passphrase";

/// Run the raid facade for a project and print the result.
pub fn run_raid(global: GlobalOpts, args: RaidArgs) -> Result<ExitCode, CliError> {
    let RaidArgs {
        project,
        yes,
        dry_run,
        no_stash,
        no_rinse,
        no_pack,
        min_risk,
        keep_sources,
        no_content_deny,
        fail_fast,
    } = args;

    let config = load_config(global.config.as_deref())?;
    let config = apply_overrides(config, &global);

    let project = resolve_project_path(project, global.root.as_deref());
    let commit = yes && !dry_run;
    let mode = if commit {
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

    let opts = RaidOptions {
        project,
        mode: if fail_fast {
            OrchestrationMode::FailFast
        } else {
            OrchestrationMode::Atomic
        },
        stash: StashPhaseOpts {
            enabled: !no_stash,
            min_risk: min_risk.to_risk(),
            remove_sources: !keep_sources,
        },
        rinse: RinsePhaseOpts { enabled: !no_rinse },
        pack: PackPhaseOpts {
            enabled: !no_pack,
            deny_content_secrets: !no_content_deny,
        },
    };

    // A passphrase is needed only for the stash phase in a Commit. When stash
    // is disabled or the run is a dry run, use a placeholder identity that is
    // never used for encryption.
    let identity = if commit && opts.stash.enabled {
        AgeIdentity::Passphrase(read_passphrase()?)
    } else {
        AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE.to_string()))
    };

    // Progress lines render only in human mode; --json stays silent.
    let mut progress: Box<dyn ProgressSink> = if global.json {
        Box::new(NullProgress)
    } else {
        Box::new(CliProgress)
    };
    let result = raid(&ctx, &opts, Some(&identity), &mut *progress)?;

    output_raid::print_raid(&result, global.json)?;
    if result.success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
