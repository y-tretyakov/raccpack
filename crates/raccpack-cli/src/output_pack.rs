//! Renders a `pack` use-case result as JSON or human-readable text.

use raccpack_core::PackResult;

use crate::error::CliError;
use crate::output::human_size;

/// Print a pack result to stdout as JSON or as a plain-text block.
pub fn print_pack(result: &PackResult, content_deny: bool, json: bool) -> Result<(), CliError> {
    let text = format_pack(result, content_deny, json)?;
    print!("{text}");
    Ok(())
}

/// Render a pack result as a JSON document or a human-readable block.
fn format_pack(result: &PackResult, content_deny: bool, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_pack(result, content_deny))
    }
}

/// Build the human-readable pack block, dry-run or commit (§5 M4.4).
fn format_human_pack(result: &PackResult, content_deny: bool) -> String {
    let mut out = String::new();
    if result.dry_run {
        out.push_str("Pack (dry-run)\n");
        out.push_str(&format!("  Source: {}\n", result.source.display()));
        out.push_str(&format!("  Would write: {}\n", result.output.display()));
        out.push_str(&format!(
            "  Content deny: {}\n",
            if content_deny { "on" } else { "off" }
        ));
        out.push_str("  (no files written)\n");
    } else {
        out.push_str("Pack complete\n");
        out.push_str(&format!("  Source: {}\n", result.source.display()));
        out.push_str(&format!("  Output: {}\n", result.output.display()));
        out.push_str(&format!("  Size: {}\n", human_size(result.size_bytes)));
        out.push_str(&format!("  Files: {}\n", result.file_count));
        out.push_str(&format!(
            "  Skipped secret files: {}\n",
            result.skipped_secret_files
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn commit_result() -> PackResult {
        PackResult {
            source: PathBuf::from("/home/user/DEV/PROJS/my-api"),
            output: PathBuf::from(
                "/home/user/.raccpack/den/packs/2026/08/my-api__20260804T181500Z.tar.zst",
            ),
            size_bytes: 4_404_019,
            file_count: 312,
            skipped_secret_files: 3,
            dry_run: false,
        }
    }

    fn dry_run_result() -> PackResult {
        PackResult {
            source: PathBuf::from("/home/user/DEV/PROJS/my-api"),
            output: PathBuf::from(
                "/home/user/.raccpack/den/packs/2026/08/my-api__20260804T181500Z.tar.zst",
            ),
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        }
    }

    #[test]
    fn format_pack_dry_run_human_block() {
        let text = format_pack(&dry_run_result(), true, false).expect("human format");
        assert!(text.starts_with("Pack (dry-run)\n"));
        assert!(text.contains("  Source: /home/user/DEV/PROJS/my-api\n"));
        assert!(text.contains("  Would write: /home/user/.raccpack/den/packs/2026/08/my-api__20260804T181500Z.tar.zst\n"));
        assert!(text.contains("  Content deny: on\n"));
        assert!(text.contains("  (no files written)\n"));
        assert!(!text.contains("Size:"), "dry run reports no size");
    }

    #[test]
    fn format_pack_content_deny_off_is_visible() {
        let text = format_pack(&dry_run_result(), false, false).expect("human format");
        assert!(text.contains("  Content deny: off\n"));
    }

    #[test]
    fn format_pack_commit_human_block() {
        let text = format_pack(&commit_result(), true, false).expect("human format");
        assert!(text.starts_with("Pack complete\n"));
        assert!(text.contains("  Source: /home/user/DEV/PROJS/my-api\n"));
        assert!(text.contains(
            "  Output: /home/user/.raccpack/den/packs/2026/08/my-api__20260804T181500Z.tar.zst\n"
        ));
        assert!(text.contains("  Size: 4.2 MiB\n"));
        assert!(text.contains("  Files: 312\n"));
        assert!(text.contains("  Skipped secret files: 3\n"));
        assert!(!text.contains("dry-run"), "commit report is not a dry run");
    }

    #[test]
    fn format_pack_human_never_has_raw_values() {
        for result in [commit_result(), dry_run_result()] {
            let text = format_pack(&result, true, false).expect("human format");
            assert!(!text.contains("AKIA"), "no token-like raw value");
        }
    }

    #[test]
    fn format_pack_json_serializes_full_result() {
        let json = format_pack(&commit_result(), true, true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(
            value["source"], "/home/user/DEV/PROJS/my-api",
            "source key present"
        );
        assert_eq!(
            value["output"],
            "/home/user/.raccpack/den/packs/2026/08/my-api__20260804T181500Z.tar.zst"
        );
        assert_eq!(value["size_bytes"], 4_404_019);
        assert_eq!(value["file_count"], 312);
        assert_eq!(value["skipped_secret_files"], 3);
        assert_eq!(value["dry_run"], false);
    }
}
