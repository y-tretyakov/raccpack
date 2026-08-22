//! Config loading, validation, and path resolution for raccpack-core.
//!
//! Config is read from a TOML file (sections style: `[paths]`, `[scanner]`,
//! `[detect]`)
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

use crate::detect::DetectMode;

mod error;
mod init;
mod migrate;
mod paths;
mod validate;

pub use error::ConfigError;
pub use init::{default_toml, init_config, InitOptions, InitResult};
pub use migrate::{default_config_version, migrate_to_current, CURRENT_CONFIG_VERSION};
pub use paths::{default_config_path, DEFAULT_DEN_DIR};

/// Top-level raccpack configuration.
///
/// Unknown TOML keys are ignored (no `deny_unknown_fields`) so future sections
/// such as `[sensitive]` or `[advanced]` do not break parsing.
#[derive(Debug, Clone, Deserialize)]
pub struct RaccConfig {
    /// Configuration schema version.
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    /// Paths for the scan input and den output.
    #[serde(default)]
    pub paths: PathsConfig,
    /// Scanner behavior settings.
    #[serde(default)]
    pub scanner: ScannerConfig,
    /// Cleanup (rinse) strategy toggles.
    #[serde(default)]
    pub cleanup: CleanupConfig,
    /// Detection pipeline settings (sniff).
    #[serde(default)]
    pub detect: DetectConfig,
}

impl Default for RaccConfig {
    fn default() -> Self {
        Self {
            config_version: default_config_version(),
            paths: PathsConfig::default(),
            scanner: ScannerConfig::default(),
            cleanup: CleanupConfig::default(),
            detect: DetectConfig::default(),
        }
    }
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

/// Cleanup (rinse) behavior settings.
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupConfig {
    /// Strategy ids enabled when `RinseOptions.strategies` is `None`.
    #[serde(default = "default_enabled_strategies")]
    pub enabled_strategies: Vec<String>,
}

/// Default enabled cleanup strategy ids: only `rust`, `node`, `python`.
///
/// `jvm`, `go`, and `generic` are opt-in: `dist`/`build`/`vendor`/`tmp` are
/// *careful* names that may be genuine source or user data (see
/// `clean::strategy::DEFAULT_STRATEGIES`).
pub fn default_enabled_strategies() -> Vec<String> {
    vec!["rust".into(), "node".into(), "python".into()]
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            enabled_strategies: default_enabled_strategies(),
        }
    }
}

/// Detection pipeline settings (`[detect]`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DetectConfig {
    /// Pipeline used by `racc sniff`; defaults to [`DetectMode::PriorityTable`].
    #[serde(default)]
    pub mode: DetectMode,
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

    /// Load, migrate, and validate configuration from an explicit path.
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
        let raw: toml::Value = toml::from_str(&content).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        let migrated = migrate::migrate_to_current(raw)?;
        validate::validate_detect_mode_raw(&migrated)?;
        let config: Self = migrated.try_into().map_err(|source| ConfigError::Parse {
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
    /// Checks that `scanner.max_depth` is at least 1 and that every
    /// `cleanup.enabled_strategies` entry is a known strategy id. Empty
    /// `scan_root` / `den_dir` strings are handled at resolve time and do not
    /// need mutation here. Called automatically by [`RaccConfig::load`] and
    /// [`RaccConfig::load_from_path`].
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate::validate_max_depth(self.scanner.max_depth)?;
        validate::validate_enabled_strategies(&self.cleanup.enabled_strategies)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn write_config(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("config.toml");
        std::fs::write(&path, body).expect("write config fixture");
        path
    }

    #[test]
    fn config_without_detect_section_defaults_to_priority_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_config(
            dir.path(),
            "[paths]\nscan_root = '/tmp'\n[scanner]\nmax_depth = 3\n",
        );
        let config = RaccConfig::load_from_path(&path).unwrap();
        assert_eq!(config.detect.mode, DetectMode::PriorityTable);
    }

    #[test]
    fn detect_section_parses_both_canonical_modes() {
        for (text, expected) in [
            ("priority_table", DetectMode::PriorityTable),
            ("composite_dag", DetectMode::CompositeDag),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = write_config(dir.path(), &format!("[detect]\nmode = \"{text}\"\n"));
            let config = RaccConfig::load_from_path(&path).unwrap();
            assert_eq!(config.detect.mode, expected);
        }
    }

    #[test]
    fn unknown_detect_mode_is_a_typed_config_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_config(dir.path(), "[detect]\nmode = \"bogus_pipeline\"\n");
        let err = RaccConfig::load_from_path(&path).unwrap_err();
        match err {
            ConfigError::UnknownDetectMode { ref value } => {
                assert_eq!(value, "bogus_pipeline")
            }
            other => panic!("expected UnknownDetectMode, got {other:?}"),
        }
        assert!(err.to_string().contains("priority_table"));
        assert!(err.suggestion().is_some());
    }
}
