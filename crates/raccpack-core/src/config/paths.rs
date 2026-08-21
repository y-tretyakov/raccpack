//! Path resolution rules for `scan_root` / `den_dir`.
//!
//! Raw path strings come from TOML (via [`super::PathsConfig`]) or from
//! CLI-override builder methods ([`super::RaccConfig::with_scan_root`],
//! [`super::RaccConfig::with_den_dir`]). They may contain `~`.
//!
//! Resolution rules:
//!
//! | Input | Behavior |
//! |---|---|
//! | `~` or `~/…` | substitute `$HOME`; `HOME` unset → [`super::ConfigError::PathResolve`] (never a silent `/`) |
//! | relative | resolved against `current_dir()` |
//! | absolute | used as-is |
//! | empty string | treated as absent — callers filter it out before calling [`resolve_path`] |
//!
//! The result is an absolute path computed lexically (`~` expansion + join
//! with the current directory). It is **not** canonicalized, so the value is
//! predictable for tests and still valid when the target does not exist yet.

use std::path::{Path, PathBuf};

use super::ConfigError;

/// Default output directory when `paths.den_dir` is not configured.
pub const DEFAULT_DEN_DIR: &str = "~/.raccpack/den";

/// Expand `~`/`~/…` to `$HOME`, join relative paths with `current_dir()`, and
/// leave absolute paths untouched.
pub fn resolve_path(raw: &str) -> Result<PathBuf, ConfigError> {
    if let Some(rest) = raw.strip_prefix('~') {
        let home = std::env::var_os("HOME").ok_or_else(|| ConfigError::PathResolve {
            raw: raw.to_string(),
            reason: "HOME is not set; cannot expand `~`".to_string(),
        })?;
        let home = PathBuf::from(home);
        if rest.is_empty() {
            return Ok(home);
        }
        return Ok(home.join(rest.trim_start_matches('/')));
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return Ok(path);
    }
    let cwd = std::env::current_dir().map_err(|err| ConfigError::PathResolve {
        raw: raw.to_string(),
        reason: format!("cannot read current directory: {err}"),
    })?;
    Ok(cwd.join(path))
}

/// Return `Some(raw)` when the value is present and non-empty, `None` for
/// absent values and empty strings (both treated the same at resolve time).
pub(super) fn non_empty(raw: &Option<String>) -> Option<&str> {
    raw.as_deref().filter(|value| !value.is_empty())
}

/// The XDG-style default config location, resolved to an absolute path.
///
/// `$XDG_CONFIG_HOME/raccpack/config.toml`, or `~/.config/raccpack/config.toml`
/// when `XDG_CONFIG_HOME` is not set.
pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("raccpack").join("config.toml"));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| ConfigError::PathResolve {
        raw: "~/.config".to_string(),
        reason: "HOME is not set; cannot locate the default config path".to_string(),
    })?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("raccpack")
        .join("config.toml"))
}

/// Check that a resolved path is an existing directory.
pub(super) fn require_dir(path: &Path) -> Result<(), ConfigError> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(ConfigError::ScanRootMissing {
            path: path.to_path_buf(),
        })
    }
}
