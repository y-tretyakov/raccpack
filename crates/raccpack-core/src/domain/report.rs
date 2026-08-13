/// Result of a workspace scan.
///
/// `total_size_bytes` is the sum of `project.size_bytes` over all projects.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScanReport {
    /// Root that was scanned.
    pub root: std::path::PathBuf,
    /// Projects discovered under the root.
    pub projects: Vec<crate::domain::project::Project>,
    /// Sum of `project.size_bytes` across all projects.
    pub total_size_bytes: u64,
    /// Schema version for JSON consumers (CI). Starts at 1.
    pub schema_version: u32,
}

impl Default for ScanReport {
    fn default() -> Self {
        Self {
            root: std::path::PathBuf::new(),
            projects: Vec::new(),
            total_size_bytes: 0,
            schema_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::{Project, Stack};

    fn sample_report() -> ScanReport {
        let project = Project {
            path: std::path::PathBuf::from("/tmp/demo"),
            name: "demo".to_string(),
            stack: Stack::default(),
            size_bytes: 100,
            is_git_repo: false,
        };
        ScanReport {
            root: std::path::PathBuf::from("/tmp"),
            projects: vec![project],
            total_size_bytes: 100,
            schema_version: 1,
        }
    }

    #[test]
    fn scan_report_json_roundtrip() {
        let report = sample_report();
        let json = serde_json::to_string(&report).unwrap();
        let back: ScanReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }

    #[test]
    fn default_schema_version_is_one() {
        assert_eq!(ScanReport::default().schema_version, 1);
    }
}
