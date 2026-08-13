//! Config loading, validation, and path resolution for raccpack-core.
//!
//! Config is read from a TOML file (sections style: `[paths]`, `[scanner]`)
//! with the following priority:
//!
//! 1. `RACCPACK_CONFIG` env var — explicit path, the file must exist.
//! 2. XDG default: `$XDG_CONFIG_HOME/raccpack/config.toml`, or
//!    `~/.config/raccpack/config.toml` when `XDG_CONFIG_HOME` is not set.
//! 3. If neither an env override nor an existing default file is present,
//!    [`RaccConfig::default`] is returned (paths can still be supplied later
//!    via the builder methods).
//!
//! Raw `scan_root` / `den_dir` strings may contain `~` and relative paths;
//! see the module docs in `paths.rs` for the resolution rules.

use std::env;
use std::path::{Path, PathBuf};

use serde::Deserialize;

mod error;
mod paths;
mod validate;

pub use error::ConfigError;

/// Top-level raccpack configuration.
///
/// Unknown TOML keys are ignored (no `deny_unknown_fields`) so future sections
/// such as `[sensitive]`, `[cleanup]`, or `[advanced]` do not break parsing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RaccConfig {
    /// Paths for the scan input and den output.
    #[serde(default)]
    pub paths: PathsConfig,
    /// Scanner behavior settings.
    #[serde(default)]
    pub scanner: ScannerConfig,
}

/// Raw path settings.
///
/// Values are kept as raw strings because they may contain `~` and are also
/// overridable from the CLI after load.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PathsConfig {
    /// Directory that contains the projects to scan. `None` when unset.
    pub scan_root: Option<String>,
    /// Output vault for packs and (later) secret archives. `None` when unset.
    pub den_dir: Option<String>,
}

/// Scanner behavior settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ScannerConfig {
    /// Maximum directory depth to descend while scanning.
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

/// Default walk depth when `scanner.max_depth` is not specified in TOML.
pub(crate) fn default_max_depth() -> usize {
    6
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
        }
    }
}

impl RaccConfig {
    /// Load configuration from `RACCPACK_CONFIG`, the XDG default location, or
    /// fall back to [`RaccConfig::default`].
    ///
    /// * `RACCPACK_CONFIG` is set → the file must exist (strict).
    /// * The XDG default file exists → parsed and validated.
    /// * No config file → [`RaccConfig::default`].
    pub fn load() -> Result<Self, ConfigError> {
        if let Some(path) = env::var_os("RACCPACK_CONFIG") {
            return Self::load_from_path(Path::new(&path));
        }
        let default_path = paths::default_config_path()?;
        if default_path.exists() {
            Self::load_from_path(&default_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load and validate configuration from an explicit path.
    ///
    /// A missing file yields [`ConfigError::FileNotFound`], an unreadable file
    /// [`ConfigError::Read`], and a malformed TOML document
    /// [`ConfigError::Parse`]. Validation runs after deserialization.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }
        let content = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&content).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Resolved absolute `scan_root`.
    ///
    /// Errors with [`ConfigError::MissingScanRoot`] when no `scan_root` is
    /// configured (including empty strings), and with
    /// [`ConfigError::ScanRootMissing`] when the resolved path does not exist
    /// or is not a directory. The result is not canonicalized.
    pub fn scan_root_dir(&self) -> Result<PathBuf, ConfigError> {
        let raw = paths::non_empty(&self.paths.scan_root).ok_or(ConfigError::MissingScanRoot)?;
        let resolved = paths::resolve_path(raw)?;
        paths::require_dir(&resolved)?;
        Ok(resolved)
    }

    /// Resolved absolute `den_dir`.
    ///
    /// Defaults to `~/.raccpack/den` when not configured. Existence is **not**
    /// checked (the directory is created by later phases). The result is not
    /// canonicalized.
    pub fn den_dir(&self) -> Result<PathBuf, ConfigError> {
        let raw = match paths::non_empty(&self.paths.den_dir) {
            Some(value) => value,
            None => paths::DEFAULT_DEN_DIR,
        };
        paths::resolve_path(raw)
    }

    /// Override `scan_root` after load (CLI flag `--root`).
    ///
    /// The path is stored as a raw string and resolved with the same rules as
    /// a value from TOML.
    pub fn with_scan_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.scan_root = Some(path.into().to_string_lossy().into_owned());
        self
    }

    /// Override `den_dir` after load (CLI flag `--den`).
    ///
    /// The path is stored as a raw string and resolved with the same rules as
    /// a value from TOML.
    pub fn with_den_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.den_dir = Some(path.into().to_string_lossy().into_owned());
        self
    }

    /// Validate the parsed configuration.
    ///
    /// Currently checks that `scanner.max_depth` is at least 1. Empty
    /// `scan_root` / `den_dir` strings are handled at resolve time and do not
    /// need mutation here. Called automatically by [`RaccConfig::load`] and
    /// [`RaccConfig::load_from_path`].
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate::validate_max_depth(self.scanner.max_depth)
    }
}
