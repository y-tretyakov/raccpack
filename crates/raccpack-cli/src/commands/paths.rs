//! Shared path helpers for the CLI subcommands.

use std::path::{Path, PathBuf};

/// Resolve a relative `--project` against the optional `--root` base.
///
/// Absolute projects are returned unchanged; a relative project without
/// `--root` is left as-is and resolved against the process cwd by the caller.
pub(crate) fn resolve_project_path(project: PathBuf, root: Option<&Path>) -> PathBuf {
    match root {
        Some(root) if project.is_relative() => root.join(project),
        _ => project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_joins_relative_project_with_root() {
        let resolved =
            resolve_project_path(PathBuf::from("app"), Some(Path::new("/home/user/PROJS")));
        assert_eq!(resolved, PathBuf::from("/home/user/PROJS/app"));
    }

    #[test]
    fn resolve_keeps_absolute_project_ignoring_root() {
        let resolved = resolve_project_path(
            PathBuf::from("/tmp/deep/app"),
            Some(Path::new("/home/user/PROJS")),
        );
        assert_eq!(resolved, PathBuf::from("/tmp/deep/app"));
    }

    #[test]
    fn resolve_keeps_relative_project_without_root() {
        let resolved = resolve_project_path(PathBuf::from("app"), None);
        assert_eq!(resolved, PathBuf::from("app"));
    }
}
