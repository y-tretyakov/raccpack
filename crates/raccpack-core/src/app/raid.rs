use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result, SensitiveRisk};

use super::context::AppContext;
use super::pack::{pack, PackOptions, PackResult};
use super::progress::{OperationKind, ProgressEvent, ProgressSink};
use super::rinse::{rinse, RinseOptions, RinseResult};
use super::stash::{stash, AgeIdentity, StashOptions, StashResult};

/// Stage-level options for the stash part of a raid run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashPhaseOpts {
    pub enabled: bool,
    pub min_risk: SensitiveRisk,
    pub remove_sources: bool,
}

/// Stage-level options for the rinse part of a raid run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RinsePhaseOpts {
    pub enabled: bool,
}

/// Stage-level options for the pack part of a raid run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackPhaseOpts {
    pub enabled: bool,
    pub deny_content_secrets: bool,
}

/// Options controlling a full raid run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidOptions {
    pub project: PathBuf,
    pub stash: StashPhaseOpts,
    pub rinse: RinsePhaseOpts,
    pub pack: PackPhaseOpts,
}

impl Default for RaidOptions {
    fn default() -> Self {
        Self {
            project: PathBuf::new(),
            stash: StashPhaseOpts {
                enabled: true,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        }
    }
}

/// Result of one logical phase within a raid orchestrated run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidStageResult {
    pub name: String,
    pub success: bool,
    pub message: String,
    pub skipped: bool,
}

/// Full result of a raid orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidResult {
    pub project_path: PathBuf,
    pub stages: Vec<RaidStageResult>,
    pub stash: Option<StashResult>,
    pub rinse: Option<RinseResult>,
    pub pack: Option<PackResult>,
    pub den_artifacts: Vec<PathBuf>,
    pub success: bool,
    pub dry_run: bool,
}

