//! Opt-in, ephemeral reveal of a single raw secret value.
//!
//! Everything else in `secrets` masks values before they cross a boundary.
//! This module is the deliberate, single exception: [`reveal_finding`]
//! re-reads a file **fresh**, re-extracts the candidate value for a specific
//! content marker + line, and returns it only when its fingerprint matches the
//! [`FindingRef`] taken from a dig run. The returned [`EphemeralSecret`] is
//! `!Serialize`, zeroized on drop, and must never be stored in long-lived
//! state — it is shown once in a UI modal and then dropped.
//!
//! Safety invariants (the ones the task pins down):
//! - the file is re-scanned from disk — never a cached raw value;
//! - the path is contained under `dir_root` (path-containment check first);
//! - the path must be a regular file (no symlink-follow beyond the scan);
//! - the value is returned only on an exact hash match; anything else is an
//!   `Error::Other` ("file changed since dig");
//! - the raw value never appears in `Debug`, logs, errors, or serde output.

use std::io::Read;
use std::path::Path;

use zeroize::Zeroizing;

use crate::domain::{Error, Result};

use super::content::{compiled_marker_by_id, extract_raw_candidates, ContentScanLimits};
use super::mask::fingerprint_secret;
use crate::scan::walk::is_path_under_root;

/// A raw secret value, shown once and zeroized on drop.
///
/// Deliberately **not** `Serialize` / `Deserialize` so it can never leak into
/// a JSON report, an IPC message, or `DigResult`/`RaidResult`. It is also not
/// `Clone` / `Copy`: there is exactly one live copy, and it is dropped when
/// the UI modal closes. `Debug` prints only `EphemeralSecret(**)`.
pub struct EphemeralSecret {
    inner: Zeroizing<String>,
}

impl EphemeralSecret {
    /// The **only** way to materialize a raw secret value.
    ///
    /// Callers at this point are the reveal path and tests; nothing else in
    /// `core` creates an `EphemeralSecret` from a value.
    pub fn new(value: String) -> Self {
        Self {
            inner: Zeroizing::new(value),
        }
    }

    /// Borrow the raw value for a transient, single display.
    pub fn expose(&self) -> &str {
        &self.inner
    }
}

impl std::fmt::Debug for EphemeralSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EphemeralSecret(**)")
    }
}

/// A safe, serializable reference to **one** secret value in **one** file,
/// sufficient to re-`reveal` it later.
///
/// It pinpoints where a value is (path + content marker + line) and fingerprints
/// the value (blake3 hex), but carries **neither** the raw value **nor** the
/// masked preview. This is the bridge that lets a dig DTO carry "where a value
/// is" without ever carrying the value itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FindingRef {
    /// The sensitive file, matching [`crate::secrets::SensitiveFinding::path`].
    pub path: std::path::PathBuf,
    /// Id of the content marker that produced the value
    /// (see [`crate::secrets::ContentMarker::id`]).
    pub marker_id: String,
    /// 1-based line number in the file.
    pub line: u32,
    /// blake3 hex fingerprint of the raw value; equals
    /// [`crate::secrets::MaskedValue::value_hash`].
    pub value_hash: String,
}

/// Re-read a file and return the raw value matching `reference`, or `None`
/// when nothing on that line hashes to `reference.value_hash`.
///
/// # Safety
///
/// `path` must be strictly under `dir_root`, otherwise
/// [`Error::PathOutsideTarget`] is returned. The file is re-opened and read
/// fresh with [`ContentScanLimits::default`]; a non-regular file (symlink or
/// device) yields [`Error::NotAFile`].
pub fn reveal_finding(
    path: &Path,
    dir_root: &Path,
    reference: &FindingRef,
) -> Result<EphemeralSecret> {
    contain_and_validate(path, dir_root)?;

    let line = read_line(path, reference.line)?;
    let compiled = compiled_marker_by_id(&reference.marker_id).ok_or_else(|| Error::Other {
        message: format!("unknown content marker {}", reference.marker_id),
    })?;

    for candidate in extract_raw_candidates(compiled, &line) {
        if fingerprint_secret(&candidate) == reference.value_hash {
            return Ok(EphemeralSecret::new(candidate));
        }
    }

    Err(Error::Other {
        message: "finding no longer present in file (content changed since dig)".to_string(),
    })
}

