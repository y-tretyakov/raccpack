//! Place a completed stash `.age` archive into the den `secrets/` layout.
//!
//! [`place_secrets_archive`] moves `source_age` (a staging file written by
//! `crate::secrets::write_stash_age`) into
//! `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age` (or
//! `…/{slug}__{batch_id}__secrets.age` when [`PlaceSecretsRequest::batch_id`]
//! is set), applying the den skeleton from [`crate::den::ensure_den`] first.
//! [`place_secrets_archive_ensured`] is the internal no-gate variant for
//! callers that already ran `ensure_den`.
//!
//! INVARIANTS:
//!
//! - The final artifact never leaves `den_root`: the computed relative target
//!   is checked for escaping components (`ParentDir` / `RootDir` / `Prefix`)
//!   before any filesystem mutation.
//! - The move is atomic on a single filesystem (rename); on a cross-device
//!   error the archive is copied and the source removed.
//! - On failure the caller's `source_age` is left untouched (the caller owns
//!   cleanup of its staging file).
//! - No `staging` orphan remains after a successful move: rename consumes the
//!   source file, and a cross-device fallback removes it explicitly.
//! - The finished `.age` is `chmod`ed `0o600` (best-effort on Unix).

use std::fs;
use std::path::PathBuf;

use crate::domain::{Error, Result};

use super::layout::{create_dir_all, ensure_den, set_mode_best_effort};
use super::names::{
    project_slug, secrets_relative_path, secrets_relative_path_token, utc_timestamp_now,
};
use super::place::{move_archive, reject_escaping, validate_name_fragment};

/// Everything [`place_secrets_archive`] needs to know about one stash placement.
#[derive(Debug, Clone)]
pub struct PlaceSecretsRequest {
    /// Root of the den to place the archive into.
    pub den_root: PathBuf,
    /// Project name used to derive the slug (may be a path).
    pub project_name: String,
    /// Completed `.age` staging file to move into the den.
    pub source_age: PathBuf,
    /// Optional explicit UTC timestamp; `None` generates one now. Also drives
    /// the `secrets/{yyyy}/{mm}` directory segments.
    pub timestamp: Option<String>,
    /// Optional custom name token replacing the timestamp in the artifact
    /// filename (`{slug}__{batch_id}__secrets.age`). Must be a safe fragment
    /// (no `/`, `\`, `\0`, `.`/`..`); validated here.
    pub batch_id: Option<String>,
}

/// Outcome of a successful [`place_secrets_archive`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceSecretsResult {
    /// Absolute path of the final archive.
    pub absolute_path: PathBuf,
    /// Path relative to `den_root` (starts with `secrets/`); for manifests later.
    pub relative_path: PathBuf,
    /// Byte size of the final archive.
    pub size_bytes: u64,
}

/// Move/rename `source_age` into the den secrets layout atomically.
///
/// 1. [`ensure_den`] on `req.den_root` (version gate + skeleton) — adds
///    redundant work when the facade has already run it (see
///    `place_secrets_archive_ensured` for the no-gate variant).
/// 2. Derive the slug, timestamp, and the relative
///    `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age` path (the token becomes
///    `req.batch_id` when present, while `yyyy`/`mm` still come from the real
///    timestamp).
/// 3. Reject any relative path that would escape `den_root`.
/// 4. `rename` into place; on cross-device, copy + remove the source.
/// 5. Best-effort `chmod 0o600` (Unix) and return absolute/relative paths and
///    the on-disk byte size.
///
/// # Errors
///
/// - [`Error::DenVersion`] via [`ensure_den`] for an incompatible den.
/// - [`Error::Io`] for any filesystem failure (rename, copy, metadata).
/// - [`Error::Other`] when the derived relative path would escape `den_root`,
///   or when `batch_id` is invalid (empty, `.`/`..`, or containing `/`, `\`,
///   `\0`).
pub fn place_secrets_archive(req: &PlaceSecretsRequest) -> Result<PlaceSecretsResult> {
    ensure_den(&req.den_root)?;
    place_secrets_archive_ensured(req)
}

