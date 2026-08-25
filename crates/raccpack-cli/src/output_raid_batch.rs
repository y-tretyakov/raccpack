//! Renders a `raid_batch` result as JSON or human-readable text.

use raccpack_core::{RaidBatchOutcome, RaidBatchResult};

use crate::error::CliError;

/// Print a batch raid result to stdout.
pub fn print_raid_batch(result: &RaidBatchResult, json: bool) -> Result<(), CliError> {
    let text = format_raid_batch(result, json)?;
    print!("{text}");
    Ok(())
}

/// Render a batch raid result as JSON or human-readable text.
fn format_raid_batch(result: &RaidBatchResult, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_batch(result))
    }
}

/// Build the human-readable batch summary.
fn format_human_batch(result: &RaidBatchResult) -> String {
    let mut out = String::new();
    let run = result.projects_run;

    for (i, item) in result.results.iter().enumerate() {
        match &item.outcome {
            RaidBatchOutcome::Raided(raid_result) => {
                if raid_result.success {
                    out.push_str(&format!(
                        "→ [{}/{}] {} — ok\n",
                        i + 1,
                        run,
                        item.project_name
                    ));
                } else {
                    out.push_str(&format!(
                        "→ [{}/{}] {} — FAILED\n",
                        i + 1,
                        run,
                        item.project_name
                    ));
                }
                for stage in &raid_result.stages {
                    let symbol = if stage.success { "✓" } else { "✗" };
                    out.push_str(&format!("  {symbol} {}: {}\n", stage.name, stage.message));
                }
            }
            RaidBatchOutcome::Skipped { reason } => {
                out.push_str(&format!(
                    "→ [{}/{}] {} — SKIPPED: {}\n",
                    i + 1,
                    run,
                    item.project_name,
                    reason
                ));
            }
            RaidBatchOutcome::Error { message } => {
                out.push_str(&format!(
                    "→ [{}/{}] {} — ERROR: {}\n",
                    i + 1,
                    run,
                    item.project_name,
                    message
                ));
            }
        }
    }

    let (ok, failed, skipped, errors) = count_outcomes(result);
    out.push_str(&format!(
        "\nBatch: {ok} ok, {failed} failed, {skipped} skipped, {errors} errors\n"
    ));

    if !result.success {
        out.push_str("\nFailed\n");
    }

    out
}

