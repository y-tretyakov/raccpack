use std::path::PathBuf;

/// Unified library error type for raccpack-core.
///
/// No `anyhow` or `Box<dyn Error>` in the public API; variants are added as
/// later phases need them.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The referenced path does not exist.
    #[error("path not found: {path}")]
    PathNotFound { path: PathBuf },
    /// The referenced path exists but is not a directory.
    #[error("not a directory: {path}")]
    NotADirectory { path: PathBuf },
    /// Underlying I/O failure at the given path.
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Invalid configuration input.
    #[error("invalid configuration: {message}")]
    Config { message: String },
    /// Incompatible major version in an existing den's `.den-version`.
    #[error("incompatible den version: found {found}, expected {expected}")]
    DenVersion {
        found: String,
        expected: &'static str,
    },
    /// Catch-all for errors without a dedicated variant.
    #[error("{message}")]
    Other { message: String },
}

impl Error {
    /// Optional UX hint for CLI / TUI / Desktop surfaces.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Error::PathNotFound { .. } => Some("Check that scan_root exists and is accessible."),
            Error::NotADirectory { .. } => Some("Provide a directory path, not a file."),
            Error::DenVersion { .. } => {
                Some("Point den_dir at a compatible den, or migrate it with a future `racc den migrate` command.")
            }
            _ => None,
        }
    }
}

/// Convenience result alias for library operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_implements_std_error_and_display() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<Error>();

        let err = Error::PathNotFound {
            path: PathBuf::from("/nope"),
        };
        assert_eq!(err.to_string(), "path not found: /nope");

        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "boom");
        let err = Error::Io {
            path: PathBuf::from("/tmp"),
            source: io,
        };
        assert!(err.to_string().contains("io error at /tmp"));
        let dyn_err: &dyn std::error::Error = &err;
        assert!(dyn_err.source().is_some());
    }

    #[test]
    fn suggestion_covers_path_errors() {
        assert_eq!(
            Error::PathNotFound {
                path: PathBuf::from("/nope")
            }
            .suggestion(),
            Some("Check that scan_root exists and is accessible.")
        );
        assert_eq!(
            Error::NotADirectory {
                path: PathBuf::from("/etc/hosts")
            }
            .suggestion(),
            Some("Provide a directory path, not a file.")
        );
        assert_eq!(
            Error::Config {
                message: "bad".into()
            }
            .suggestion(),
            None
        );
        assert_eq!(
            Error::Other {
                message: "bad".into()
            }
            .suggestion(),
            None
        );
    }

    #[test]
    fn result_alias_resolves_to_error() {
        let r: Result<()> = Err(Error::Other {
            message: "x".into(),
        });
        assert!(r.is_err());
    }
}
