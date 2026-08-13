//! CLI error type and mapping to process exit codes.

use std::fmt;
use std::process::ExitCode;

use raccpack_core::{ConfigError, Error};

/// Error surfaced by the CLI layer.
///
/// Wraps typed errors from config loading and core use-cases; every variant
/// maps to exit code 1 on this stage (exit code 2 is reserved for the secret
/// policy later).
#[derive(Debug)]
pub enum CliError {
    /// Config loading or path resolution failed.
    Config(ConfigError),
    /// A core use-case failed.
    Core(Error),
    /// The output could not be serialized.
    Json(serde_json::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(err) => write!(f, "{err}"),
            Self::Core(err) => write!(f, "{err}"),
            Self::Json(err) => write!(f, "failed to serialize output: {err}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<ConfigError> for CliError {
    fn from(err: ConfigError) -> Self {
        Self::Config(err)
    }
}

impl From<Error> for CliError {
    fn from(err: Error) -> Self {
        Self::Core(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl CliError {
    /// Print the error message and its UX suggestion to stderr.
    pub fn report(&self) {
        eprintln!("error: {self}");
        if let Some(suggestion) = self.suggestion() {
            eprintln!("hint: {suggestion}");
        }
    }

    /// The exit code this error maps to (always 1 for now).
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::FAILURE
    }

    fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::Config(err) => err.suggestion(),
            Self::Core(err) => err.suggestion(),
            Self::Json(_) => None,
        }
    }
}