/// Place `source_age` assuming the den is already initialized.
///
/// Caller guarantees [`ensure_den`] already ran on `req.den_root`, so the
/// version gate and skeleton creation are skipped. Fails with the same
/// [`Error::Io`] / [`Error::Other`] variants as [`place_secrets_archive`].
pub(crate) fn place_secrets_archive_ensured(
    req: &PlaceSecretsRequest,
) -> Result<PlaceSecretsResult> {
    let slug = project_slug(&req.project_name);
    let ts = req.timestamp.clone().unwrap_or_else(utc_timestamp_now);
    let rel = match &req.batch_id {
        Some(batch_id) => {
            validate_name_fragment(batch_id, "stash batch id")?;
            secrets_relative_path_token(&slug, &ts, batch_id)
        }
        None => secrets_relative_path(&slug, &ts),
    };
    reject_escaping(&rel)?;

    let abs = req.den_root.join(&rel);
    if let Some(parent) = abs.parent() {
        create_dir_all(parent)?;
    }

    move_archive(&req.source_age, &abs)?;
    set_mode_best_effort(&abs, 0o600);

    let size_bytes = fs::metadata(&abs)
        .map_err(|source| Error::Io {
            path: abs.clone(),
            source,
        })?
        .len();

    Ok(PlaceSecretsResult {
        absolute_path: abs,
        relative_path: rel,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn place_moves_age_into_secrets_layout() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"age-blob").unwrap();

        let result = place_secrets_archive(&PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "My API".to_string(),
            source_age: age.clone(),
            timestamp: Some("20260804T155230Z".to_string()),
            batch_id: None,
        })
        .unwrap();

        let rel = PathBuf::from("secrets/2026/08/My-API__20260804T155230Z__secrets.age");
        assert_eq!(result.relative_path, rel);
        assert_eq!(result.absolute_path, den.path().join(&rel));
        assert_eq!(result.size_bytes, 8);
        assert!(result.absolute_path.is_file());
        assert!(!age.exists(), "source should be consumed by the move");
    }

    #[test]
    fn place_generates_timestamp_when_absent() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"data").unwrap();

        let result = place_secrets_archive(&PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_age: age,
            timestamp: None,
            batch_id: None,
        })
        .unwrap();

        assert!(result.relative_path.starts_with("secrets/"));
        assert!(result.absolute_path.is_file());
        let file_name = result.absolute_path.file_name().unwrap().to_string_lossy();
        assert!(file_name.starts_with("proj__"));
        assert!(file_name.ends_with("__secrets.age"));
    }

    #[test]
    fn place_batch_id_overrides_name_token_but_not_dir_segments() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"data").unwrap();

        let result = place_secrets_archive(&PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_age: age,
            timestamp: Some("20260804T155230Z".to_string()),
            batch_id: Some("nightly-run".to_string()),
        })
        .unwrap();

        assert_eq!(
            result.relative_path,
            PathBuf::from("secrets/2026/08/proj__nightly-run__secrets.age")
        );
        assert!(result.absolute_path.is_file());
    }

    #[test]
    fn place_creates_den_skeleton() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"x").unwrap();

        place_secrets_archive(&PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_age: age,
            timestamp: Some("20260804T155230Z".to_string()),
            batch_id: None,
        })
        .unwrap();

        assert!(den.path().join(".den-version").is_file());
        assert!(den.path().join("README.txt").is_file());
        assert!(den.path().join("packs").is_dir());
    }

    #[test]
    fn place_rejects_escaping_timestamp() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"data").unwrap();

        let err = place_secrets_archive(&PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_age: age,
            timestamp: Some("../../evil".to_string()),
            batch_id: None,
        })
        .unwrap_err();
        assert!(err.to_string().contains("would escape den root"));
    }

    #[test]
    fn place_rejects_dangerous_batch_id() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"data").unwrap();

        for bad in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
            let err = place_secrets_archive(&PlaceSecretsRequest {
                den_root: den.path().to_path_buf(),
                project_name: "proj".to_string(),
                source_age: age.clone(),
                timestamp: Some("20260804T155230Z".to_string()),
                batch_id: Some(bad.to_string()),
            })
            .unwrap_err();
            assert!(
                err.to_string().contains("invalid stash batch id"),
                "{bad:?} must be rejected, got: {err}"
            );
        }
        assert!(
            age.exists(),
            "source must be untouched on a rejected placement"
        );
    }

    #[test]
    fn place_ensured_skips_validation_and_does_not_bootstrap_den() {
        let den = TempDir::new().unwrap();

        // Uninitialized den: nobody ran `ensure_den`, so the ensured variant
        // must not create `.den-version`.
        let staging = tempfile::TempDir::new().unwrap();
        let age = staging.path().join("secrets.age");
        std::fs::write(&age, b"data").unwrap();
        let req = PlaceSecretsRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_age: age.clone(),
            timestamp: Some("20260804T155230Z".to_string()),
            batch_id: None,
        };
        let _ = place_secrets_archive_ensured(&req);
        assert!(
            !den.path().join(".den-version").exists(),
            "place_secrets_archive_ensured must not create the den skeleton"
        );

        // After an explicit ensure_den the ensured variant places successfully.
        ensure_den(den.path()).unwrap();
        let age2 = staging.path().join("secrets2.age");
        std::fs::write(&age2, b"data").unwrap();
        let placed = place_secrets_archive_ensured(&PlaceSecretsRequest {
            source_age: age2,
            ..req
        })
        .unwrap();
        assert!(placed.absolute_path.is_file());
    }
}
