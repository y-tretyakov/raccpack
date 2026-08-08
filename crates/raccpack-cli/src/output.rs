//! Renders use-case results as JSON or human-readable text.

use raccpack_core::{Project, SniffResult, Stack};

use crate::error::CliError;

const NAME_HDR: &str = "NAME";
const STACK_HDR: &str = "STACK";
const SIZE_HDR: &str = "SIZE";
const GIT_HDR: &str = "GIT";
const PATH_HDR: &str = "PATH";

/// Print a sniff result to stdout as JSON or as a plain-text table.
pub fn print_sniff(result: &SniffResult, json: bool) -> Result<(), CliError> {
    let text = format_sniff(result, json)?;
    print!("{text}");
    Ok(())
}

/// Render a sniff result as a JSON document or a plain-text table.
fn format_sniff(result: &SniffResult, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human(result))
    }
}

/// Build the human-readable sniff summary and project table.
fn format_human(result: &SniffResult) -> String {
    let mut out = String::new();
    let root = result.report.root.display();
    out.push_str(&format!("Scan root: {root}\n"));
    let count = result.report.projects.len();
    let size = human_size(result.report.total_size_bytes);
    let ms = result.duration_ms;
    let cache = if result.from_cache { "hit" } else { "miss" };
    out.push_str(&format!(
        "Projects: {count}  |  Total size: {size}  |  {ms} ms  |  cache: {cache}\n"
    ));
    out.push('\n');
    format_table(&result.report.projects, &mut out);
    out
}

/// Append the project table with stable, left-aligned columns.
fn format_table(projects: &[Project], out: &mut String) {
    let name_w = projects
        .iter()
        .map(|p| p.name.len())
        .fold(NAME_HDR.len(), usize::max);
    let stack_w = projects
        .iter()
        .map(|p| stack_str(&p.stack).len())
        .fold(STACK_HDR.len(), usize::max);
    let size_w = projects
        .iter()
        .map(|p| human_size(p.size_bytes).len())
        .fold(SIZE_HDR.len(), usize::max);

    out.push_str(&format!(
        "{NAME_HDR:<name_w$}  {STACK_HDR:<stack_w$}  {SIZE_HDR:<size_w$}  {GIT_HDR:<3}  {PATH_HDR}\n"
    ));
    for project in projects {
        let name = &project.name;
        let stack = stack_str(&project.stack);
        let size = human_size(project.size_bytes);
        let git = if project.is_git_repo { "yes" } else { "no" };
        let path = project.path.display();
        out.push_str(&format!(
            "{name:<name_w$}  {stack:<stack_w$}  {size:<size_w$}  {git:<3}  {path}\n"
        ));
    }
}

/// Compact human-readable stack: `language + framework1 + framework2` or `-`.
fn stack_str(stack: &Stack) -> String {
    let Some(language) = &stack.language else {
        return "-".to_string();
    };
    if stack.frameworks.is_empty() {
        language.clone()
    } else {
        let frameworks = stack.frameworks.join(" + ");
        format!("{language} + {frameworks}")
    }
}

/// Format a byte count with binary units (KiB/MiB/GiB/TiB), one decimal place.
fn human_size(bytes: u64) -> String {
    const KIB: u64 = 1 << 10;
    const MIB: u64 = 1 << 20;
    const GIB: u64 = 1 << 30;
    const TIB: u64 = 1 << 40;

    if bytes >= TIB {
        let value = bytes as f64 / TIB as f64;
        format!("{value:.1} TiB")
    } else if bytes >= GIB {
        let value = bytes as f64 / GIB as f64;
        format!("{value:.1} GiB")
    } else if bytes >= MIB {
        let value = bytes as f64 / MIB as f64;
        format!("{value:.1} MiB")
    } else if bytes >= KIB {
        let value = bytes as f64 / KIB as f64;
        format!("{value:.1} KiB")
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raccpack_core::ScanReport;

    fn sample_result() -> SniffResult {
        SniffResult {
            report: ScanReport {
                root: std::path::PathBuf::from("/tmp"),
                projects: vec![
                    Project {
                        path: std::path::PathBuf::from("/tmp/app-api"),
                        name: "app-api".to_string(),
                        stack: Stack {
                            language: Some("Rust".to_string()),
                            frameworks: Vec::new(),
                            markers: Vec::new(),
                        },
                        size_bytes: 12_689_920,
                        is_git_repo: true,
                    },
                    Project {
                        path: std::path::PathBuf::from("/tmp/scripts"),
                        name: "scripts".to_string(),
                        stack: Stack::default(),
                        size_bytes: 2048,
                        is_git_repo: false,
                    },
                ],
                total_size_bytes: 12_691_968,
                schema_version: 1,
            },
            from_cache: false,
            duration_ms: 42,
        }
    }

    #[test]
    fn human_size_uses_binary_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KiB");
        assert_eq!(human_size(1024 * 1024), "1.0 MiB");
        assert_eq!(human_size(128 * 1024 * 1024 + 419430), "128.4 MiB");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn stack_str_formats_language_and_frameworks() {
        let bare = Stack {
            language: Some("Rust".to_string()),
            ..Stack::default()
        };
        assert_eq!(stack_str(&bare), "Rust");

        let web = Stack {
            language: Some("TypeScript".to_string()),
            frameworks: vec!["Next.js".to_string(), "Tailwind".to_string()],
            ..Stack::default()
        };
        assert_eq!(stack_str(&web), "TypeScript + Next.js + Tailwind");

        assert_eq!(stack_str(&Stack::default()), "-");
    }

    #[test]
    fn format_human_includes_summary_and_columns() {
        let text = format_sniff(&sample_result(), false).expect("human format");
        assert!(text.starts_with("Scan root: /tmp\n"));
        assert!(text.contains("Projects: 2  |  Total size: 12.1 MiB  |  42 ms  |  cache: miss"));
        assert!(text.contains("NAME"));
        assert!(text.contains("STACK"));
        assert!(text.contains("SIZE"));
        assert!(text.contains("GIT"));
        assert!(text.contains("PATH"));
        assert!(text.contains("app-api"));
        assert!(text.contains("scripts"));
        assert!(text.contains("yes"));
        assert!(text.contains("no"));
    }

    #[test]
    fn format_human_empty_projects_still_prints_header() {
        let mut result = sample_result();
        result.report.projects.clear();
        result.report.total_size_bytes = 0;
        let text = format_sniff(&result, false).expect("human format");
        assert!(text.starts_with("Scan root: /tmp\n"));
        assert!(text.contains("Projects: 0  |  Total size: 0 B"));
        assert!(text.contains("NAME"));
    }
}
