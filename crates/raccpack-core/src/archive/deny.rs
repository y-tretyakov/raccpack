//! Name and content deny helpers for packing.
//!
//! [`should_deny_file_in_pack`] decides, by file name only, whether a file must
//! be omitted from an archive (reuses the filename secret table with a High+
//! risk threshold). [`content_deny_hit`] optionally scans file contents against
//! the content-marker table; it is off by default in M4.1.

use std::path::Path;

use crate::domain::Error;
use crate::secrets::{match_filename, scan_file_content, ContentScanLimits, SensitiveRisk};

/// Whether `path`'s file name matches a pack deny pattern.
///
/// A file is denied when [`match_filename`] reports a match at or above
/// [`SensitiveRisk::High`]. Matching is name-only (no content is read); paths
/// without a file name never match.
pub fn should_deny_file_in_pack(path: &Path) -> bool {
    match_filename(path)
        .map(|m| m.risk >= SensitiveRisk::High)
        .unwrap_or(false)
}

/// Options controlling content-based deny during packing.
#[derive(Debug, Clone)]
pub struct ContentDenyOptions {
    /// Whether to scan file contents before including them in an archive.
    pub enabled: bool,
    /// Minimum content-hit risk that causes a file to be omitted.
    pub min_risk: SensitiveRisk,
}

impl Default for ContentDenyOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            min_risk: SensitiveRisk::Critical,
        }
    }
}

/// Whether `path` should be omitted due to a content deny hit.
///
/// Returns `Ok(false)` when `opts.enabled` is false (name-only mode). Otherwise
/// the file is scanned with the default [`ContentScanLimits`] and this returns
/// `Ok(true)` when any hit meets `opts.min_risk`. Open/read failures propagate
/// as [`Error::Io`] (fail closed: a file that cannot be scanned is not packed).
pub fn content_deny_hit(path: &Path, opts: &ContentDenyOptions) -> Result<bool, Error> {
    if !opts.enabled {
        return Ok(false);
    }
    let hits = scan_file_content(path, &ContentScanLimits::default())?;
    Ok(hits.iter().any(|hit| hit.risk.at_least(opts.min_risk)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_risk_secret_names_are_denied() {
        assert!(should_deny_file_in_pack(Path::new("proj/.env")));
        assert!(should_deny_file_in_pack(Path::new("proj/src/key.pem")));
        assert!(should_deny_file_in_pack(Path::new("proj/.git-credentials")));
    }

    #[test]
    fn medium_or_lower_risk_names_are_not_denied() {
        assert!(!should_deny_file_in_pack(Path::new("proj/config.json")));
        assert!(!should_deny_file_in_pack(Path::new("proj/notes.txt")));
        assert!(!should_deny_file_in_pack(Path::new("proj/src/main.rs")));
    }

    #[test]
    fn content_deny_defaults_are_off_critical() {
        let opts = ContentDenyOptions::default();
        assert!(!opts.enabled);
        assert_eq!(opts.min_risk, SensitiveRisk::Critical);
    }

    #[test]
    fn content_deny_hit_skips_when_disabled() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("creds.txt");
        std::fs::write(&path, b"AKIAABCDEFGHIJKLMNOPQRST\n").unwrap();

        let opts = ContentDenyOptions {
            enabled: false,
            min_risk: SensitiveRisk::Critical,
        };
        assert!(!content_deny_hit(&path, &opts).unwrap());
    }

    #[test]
    fn content_deny_hit_fires_when_enabled() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("creds.txt");
        std::fs::write(&path, b"AKIAABCDEFGHIJKLMNOPQRST\n").unwrap();

        let opts = ContentDenyOptions {
            enabled: true,
            min_risk: SensitiveRisk::Critical,
        };
        assert!(content_deny_hit(&path, &opts).unwrap());
    }

    #[test]
    fn content_deny_hit_respects_min_risk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"xoxb-1234567890123456\n").unwrap();

        let critical = ContentDenyOptions {
            enabled: true,
            min_risk: SensitiveRisk::Critical,
        };
        assert!(!content_deny_hit(&path, &critical).unwrap());

        let low = ContentDenyOptions {
            enabled: true,
            min_risk: SensitiveRisk::Low,
        };
        assert!(content_deny_hit(&path, &low).unwrap());
    }

    #[test]
    fn content_deny_hit_propagates_io_error() {
        let opts = ContentDenyOptions {
            enabled: true,
            min_risk: SensitiveRisk::Critical,
        };
        let err = content_deny_hit(Path::new("/nonexistent/creds.txt"), &opts).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }
}
