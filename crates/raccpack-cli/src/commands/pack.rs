//! The `pack` subcommand: archive a project tree into the den.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use raccpack_core::{
    pack, AppContext, NullProgress, PackOptions, RunMode, SecretExitPolicy, WorkspacePaths,
};

use crate::cli::{GlobalOpts, PackArgs};
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

/// Resolve a relative `--project` against the optional `--root` base.
///
/// Absolute projects are returned unchanged; a relative project without
/// `--root` is left as-is and resolved against the process cwd by the caller.
fn resolve_project_path(project: PathBuf, root: Option<&Path>) -> PathBuf {
    match root {
        Some(root) if project.is_relative() => root.join(project),
        _ => project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_joins_relative_project_with_root() {
        let resolved =
            resolve_project_path(PathBuf::from("app"), Some(Path::new("/home/user/PROJS")));
        assert_eq!(resolved, PathBuf::from("/home/user/PROJS/app"));
    }

    #[test]
    fn resolve_keeps_absolute_project_ignoring_root() {
        let resolved = resolve_project_path(
            PathBuf::from("/tmp/deep/app"),
            Some(Path::new("/home/user/PROJS")),
        );
        assert_eq!(resolved, PathBuf::from("/tmp/deep/app"));
    }

    #[test]
    fn resolve_keeps_relative_project_without_root() {
        let resolved = resolve_project_path(PathBuf::from("app"), None);
        assert_eq!(resolved, PathBuf::from("app"));
    }
}
