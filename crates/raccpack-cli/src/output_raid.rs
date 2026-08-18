//! Renders a `raid` use-case result as JSON or a minimal human summary.
//!
//! Phase progress lines are printed by [`crate::progress::CliProgress`] as the
//! run happens; the human form here only reports the final outcome line.

use raccpack_core::RaidResult;

use crate::error::CliError;

/// Print a raid result to stdout as JSON or as a one-line human summary.
pub fn print_raid(result: &RaidResult, json: bool) -> Result<(), CliError> {
    let text = format_raid(result, json)?;
    print!("{text}");
    Ok(())
}

/// Render a raid result as a JSON document or a `Success`/`Failed` line.
fn format_raid(result: &RaidResult, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_raid(result))
    }
}

/// The final human outcome line: `Success` or `Failed`.
fn format_human_raid(result: &RaidResult) -> String {
    if result.success {
        "Success\n".to_string()
    } else {
        "Failed\n".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use raccpack_core::RaidStageResult;

    use super::*;

    fn result(success: bool) -> RaidResult {
        RaidResult {
            project_path: PathBuf::from("/tmp/app"),
            stages: vec![RaidStageResult {
                name: "stash".to_string(),
                success,
                message: "stashed 3 files".to_string(),
                skipped: false,
            }],
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: Vec::new(),
            success,
            dry_run: true,
        }
    }

    #[test]
    fn format_human_success_and_failed_lines() {
        assert_eq!(
            format_raid(&result(true), false).expect("human format"),
            "Success\n"
        );
        assert_eq!(
            format_raid(&result(false), false).expect("human format"),
            "Failed\n"
        );
    }

    #[test]
    fn format_json_serializes_outcome_fields() {
        let json = format_raid(&result(true), true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["success"], true);
        assert_eq!(value["dry_run"], true);
        assert_eq!(value["stages"].as_array().expect("stages array").len(), 1);
        assert_eq!(value["stages"][0]["name"], "stash");
        assert_eq!(value["stages"][0]["success"], true);
    }
}
