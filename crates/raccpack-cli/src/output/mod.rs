//! Renders use-case results as JSON or human-readable text.

mod tree_render;

use raccpack_core::{DigResult, Project, RepeatedSecret, SensitiveFile, SniffResult, Stack};

use crate::error::CliError;

const NAME_HDR: &str = "NAME";
const STACK_HDR: &str = "STACK";
const SIZE_HDR: &str = "SIZE";
const GIT_HDR: &str = "GIT";
const PATH_HDR: &str = "PATH";
const RISK_HDR: &str = "RISK";
const LABEL_HDR: &str = "LABEL";

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

    for project in &result.report.projects {
        if let Some(tree) = &project.stack_tree {
            out.push('\n');
            let size = human_size(project.size_bytes);
            out.push_str(&format!("  {} ({size})\n", project.name));
            out.push_str(&tree_render::render_tree(tree, &project.path));
        }
    }

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
pub(crate) fn human_size(bytes: u64) -> String {
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

/// Print a dig result to stdout as JSON or as a plain-text table.
pub fn print_dig(result: &DigResult, json: bool) -> Result<(), CliError> {
    let text = format_dig(result, json)?;
    print!("{text}");
    Ok(())
}

/// Render a dig result as a JSON document or a human-readable report.
fn format_dig(result: &DigResult, json: bool) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(result)?)
    } else {
        Ok(format_human_dig(result))
    }
}

/// Build the human-readable dig summary, findings table and repeated block.
fn format_human_dig(result: &DigResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Dig root: {}\n", result.root.display()));
    out.push_str(&format!(
        "Files scanned: {}  |  Findings: {}  |  Repeated: {}  |  {} ms\n",
        result.files_scanned,
        result.files.len(),
        result.repeated.len(),
        result.duration_ms,
    ));
    out.push('\n');

    let mut files = result.files.clone();
    files.sort_by(|a, b| b.risk.cmp(&a.risk).then(a.path.cmp(&b.path)));
    if !files.is_empty() {
        format_findings_table(&files, &mut out);
    }

    if !result.repeated.is_empty() {
        out.push('\n');
        format_repeated(&result.repeated, &mut out);
    }
    out
}

/// Append the findings table sorted by risk desc, then path asc.
fn format_findings_table(files: &[SensitiveFile], out: &mut String) {
    let risk_w = files
        .iter()
        .map(|f| f.risk.as_str().len())
        .fold(RISK_HDR.len(), usize::max);
    let label_w = files
        .iter()
        .map(|f| primary_label(f).len())
        .fold(LABEL_HDR.len(), usize::max);

    out.push_str(&format!(
        "{RISK_HDR:<risk_w$}  {LABEL_HDR:<label_w$}  {PATH_HDR}\n"
    ));
    for file in files {
        out.push_str(&format!(
            "{:<risk_w$}  {:<label_w$}  {}\n",
            file.risk.as_str(),
            primary_label(file),
            file.path.display(),
        ));
    }
}

/// The primary label of a sensitive file (`labels[0]`), or `-` when empty.
fn primary_label(file: &SensitiveFile) -> &str {
    file.labels.first().map(String::as_str).unwrap_or("-")
}

/// Append the repeated-secrets block with a short hash preview per group.
fn format_repeated(repeated: &[RepeatedSecret], out: &mut String) {
    out.push_str("Repeated secrets:\n");
    for item in repeated {
        out.push_str(&format!(
            "  hash={}  risk={}  count={}\n",
            short_hash(&item.value_hash),
            item.risk.as_str(),
            item.count,
        ));
        for path in &item.paths {
            out.push_str(&format!("    {}\n", path.display()));
        }
    }
}

