/// Description of a project's technology stack (result of detection).
///
/// Default: no language detected, empty frameworks and markers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct Stack {
    /// Primary language, if detected (e.g. "Rust", "TypeScript").
    pub language: Option<String>,
    /// Framework / runtime hints (e.g. "Axum", "Next.js").
    pub frameworks: Vec<String>,
    /// Raw marker hits that contributed (optional, for debug).
    pub markers: Vec<String>,
}

/// A single discovered project.
///
/// `name` usually equals `path.file_name()`, but callers may override it.
/// `path` is not required to be canonicalized at the DTO stage; normalization
/// is the caller's (facade) responsibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Project {
    /// Absolute or normalized path to project root.
    pub path: std::path::PathBuf,
    /// Directory name (or derived display name).
    pub name: String,
    /// Detected technology stack.
    pub stack: Stack,
    /// Total size in bytes (files under project, after skip policy — later).
    pub size_bytes: u64,
    /// Whether the project root is inside a git repository.
    pub is_git_repo: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_default_is_empty() {
        let stack = Stack::default();
        assert_eq!(stack.language, None);
        assert!(stack.frameworks.is_empty());
        assert!(stack.markers.is_empty());
    }

    #[test]
    fn project_serde_roundtrip_with_pathbuf() {
        let project = Project {
            path: std::path::PathBuf::from("/tmp/demo"),
            name: "demo".to_string(),
            stack: Stack {
                language: Some("Rust".to_string()),
                frameworks: vec!["Axum".to_string()],
                markers: vec!["Cargo.toml".to_string()],
            },
            size_bytes: 4096,
            is_git_repo: true,
        };
        let json = serde_json::to_string(&project).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, back);
    }
}