/// Count batch outcomes by type.
fn count_outcomes(result: &RaidBatchResult) -> (usize, usize, usize, usize) {
    let mut ok = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for item in &result.results {
        match &item.outcome {
            RaidBatchOutcome::Raided(r) if r.success => ok += 1,
            RaidBatchOutcome::Raided(_) => failed += 1,
            RaidBatchOutcome::Skipped { .. } => skipped += 1,
            RaidBatchOutcome::Error { .. } => errors += 1,
        }
    }

    (ok, failed, skipped, errors)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use raccpack_core::{
        RaidBatchItem, RaidBatchOutcome, RaidBatchResult, RaidResult, RaidStageResult,
    };

    use super::*;

    fn sample_batch() -> RaidBatchResult {
        RaidBatchResult {
            root: PathBuf::from("/tmp/projs"),
            dry_run: true,
            projects_total: 4,
            projects_run: 4,
            results: vec![
                RaidBatchItem {
                    project_path: PathBuf::from("/tmp/projs/my-api"),
                    project_name: "my-api".to_string(),
                    outcome: RaidBatchOutcome::Raided(Box::new(RaidResult {
                        project_path: PathBuf::from("/tmp/projs/my-api"),
                        stages: vec![
                            RaidStageResult {
                                name: "stash".to_string(),
                                success: true,
                                message: "stashed 2 files".to_string(),
                                skipped: false,
                            },
                            RaidStageResult {
                                name: "rinse".to_string(),
                                success: true,
                                message: "cleaned".to_string(),
                                skipped: false,
                            },
                            RaidStageResult {
                                name: "pack".to_string(),
                                success: true,
                                message: "archived".to_string(),
                                skipped: false,
                            },
                        ],
                        stash: None,
                        rinse: None,
                        pack: None,
                        den_artifacts: vec![],
                        success: true,
                        dry_run: true,
                        rolled_back: false,
                        rollback_warnings: vec![],
                    })),
                },
                RaidBatchItem {
                    project_path: PathBuf::from("/tmp/projs/webapp"),
                    project_name: "webapp".to_string(),
                    outcome: RaidBatchOutcome::Raided(Box::new(RaidResult {
                        project_path: PathBuf::from("/tmp/projs/webapp"),
                        stages: vec![RaidStageResult {
                            name: "rinse".to_string(),
                            success: false,
                            message: "failed".to_string(),
                            skipped: false,
                        }],
                        stash: None,
                        rinse: None,
                        pack: None,
                        den_artifacts: vec![],
                        success: false,
                        dry_run: true,
                        rolled_back: false,
                        rollback_warnings: vec![],
                    })),
                },
                RaidBatchItem {
                    project_path: PathBuf::from("/tmp/projs/skipped-proj"),
                    project_name: "skipped-proj".to_string(),
                    outcome: RaidBatchOutcome::Skipped {
                        reason: "filter mismatch".to_string(),
                    },
                },
                RaidBatchItem {
                    project_path: PathBuf::from("/tmp/projs/error-proj"),
                    project_name: "error-proj".to_string(),
                    outcome: RaidBatchOutcome::Error {
                        message: "permission denied".to_string(),
                    },
                },
            ],
            success: false,
        }
    }

    #[test]
    fn format_human_lists_projects_with_status() {
        let text = format_raid_batch(&sample_batch(), false).expect("human format");
        assert!(text.contains("→ [1/4] my-api — ok"));
        assert!(text.contains("✓ stash:"));
        assert!(text.contains("✓ rinse:"));
        assert!(text.contains("✓ pack:"));
        assert!(text.contains("→ [2/4] webapp — FAILED"));
        assert!(text.contains("✗ rinse:"));
        assert!(text.contains("→ [3/4] skipped-proj — SKIPPED:"));
        assert!(text.contains("→ [4/4] error-proj — ERROR:"));
    }

    #[test]
    fn format_human_summary_line() {
        let text = format_raid_batch(&sample_batch(), false).expect("human format");
        assert!(text.contains("Batch: 1 ok, 1 failed, 1 skipped, 1 errors"));
    }

    #[test]
    fn format_human_failed_footer() {
        let text = format_raid_batch(&sample_batch(), false).expect("human format");
        assert!(text.contains("\nFailed\n"));
    }

    #[test]
    fn format_human_success_batch_has_no_failed_footer() {
        let mut batch = sample_batch();
        batch.success = true;
        batch.results[1].outcome = RaidBatchOutcome::Raided(Box::new(RaidResult {
            project_path: PathBuf::from("/tmp/projs/webapp"),
            stages: vec![RaidStageResult {
                name: "stash".to_string(),
                success: true,
                message: "ok".to_string(),
                skipped: false,
            }],
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: vec![],
            success: true,
            dry_run: true,
            rolled_back: false,
            rollback_warnings: vec![],
        }));
        batch.results[2].outcome = RaidBatchOutcome::Raided(Box::new(RaidResult {
            project_path: PathBuf::from("/tmp/projs/skipped-proj"),
            stages: vec![],
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: vec![],
            success: true,
            dry_run: true,
            rolled_back: false,
            rollback_warnings: vec![],
        }));
        batch.results[3].outcome = RaidBatchOutcome::Raided(Box::new(RaidResult {
            project_path: PathBuf::from("/tmp/projs/error-proj"),
            stages: vec![],
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: vec![],
            success: true,
            dry_run: true,
            rolled_back: false,
            rollback_warnings: vec![],
        }));
        let text = format_raid_batch(&batch, false).expect("human format");
        assert!(text.contains("Batch: 4 ok, 0 failed, 0 skipped, 0 errors"));
        assert!(!text.contains("Failed"));
    }

    #[test]
    fn format_json_serializes_full_result() {
        let json = format_raid_batch(&sample_batch(), true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["projects_total"], 4);
        assert_eq!(value["projects_run"], 4);
        assert_eq!(value["success"], false);
        let results = value["results"].as_array().expect("results array");
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn count_outcomes_matches_batch() {
        let (ok, failed, skipped, errors) = count_outcomes(&sample_batch());
        assert_eq!(ok, 1);
        assert_eq!(failed, 1);
        assert_eq!(skipped, 1);
        assert_eq!(errors, 1);
    }
}
