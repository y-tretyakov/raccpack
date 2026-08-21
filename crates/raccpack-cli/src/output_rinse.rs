//! Renders a `rinse` use-case result as JSON or human-readable text.

use std::path::Path;

use raccpack_core::{RinseResult, TrashDir};

use crate::error::CliError;
use crate::output::human_size;

/// Print a rinse result to stdout as JSON or as a plain-text block.
pub fn print_rinse(result: &RinseResult, target: &Path, json: bool) -> Result<(), CliError> {
    let text = format_rinse(result, target, json)?;
    print!("{text}");
    Ok(())
}

/// Render a rinse result as a JSON document or a human-readable block.
fn format_rinse(result: &RinseResult, target: &Path, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_rinse(result, target))
    }
}

/// Build the human-readable rinse block, dry-run or commit.
fn format_human_rinse(result: &RinseResult, target: &Path) -> String {
    let mut out = String::new();
    if result.dry_run {
        out.push_str("Rinse (dry-run)\n");
        out.push_str(&format!("  Project: {}\n", target.display()));
        out.push_str(&format!(
            "  Would remove {} directories ({})\n",
            result.removed.len(),
            human_size(result.bytes_freed)
        ));
        for dir in &result.removed {
            out.push_str(&format!(
                "    {}  [{}]  {}\n",
                dir_name(dir),
                dir.strategy,
                human_size(dir.size_bytes)
            ));
        }
        out.push_str("  (nothing deleted)\n");
    } else {
        out.push_str("Rinse complete\n");
        out.push_str(&format!(
            "  Removed {} directories, freed {}\n",
            result.removed.len(),
            human_size(result.bytes_freed)
        ));
    }
    out
}

/// The last path component of a removed trash dir, falling back to the full
/// path when the name cannot be extracted (should not happen for matched dirs).
fn dir_name(dir: &TrashDir) -> String {
    dir.path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn trash_dir(path: &str, strategy: &str, size: u64) -> TrashDir {
        TrashDir {
            path: PathBuf::from(path),
            strategy: strategy.to_string(),
            pattern_name: "pattern".to_string(),
            size_bytes: size,
        }
    }

    fn dry_run_result() -> RinseResult {
        RinseResult {
            removed: vec![
                trash_dir(
                    "/home/user/DEV/PROJS/my-api/node_modules",
                    "node",
                    125_829_120,
                ),
                trash_dir("/home/user/DEV/PROJS/my-api/target", "rust", 21_181_235),
            ],
            bytes_freed: 147_010_355,
            dry_run: true,
        }
    }

    #[test]
    fn format_rinse_dry_run_human_block() {
        let target = Path::new("/home/user/DEV/PROJS/my-api");
        let text = format_rinse(&dry_run_result(), target, false).expect("human format");
        assert!(text.starts_with("Rinse (dry-run)\n"));
        assert!(text.contains("  Project: /home/user/DEV/PROJS/my-api\n"));
        assert!(text.contains("  Would remove 2 directories (140.2 MiB)\n"));
        assert!(text.contains("    node_modules  [node]  120.0 MiB\n"));
        assert!(text.contains("    target  [rust]  20.2 MiB\n"));
        assert!(text.contains("  (nothing deleted)\n"));
        assert!(!text.contains("Removed"), "dry run deletes nothing");
    }

    #[test]
    fn format_rinse_commit_human_block() {
        let result = RinseResult {
            removed: vec![
                trash_dir(
                    "/home/user/DEV/PROJS/my-api/node_modules",
                    "node",
                    125_829_120,
                ),
                trash_dir("/home/user/DEV/PROJS/my-api/target", "rust", 21_181_235),
            ],
            bytes_freed: 147_010_355,
            dry_run: false,
        };
        let text = format_rinse(&result, Path::new("/home/user/DEV/PROJS/my-api"), false)
            .expect("human format");
        assert!(text.starts_with("Rinse complete\n"));
        assert!(text.contains("  Removed 2 directories, freed 140.2 MiB\n"));
        assert!(!text.contains("dry-run"), "commit report is not a dry run");
        assert!(!text.contains("(nothing deleted)"), "commit deletes");
    }

    #[test]
    fn format_rinse_json_serializes_full_result() {
        let target = Path::new("/home/user/DEV/PROJS/my-api");
        let json = format_rinse(&dry_run_result(), target, true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["dry_run"], true);
        assert_eq!(value["bytes_freed"], 147_010_355);
        assert_eq!(value["removed"][0]["strategy"], "node");
        assert_eq!(
            value["removed"][0]["path"],
            "/home/user/DEV/PROJS/my-api/node_modules"
        );
        assert_eq!(value["removed"][1]["size_bytes"], 21_181_235);
    }
}
