//! Renders a `stash` use-case result as JSON or human-readable text.

use raccpack_core::StashResult;

use crate::error::CliError;
use crate::output::human_size;

/// Print a stash result to stdout as JSON or as a plain-text block.
pub fn print_stash(result: &StashResult, remove_sources: bool, json: bool) -> Result<(), CliError> {
    let text = format_stash(result, remove_sources, json)?;
    print!("{text}");
    Ok(())
}

/// Render a stash result as a JSON document or a human-readable block.
fn format_stash(
    result: &StashResult,
    remove_sources: bool,
    json: bool,
) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_stash(result, remove_sources))
    }
}

/// Build the human-readable stash block, dry-run or commit.
fn format_human_stash(result: &StashResult, remove_sources: bool) -> String {
    let mut out = String::new();
    if result.dry_run {
        out.push_str("Stash (dry-run)\n");
        out.push_str(&format!(
            "  Would archive: {} files → {}\n",
            result.files_archived,
            result.archive_path.display()
        ));
        out.push_str(&format!(
            "  Would remove sources: {}\n",
            if remove_sources {
                "yes (requires --yes)"
            } else {
                "no (--remove-sources not set)"
            }
        ));
        out.push_str("  (nothing written or deleted)\n");
    } else {
        out.push_str("Stash complete\n");
        out.push_str(&format!("  Archive: {}\n", result.archive_path.display()));
        out.push_str(&format!(
            "  Files: {}  ({} plaintext)\n",
            result.files_archived,
            human_size(result.bytes_archived)
        ));
        out.push_str(&format!("  Removed sources: {}\n", result.removed_sources));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use raccpack_core::secrets::stash_batch::StashManifestEntry;
    use raccpack_core::SensitiveRisk;

    fn commit_result() -> StashResult {
        StashResult {
            archive_path: PathBuf::from(
                "/home/user/.raccpack/den/secrets/2026/08/my-api__20260804T181500Z__secrets.age",
            ),
            files_archived: 3,
            bytes_archived: 1_228,
            removed_sources: 3,
            dry_run: false,
            manifest: vec![StashManifestEntry {
                original_path: PathBuf::from("/home/user/DEV/PROJS/my-api/.env"),
                risk: SensitiveRisk::High,
                size_bytes: 409,
            }],
        }
    }

    fn dry_run_result() -> StashResult {
        let mut result = commit_result();
        result.dry_run = true;
        result.removed_sources = 0;
        result
    }

    #[test]
    fn format_stash_dry_run_human_block() {
        let text = format_stash(&dry_run_result(), false, false).expect("human format");
        assert!(text.starts_with("Stash (dry-run)\n"));
        assert!(text.contains(
            "  Would archive: 3 files → /home/user/.raccpack/den/secrets/2026/08/my-api__20260804T181500Z__secrets.age\n"
        ));
        assert!(text.contains("  Would remove sources: no (--remove-sources not set)\n"));
        assert!(text.contains("  (nothing written or deleted)\n"));
        assert!(!text.contains("Archive:"), "dry run has no archive yet");
    }

    #[test]
    fn format_stash_dry_run_remove_sources_visible() {
        let text = format_stash(&dry_run_result(), true, false).expect("human format");
        assert!(text.contains("  Would remove sources: yes (requires --yes)\n"));
    }

    #[test]
    fn format_stash_commit_human_block() {
        let text = format_stash(&commit_result(), true, false).expect("human format");
        assert!(text.starts_with("Stash complete\n"));
        assert!(text.contains(
            "  Archive: /home/user/.raccpack/den/secrets/2026/08/my-api__20260804T181500Z__secrets.age\n"
        ));
        assert!(text.contains("  Files: 3  (1.2 KiB plaintext)\n"));
        assert!(text.contains("  Removed sources: 3\n"));
        assert!(!text.contains("dry-run"), "commit report is not a dry run");
    }

    #[test]
    fn format_stash_human_never_has_raw_values() {
        for result in [commit_result(), dry_run_result()] {
            let text = format_stash(&result, true, false).expect("human format");
            assert!(!text.contains("AKIA"), "no token-like raw value");
            assert!(!text.contains("passphrase"), "no passphrase leaked");
        }
    }

    #[test]
    fn format_stash_json_serializes_full_result() {
        let json = format_stash(&commit_result(), true, true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(
            value["archive_path"],
            "/home/user/.raccpack/den/secrets/2026/08/my-api__20260804T181500Z__secrets.age"
        );
        assert_eq!(value["files_archived"], 3);
        assert_eq!(value["bytes_archived"], 1_228);
        assert_eq!(value["removed_sources"], 3);
        assert_eq!(value["dry_run"], false);
        assert_eq!(value["manifest"][0]["risk"], "High");
    }
}
