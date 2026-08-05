//! Directory-skip policy for the walker.
//!
//! [`SkipPolicy`] decides which *directories* a walk never descends into.
//! Skipping files (e.g. `.DS_Store`) is deliberately out of scope on this
//! stage and is a follow-up; the policy matches directory names only.

use std::path::Path;

/// Reason a directory was skipped by a [`SkipPolicy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Matched a built-in default directory name.
    DefaultDirName,
    /// Matched a caller-supplied custom name.
    CustomPattern,
    /// `skip_hidden_dirs` was enabled and the name starts with `.`.
    Hidden,
}

/// Built-in default directory names, in deterministic order.
const DEFAULT_DIR_NAMES: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".cache",
    "dist",
    "build",
    ".idea",
    ".vscode",
    ".raccpack",
    "*.egg-info",
];

/// Policy for which directories a walk never descends into.
///
/// Names are matched against `file_name()` via its lossy string form. A
/// pattern starting with `*` is a suffix match (`*.egg-info` matches any name
/// ending in `.egg-info`); anything else is an exact match.
///
/// No whitelist is applied on this stage (M1.4); hidden-directory skipping is
/// opt-in via [`SkipPolicy::with_skip_hidden_dirs`].
#[derive(Debug, Clone)]
pub struct SkipPolicy {
    dir_names: Vec<String>,
    custom_dir_names: Vec<String>,
    skip_hidden_dirs: bool,
}

impl SkipPolicy {
    /// Built-in defaults suitable for source-tree scanning.
    ///
    /// Includes `node_modules`, `target`, common VCS dirs (`.git`, `.svn`,
    /// `.hg`), virtualenvs and cache dirs (`.venv`, `venv`, `__pycache__`,
    /// `.tox`, `.mypy_cache`, `.pytest_cache`, `.cache`), build outputs
    /// (`dist`, `build`), IDE dirs (`.idea`, `.vscode`), the den (`.raccpack`),
    /// and Python egg-info dirs (`*.egg-info`).
    pub fn default_scan() -> Self {
        Self {
            dir_names: DEFAULT_DIR_NAMES.iter().map(|s| (*s).to_string()).collect(),
            custom_dir_names: Vec::new(),
            skip_hidden_dirs: false,
        }
    }

    /// Empty policy: nothing is skipped by name; only `max_depth` and explicit
    /// filters apply.
    pub fn empty() -> Self {
        Self {
            dir_names: Vec::new(),
            custom_dir_names: Vec::new(),
            skip_hidden_dirs: false,
        }
    }

    /// Replace the custom names/patterns with the given iterator of values.
    pub fn with_custom_dir_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.custom_dir_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// Toggle skipping of hidden directories (names starting with `.`).
    pub fn with_skip_hidden_dirs(mut self, enabled: bool) -> Self {
        self.skip_hidden_dirs = enabled;
        self
    }

    /// Whether `path`'s directory name is skipped by this policy.
    ///
    /// Equivalent to `skip_reason_dir(path).is_some()`.
    pub fn should_skip_dir(&self, path: &Path) -> bool {
        self.skip_reason_dir(path).is_some()
    }

    /// Reason `path`'s directory name is skipped, if any.
    ///
    /// Deterministic ordering: built-in default names first, then custom
    /// names, then the hidden-directory rule.
    pub fn skip_reason_dir(&self, path: &Path) -> Option<SkipReason> {
        let name = path.file_name()?.to_string_lossy();
        if self.matches_any(&name, &self.dir_names) {
            return Some(SkipReason::DefaultDirName);
        }
        if self.matches_any(&name, &self.custom_dir_names) {
            return Some(SkipReason::CustomPattern);
        }
        if self.skip_hidden_dirs && name.starts_with('.') {
            return Some(SkipReason::Hidden);
        }
        None
    }

    fn matches_any(&self, name: &str, patterns: &[String]) -> bool {
        patterns
            .iter()
            .any(|pattern| match pattern.strip_prefix('*') {
                Some(suffix) => name.ends_with(suffix),
                None => name == pattern,
            })
    }
}