/// Enforce path containment and regular-file-ness before any read.
///
/// Containment uses [`is_path_under_root`] (canonicalize both sides). A file
/// that is a symlink is rejected with [`Error::NotAFile`] — the scan never
/// follows symlinks, so reveal must not either.
fn contain_and_validate(path: &Path, dir_root: &Path) -> Result<()> {
    if !is_path_under_root(path, dir_root)? {
        return Err(Error::PathOutsideTarget {
            path: path.to_path_buf(),
        });
    }
    let meta = std::fs::symlink_metadata(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !meta.file_type().is_file() {
        return Err(Error::NotAFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// Read the 1-based `line` from `path`, honoring [`ContentScanLimits::default`].
///
/// A file larger than the size limit is treated as a miss ("cannot reveal from
/// a large file"), never a panic. Returns an empty string for an out-of-range
/// line, which then fails the hash match upstream.
fn read_line(path: &Path, line: u32) -> Result<String> {
    let limits = ContentScanLimits::default();
    let file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let len = file
        .metadata()
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?
        .len();
    if len == 0 || len > limits.max_file_bytes {
        return Ok(String::new());
    }

    let mut content = Vec::new();
    file.take(limits.max_read_bytes)
        .read_to_end(&mut content)
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let text = String::from_utf8_lossy(&content);
    Ok(text
        .split('\n')
        .nth((line.saturating_sub(1)) as usize)
        .unwrap_or("")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(root: &std::path::Path, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = root.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn aws_ref(line: u32, value_hash: &str) -> FindingRef {
        FindingRef {
            path: std::path::PathBuf::new(),
            marker_id: "aws_access_key".to_string(),
            line,
            value_hash: value_hash.to_string(),
        }
    }

    #[test]
    fn ephemeral_secret_is_not_serializable_and_redacts_debug() {
        let secret = EphemeralSecret::new("AKIASECRETVALUE".to_string());
        assert_eq!(secret.expose(), "AKIASECRETVALUE");
        let debug = format!("{secret:?}");
        assert_eq!(debug, "EphemeralSecret(**)");
        assert!(
            !debug.contains("AKIASECRETVALUE"),
            "Debug must not leak the raw value"
        );
    }

    #[test]
    fn reveal_exact_match_on_prefix_line() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "creds.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        let hash = fingerprint_secret("AKIAABCDEFGHIJKLMNOPQRST");
        let reference = aws_ref(1, &hash);
        let secret = reveal_finding(&path, dir.path(), &reference).unwrap();
        assert_eq!(secret.expose(), "AKIAABCDEFGHIJKLMNOPQRST");
    }

    #[test]
    fn reveal_disambiguates_multiple_candidates_by_hash() {
        let dir = TempDir::new().unwrap();
        // Two tokens on one line; the reference must pick the matching one.
        let path = write(
            dir.path(),
            "tokens.txt",
            b"ghp_AAAABBBBCCCCDDDD ghp_1111222233334444\n",
        );
        let hash = fingerprint_secret("ghp_1111222233334444");
        let reference = FindingRef {
            path: path.clone(),
            marker_id: "github_pat".to_string(),
            line: 1,
            value_hash: hash,
        };
        let secret = reveal_finding(&path, dir.path(), &reference).unwrap();
        assert_eq!(secret.expose(), "ghp_1111222233334444");
    }

    #[test]
    fn reveal_fails_when_hash_does_not_match() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "creds.txt", b"AKIADIFFERENTVALUE123\n");
        let reference = aws_ref(1, fingerprint_secret("AKIASOMETHINGELSE").as_str());
        let err = reveal_finding(&path, dir.path(), &reference).unwrap_err();
        assert!(matches!(
            err,
            Error::Other { message } if message.contains("no longer present")
        ));
    }

    #[test]
    fn reveal_rejects_path_outside_root() {
        let dir = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let path = other.path().join(".env");
        std::fs::write(&path, b"AKIAOUTSIDEROOT12345\n").unwrap();
        let reference = aws_ref(1, fingerprint_secret("AKIAOUTSIDEROOT12345").as_str());
        let err = reveal_finding(&path, dir.path(), &reference).unwrap_err();
        assert!(matches!(err, Error::PathOutsideTarget { path: p } if p == path));
    }

    #[test]
    fn reveal_rejects_symlink_file() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let dir = TempDir::new().unwrap();
            let target = dir.path().join("real.env");
            std::fs::write(&target, b"AKIATARGETFILE1234567\n").unwrap();
            let link = dir.path().join("link.env");
            symlink(&target, &link).unwrap();
            let reference = aws_ref(1, fingerprint_secret("AKIATARGETFILE1234567").as_str());
            let err = reveal_finding(&link, dir.path(), &reference).unwrap_err();
            assert!(matches!(err, Error::NotAFile { .. }));
        }
    }

    #[test]
    fn reveal_out_of_range_line_returns_miss() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "creds.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        let reference = aws_ref(99, fingerprint_secret("AKIAABCDEFGHIJKLMNOPQRST").as_str());
        let err = reveal_finding(&path, dir.path(), &reference).unwrap_err();
        assert!(matches!(err, Error::Other { .. }));
    }

    #[test]
    fn finding_ref_round_trips_serde_without_raw() {
        let reference = FindingRef {
            path: std::path::PathBuf::from("/a/.env"),
            marker_id: "aws_access_key".to_string(),
            line: 1,
            value_hash: "abc123".to_string(),
        };
        let json = serde_json::to_string(&reference).unwrap();
        assert!(
            !json.contains("AKIA"),
            "serde must never carry the raw value"
        );
        let back: FindingRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, reference);
    }
}