/// Short stable preview of a value hash (never the raw value).
fn short_hash(hash: &str) -> String {
    if hash.len() <= 5 {
        hash.to_string()
    } else {
        format!("{}…", &hash[..4])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raccpack_core::{ScanReport, SensitiveRisk};

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
                        stack_tree: None,
                        size_bytes: 12_689_920,
                        is_git_repo: true,
                    },
                    Project {
                        path: std::path::PathBuf::from("/tmp/scripts"),
                        name: "scripts".to_string(),
                        stack: Stack::default(),
                        stack_tree: None,
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

    fn masked(value: &str) -> raccpack_core::MaskedValue {
        raccpack_core::MaskedValue {
            masked: value.to_string(),
            value_hash: "abcd1234".to_string(),
            original_len: value.len(),
        }
    }

    fn sample_dig_result() -> DigResult {
        DigResult {
            root: std::path::PathBuf::from("/tmp/proj"),
            files: vec![
                SensitiveFile {
                    path: std::path::PathBuf::from("/tmp/proj/scripts/token.txt"),
                    risk: SensitiveRisk::Medium,
                    labels: vec!["JWT-like".to_string()],
                    content_match: Some(masked("eyJ0…abc")),
                    git_status: None,
                },
                SensitiveFile {
                    path: std::path::PathBuf::from("/tmp/proj/app/.env"),
                    risk: SensitiveRisk::High,
                    labels: vec!["Env file".to_string()],
                    content_match: Some(masked("DB=****")),
                    git_status: None,
                },
                SensitiveFile {
                    path: std::path::PathBuf::from("/tmp/proj/certs/server.key"),
                    risk: SensitiveRisk::Critical,
                    labels: vec!["Private key PEM".to_string()],
                    content_match: None,
                    git_status: None,
                },
                SensitiveFile {
                    path: std::path::PathBuf::from("/tmp/proj/app/config/aws.env"),
                    risk: SensitiveRisk::Critical,
                    labels: vec!["AWS Access Key".to_string()],
                    content_match: Some(masked("AKIA…23")),
                    git_status: None,
                },
            ],
            repeated: vec![RepeatedSecret {
                value_hash: "abcd1234".to_string(),
                masked: "DB=****".to_string(),
                risk: SensitiveRisk::High,
                paths: vec![
                    std::path::PathBuf::from("/tmp/proj/app/.env"),
                    std::path::PathBuf::from("/tmp/proj/app/.env.backup"),
                ],
                count: 2,
            }],
            duration_ms: 180,
            files_scanned: 1204,
        }
    }

    #[test]
    fn format_dig_human_sorts_risk_desc_then_path_asc() {
        let text = format_dig(&sample_dig_result(), false).expect("human format");
        let critical_aws = text
            .find("/tmp/proj/app/config/aws.env")
            .expect("aws present");
        let critical_key = text
            .find("/tmp/proj/certs/server.key")
            .expect("key present");
        let high_env = text.find("/tmp/proj/app/.env").expect("env present");
        let medium_tok = text
            .find("/tmp/proj/scripts/token.txt")
            .expect("token present");
        assert!(
            critical_aws < critical_key && critical_key < high_env && high_env < medium_tok,
            "findings must be sorted risk desc, then path asc"
        );
    }

    #[test]
    fn format_dig_human_summary_and_headers() {
        let text = format_dig(&sample_dig_result(), false).expect("human format");
        assert!(text.starts_with("Dig root: /tmp/proj\n"));
        assert!(text.contains("Files scanned: 1204  |  Findings: 4  |  Repeated: 1  |  180 ms"));
        assert!(text.contains("RISK"));
        assert!(text.contains("LABEL"));
        assert!(text.contains("PATH"));
        assert!(text.contains("AWS Access Key"));
        assert!(text.contains("Private key PEM"));
    }

    #[test]
    fn format_dig_human_repeated_block_only_when_non_empty() {
        let mut result = sample_dig_result();
        result.repeated.clear();
        let text = format_dig(&result, false).expect("human format");
        assert!(!text.contains("Repeated secrets:"));

        let text = format_dig(&sample_dig_result(), false).expect("human format");
        assert!(text.contains("Repeated secrets:"));
        assert!(text.contains("hash=abcd…  risk=High  count=2"));
        assert!(text.contains("/tmp/proj/app/.env.backup"));
    }

    #[test]
    fn format_dig_human_never_prints_raw_values() {
        let raw = "super-secret-password-value";
        let mut result = sample_dig_result();
        result.files[0].content_match = Some(masked(raw));
        let text = format_dig(&result, false).expect("human format");
        assert!(
            !text.contains(raw),
            "raw value must never appear in human output"
        );
    }

    #[test]
    fn format_dig_json_serializes_full_result() {
        let json = format_dig(&sample_dig_result(), true).expect("json format");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["root"], "/tmp/proj");
        assert_eq!(value["files"].as_array().expect("files array").len(), 4);
        assert_eq!(value["files_scanned"], 1204);
        assert_eq!(
            value["repeated"].as_array().expect("repeated array").len(),
            1
        );
        assert!(json.contains("\"Critical\""));
    }
}