/// Orchestrate the project lifecycle: stash → rinse → pack → move.
///
/// A failed enabled phase short-circuits the rest of the run; a phase error is
/// reported in `RaidResult.stages` and the final `success` flag is `false`.
/// Precondition failures still return `Err(...)`.
pub fn raid(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidResult> {
    if opts.project.as_os_str().is_empty() {
        return Err(Error::PathNotFound {
            path: opts.project.clone(),
        });
    }

    if opts.stash.enabled && identity.is_none() {
        return Err(Error::Other {
            message: "stash phase requires an age identity (passphrase)".to_string(),
        });
    }

    if let Some(AgeIdentity::Recipients(_)) = identity {
        return Err(Error::Unsupported {
            feature: "age recipient identities".to_string(),
        });
    }

    let total_phases = enabled_phase_count(opts) + 1;
    progress.emit(ProgressEvent {
        operation: OperationKind::Raid,
        phase: "raid".to_string(),
        phase_index: 0,
        phase_count: total_phases as u32,
        percent: 0,
        overall_percent: 0,
        message: "Starting raid orchestration…".to_string(),
        phase_complete: false,
    });

    let mut stages = Vec::new();
    let mut den_artifacts = Vec::new();
    let mut stash_result: Option<StashResult> = None;
    let mut rinse_result: Option<RinseResult> = None;
    let mut pack_result: Option<PackResult> = None;
    let mut overall_ok = true;

    if opts.stash.enabled {
        let stash_opts = StashOptions {
            target: opts.project.clone(),
            only_files: None,
            min_risk: opts.stash.min_risk,
            remove_sources: opts.stash.remove_sources,
            batch_id: None,
        };
        let stash_id = identity.expect("stash identity checked above");
        match stash(ctx, &stash_opts, stash_id, progress) {
            Ok(result) => {
                stash_result = Some(result.clone());
                if !ctx.mode.is_dry_run() {
                    den_artifacts.push(result.archive_path.clone());
                }
                stages.push(RaidStageResult {
                    name: "stash".to_string(),
                    success: true,
                    message: "encrypted sensitive files".to_string(),
                    skipped: false,
                });
            }
            Err(err) => {
                stages.push(RaidStageResult {
                    name: "stash".to_string(),
                    success: false,
                    message: err.to_string(),
                    skipped: false,
                });
                overall_ok = false;
            }
        }
    } else {
        stages.push(RaidStageResult {
            name: "stash".to_string(),
            success: true,
            message: "disabled".to_string(),
            skipped: true,
        });
    }

    if overall_ok && opts.rinse.enabled {
        let rinse_opts = RinseOptions {
            target: opts.project.clone(),
            strategies: None,
            include_custom_patterns: false,
        };
        match rinse(ctx, &rinse_opts, progress) {
            Ok(result) => {
                rinse_result = Some(result.clone());
                stages.push(RaidStageResult {
                    name: "rinse".to_string(),
                    success: true,
                    message: format!("removed {} directories", result.removed.len()),
                    skipped: false,
                });
            }
            Err(err) => {
                stages.push(RaidStageResult {
                    name: "rinse".to_string(),
                    success: false,
                    message: err.to_string(),
                    skipped: false,
                });
                overall_ok = false;
            }
        }
    } else if opts.rinse.enabled {
        stages.push(RaidStageResult {
            name: "rinse".to_string(),
            success: false,
            message: "not run due to prior failure".to_string(),
            skipped: true,
        });
    } else {
        stages.push(RaidStageResult {
            name: "rinse".to_string(),
            success: true,
            message: "disabled".to_string(),
            skipped: true,
        });
    }

    if overall_ok && opts.pack.enabled {
        let pack_opts = PackOptions {
            project: opts.project.clone(),
            output_name: None,
            deny_content_secrets: opts.pack.deny_content_secrets,
            zstd_level: None,
        };
        match pack(ctx, &pack_opts, progress) {
            Ok(result) => {
                pack_result = Some(result.clone());
                if !ctx.mode.is_dry_run() {
                    den_artifacts.push(result.output.clone());
                }
                stages.push(RaidStageResult {
                    name: "pack".to_string(),
                    success: true,
                    message: format!("packed {} files", result.file_count),
                    skipped: false,
                });
            }
            Err(err) => {
                stages.push(RaidStageResult {
                    name: "pack".to_string(),
                    success: false,
                    message: err.to_string(),
                    skipped: false,
                });
                overall_ok = false;
            }
        }
    } else if opts.pack.enabled {
        stages.push(RaidStageResult {
            name: "pack".to_string(),
            success: false,
            message: "not run due to prior failure".to_string(),
            skipped: true,
        });
    } else {
        stages.push(RaidStageResult {
            name: "pack".to_string(),
            success: true,
            message: "disabled".to_string(),
            skipped: true,
        });
    }

    if overall_ok {
        stages.push(RaidStageResult {
            name: "move".to_string(),
            success: true,
            message: if den_artifacts.is_empty() {
                "nothing to finalize".to_string()
            } else {
                "finalized staged artifacts".to_string()
            },
            skipped: false,
        });
    } else {
        stages.push(RaidStageResult {
            name: "move".to_string(),
            success: false,
            message: "not run due to earlier failure".to_string(),
            skipped: true,
        });
    }

    progress.emit(ProgressEvent {
        operation: OperationKind::Raid,
        phase: "raid".to_string(),
        phase_index: total_phases as u32 - 1,
        phase_count: total_phases as u32,
        percent: 100,
        overall_percent: 100,
        message: if overall_ok {
            "Raid completed successfully".to_string()
        } else {
            "Raid stopped after a failed phase".to_string()
        },
        phase_complete: true,
    });

    Ok(RaidResult {
        project_path: opts.project.clone(),
        stages,
        stash: stash_result,
        rinse: rinse_result,
        pack: pack_result,
        den_artifacts,
        success: overall_ok,
        dry_run: ctx.mode.is_dry_run(),
    })
}

fn enabled_phase_count(opts: &RaidOptions) -> usize {
    let mut count = 0usize;
    if opts.stash.enabled {
        count += 1;
    }
    if opts.rinse.enabled {
        count += 1;
    }
    if opts.pack.enabled {
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use zeroize::Zeroizing;

    use super::*;
    use crate::app::{AppContext, NullProgress, RunMode};
    use crate::config::RaccConfig;

    fn test_context(project: &std::path::Path, den: &std::path::Path, mode: RunMode) -> AppContext {
        let config = RaccConfig::default().with_scan_root(project).with_den_dir(den);
        AppContext::from_config(config, mode).expect("valid test context")
    }

    #[test]
    fn raid_default_options_have_all_phases_enabled() {
        let opts = RaidOptions::default();
        assert!(opts.stash.enabled);
        assert!(opts.rinse.enabled);
        assert!(opts.pack.enabled);
        assert_eq!(opts.stash.min_risk, SensitiveRisk::High);
        assert!(opts.stash.remove_sources);
        assert!(opts.pack.deny_content_secrets);
    }

    #[test]
    fn raid_dry_run_all_enabled_is_successful_and_writes_nothing() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(".env"), "API_KEY=super-secret-value\n").unwrap();
        fs::write(project.join("README.md"), "# demo\n").unwrap();

        let den = root.path().join("den");
        let ctx = test_context(&project, &den, RunMode::DryRun);
        let identity = Some(AgeIdentity::Passphrase(Zeroizing::new("s3cr3t".to_string())));
        let opts = RaidOptions {
            project: project.clone(),
            ..RaidOptions::default()
        };

        let mut progress = NullProgress;
        let result = raid(&ctx, &opts, identity.as_ref(), &mut progress).unwrap();

        assert!(result.success);
        assert!(result.dry_run);
        assert!(result.den_artifacts.is_empty());
        assert!(!result.stages.is_empty());
        assert!(result.stages.iter().any(|stage| stage.name == "move"));
    }

    #[test]
    fn raid_stash_failure_short_circuits_following_phases() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("README.md"), "# demo\n").unwrap();

        let den = root.path().join("den");
        let ctx = test_context(&project, &den, RunMode::Commit);
        let identity = Some(AgeIdentity::Passphrase(Zeroizing::new(String::new())));
        let opts = RaidOptions {
            project: project.clone(),
            stash: StashPhaseOpts {
                enabled: true,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        };

        let mut progress = NullProgress;
        let result = raid(&ctx, &opts, identity.as_ref(), &mut progress).unwrap();

        assert!(!result.success);
        assert!(result.stages.iter().any(|stage| stage.name == "stash" && !stage.success));
        assert!(result.stages.iter().any(|stage| stage.name == "move"));
        assert!(result.stages.iter().all(|stage| stage.name != "pack" || stage.skipped || !stage.success));
    }

    #[test]
    fn raid_without_stash_identity_and_disabled_stash_still_runs() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("README.md"), "# demo\n").unwrap();

        let den = root.path().join("den");
        let ctx = test_context(&project, &den, RunMode::Commit);

        let opts = RaidOptions {
            project: project.clone(),
            stash: StashPhaseOpts {
                enabled: false,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        };

        let mut progress = NullProgress;
        let result = raid(&ctx, &opts, None, &mut progress).unwrap();
        assert!(result.success);
        assert!(result.stages.iter().any(|stage| stage.name == "stash" && stage.skipped));
    }
}
