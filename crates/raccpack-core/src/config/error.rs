use std::path::PathBuf;

/// Strict, typed errors for config loading, parsing, and path resolution.
///
/// Kept separate from [`crate::Error`]; merging them into one library error is
/// planned as a follow-up (facade phase).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The config file does not exist at the given path.
    #[error("config file not found: {path}")]
    FileNotFound { path: PathBuf },
    /// Failed to read the config file from disk.
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The config file is not valid TOML.
    #[error("invalid TOML in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    /// No `scan_root` was provided in config, env, or CLI.
    #[error("missing scan_root: set paths.scan_root in config or pass --root")]
    MissingScanRoot,
    /// A raw path string could not be resolved (e.g. `~` without HOME).
    #[error("cannot resolve path `{raw}`: {reason}")]
    PathResolve { raw: String, reason: String },
    /// `scan_root` does not exist or is not a directory.
    #[error("scan_root does not exist: {path}")]
    ScanRootMissing { path: PathBuf },
    /// `scanner.max_depth` is not usable (must be >= 1).
    #[error("invalid max_depth: {value} (must be >= 1)")]
    InvalidMaxDepth { value: usize },
    /// `cleanup.enabled_strategies` contains an unknown strategy id.
    #[error("unknown cleanup strategy `{id}`")]
    UnknownCleanupStrategy { id: String },
}

impl ConfigError {
    /// Optional UX hint for CLI / TUI / Desktop surfaces.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            ConfigError::FileNotFound { .. } => {
                Some("Create the config file or unset RACCPACK_CONFIG to fall back to defaults.")
            }
            ConfigError::MissingScanRoot => {
                Some("Set paths.scan_root in the config file or pass --root on the command line.")
            }
            ConfigError::PathResolve { .. } => {
                Some("Use an absolute path or make sure the HOME environment variable is set.")
            }
            ConfigError::ScanRootMissing { .. } => {
                Some("Check that scan_root exists and is a directory.")
            }
            ConfigError::InvalidMaxDepth { .. } => {
                Some("Set scanner.max_depth to a value of at least 1.")
            }
            ConfigError::UnknownCleanupStrategy { .. } => {
                Some("Set cleanup.enabled_strategies to known ids: rust, node, python, jvm, go, generic.")
            }
            ConfigError::Read { .. } | ConfigError::Parse { .. } => None,
        }
    }
}
