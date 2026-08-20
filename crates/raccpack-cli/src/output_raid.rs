//! Renders a `raid` use-case result as JSON or a compact human summary.
//!
//! Phase progress lines are printed by [`crate::progress::CliProgress`] as the
//! run happens; the human form here reports the final outcome and, when
//! relevant, the placed den artifacts or the rollback warning count.

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

/// The final human outcome block: `Success` or `Failed`, plus placed den
/// artifacts (commit only) or the rollback warning count.
fn format_human_raid(result: &RaidResult) -> String {
    let mut out = String::new();
    if result.success {
        out.push_str("Success\n");
        if !result.dry_run && !result.den_artifacts.is_empty() {
            out.push_str(&format!(
                "  placed {} artifact(s):\n",
                result.den_artifacts.len()
            ));
            for path in &result.den_artifacts {
                out.push_str(&format!("    {}\n", path.display()));
            }
        }
    } else {
        out.push_str("Failed\n");
        if result.rolled_back {
            out.push_str(&format!(
                "  rolled back ({} warnings)\n",
                result.rollback_warnings.len()
            ));
        }
    }
    out
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
            rolled_back: false,
            rollback_warnings: Vec::new(),
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
    fn format_human_success_lists_placed_artifacts() {
        let mut res = result(true);
        res.dry_run = false;
        res.den_artifacts = vec![
            PathBuf::from("/tmp/den/packs/2026/08/app__t.tar.zst"),
            PathBuf::from("/tmp/den/secrets/2026/08/app__t.age"),
        ];
        let text = format_raid(&res, false).expect("human format");
        assert_eq!(
            text,
            "Success\n  placed 2 artifact(s):\n    /tmp/den/packs/2026/08/app__t.tar.zst\n    /tmp/den/secrets/2026/08/app__t.age\n"
        );
    }

    #[test]
    fn format_human_dry_run_success_omits_artifacts() {
        let mut res = result(true);
        res.den_artifacts = vec![PathBuf::from("/tmp/den/packs/2026/08/app__t.tar.zst")];
        assert_eq!(
            format_raid(&res, false).expect("human format"),
            "Success\n",
            "dry-run must not list artifacts that were never placed"
        );
    }

    #[test]
    fn format_human_failed_reports_rollback_warnings() {
        let mut res = result(false);
        res.rolled_back = true;
        res.rollback_warnings = vec![
            "could not remove /tmp/den/secrets/2026/08/x.age".to_string(),
            "irreversible delete: .env".to_string(),
        ];
        let text = format_raid(&res, false).expect("human format");
        assert_eq!(text, "Failed\n  rolled back (2 warnings)\n");
    }

    #[test]
    fn format_human_failed_without_rollback_stays_short() {
        let res = result(false);
        assert_eq!(
            format_raid(&res, false).expect("human format"),
            "Failed\n",
            "phase failures (nothing reached the den) stay a one-liner"
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
