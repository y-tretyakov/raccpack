//! Place a completed pack archive into the den `packs/` layout.
//!
//! [`place_pack`] moves `source_archive` (a completed tempfile/staging file
//! from `crate::archive::pack_tree`) into
//! `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst` (or `{name}.tar.zst` when
//! [`PlacePackRequest::output_name`] is set), applying the den skeleton from
//! [`crate::den::ensure_den`] first.
//!
//! INVARIANTS:
//!
//! - The final artifact never leaves `den_root`: the computed relative target
//!   is checked for escaping components (`ParentDir` / `RootDir` / `Prefix`)
//!   before any filesystem mutation.
//! - The move is atomic on a single filesystem (rename); on a cross-device
//!   error the archive is copied and the source removed.
//! - On failure the caller's `source_archive` is left untouched (the caller
//!   owns cleanup of its tempfile).
//! - No `staging` orphan remains after a successful move: rename consumes the
//!   source file, and a cross-device fallback removes it explicitly.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::domain::{Error, Result};

use super::layout::{create_dir_all, ensure_den, set_mode_best_effort};
use super::names::{pack_relative_path, project_slug, utc_timestamp_now};

/// Everything [`place_pack`] needs to know about one pack placement.
#[derive(Debug, Clone)]
pub struct PlacePackRequest {
    /// Root of the den to place the archive into.
    pub den_root: PathBuf,
    /// Project name used to derive the slug (may be a path).
    pub project_name: String,
    /// Completed archive (tempfile from [`crate::archive::pack_tree`]) to move.
    pub source_archive: PathBuf,
    /// Optional explicit UTC timestamp; `None` generates one now.
    pub timestamp: Option<String>,
    /// Optional custom artifact filename (without `.tar.zst`); `None` →
    /// `{slug}__{ts}.tar.zst`.
    pub output_name: Option<String>,
}

/// Outcome of a successful [`place_pack`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacePackResult {
    /// Absolute path of the final archive.
    pub absolute_path: PathBuf,
    /// Path relative to `den_root` (starts with `packs/`); for manifests later.
    pub relative_path: PathBuf,
    /// Byte size of the final archive.
    pub size_bytes: u64,
}

/// Move/rename `source_archive` into the den packs layout atomically.
///
/// 1. [`ensure_den`] on `req.den_root` (version gate + skeleton).
/// 2. Derive the slug and timestamp, then the relative
///    `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst` path. When
///    [`PlacePackRequest::output_name`] is set the artifact filename becomes
///    `{output_name}.tar.zst` (still under `packs/{yyyy}/{mm}`).
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
///   or when `output_name` is invalid (empty, `.`/`..`, or containing `/`,
///   `\`, `\0`).
pub fn place_pack(req: &PlacePackRequest) -> Result<PlacePackResult> {
    ensure_den(&req.den_root)?;

    let slug = project_slug(&req.project_name);
    let ts = req.timestamp.clone().unwrap_or_else(utc_timestamp_now);
    let rel = match &req.output_name {
        Some(name) => {
            validate_output_name(name)?;
            let rel = pack_relative_path(&slug, &ts);
            rel.with_file_name(format!("{name}.tar.zst"))
        }
        None => pack_relative_path(&slug, &ts),
    };
    reject_escaping(&rel)?;

    let abs = req.den_root.join(&rel);
    if let Some(parent) = abs.parent() {
        create_dir_all(parent)?;
    }

    move_archive(&req.source_archive, &abs)?;
    set_mode_best_effort(&abs, 0o600);

    let size_bytes = fs::metadata(&abs)
        .map_err(|source| Error::Io {
            path: abs.clone(),
            source,
        })?
        .len();

    Ok(PlacePackResult {
        absolute_path: abs,
        relative_path: rel,
        size_bytes,
    })
}

/// Reject a relative den path containing `..`, `/`, or `Prefix` components.
///
/// Defense in depth: the slug is already restricted to `[a-zA-Z0-9._-]`, but
/// a caller-supplied `timestamp` could smuggle in path separators.
fn reject_escaping(rel: &Path) -> Result<()> {
    let escapes = rel.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if escapes {
        Err(Error::Other {
            message: format!("den path would escape den root: {}", rel.display()),
        })
    } else {
        Ok(())
    }
}

/// Validate a custom artifact filename (without `.tar.zst`).
///
/// Rejects empty names, `.` and `..`, and names containing `/`, `\` or a NUL
/// byte. Shared by [`place_pack`] and the facade `pack` use-case so both apply
/// the identical rule.
pub(crate) fn validate_output_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Err(Error::Other {
            message: format!("invalid pack output name: {name:?}"),
        });
    }
    Ok(())
}

/// Rename `source` to `destination`, falling back to copy + remove on
/// cross-device moves.
fn move_archive(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(err) if is_cross_devices(&err) => {
            fs::copy(source, destination).map_err(|source_err| Error::Io {
                path: destination.to_path_buf(),
                source: source_err,
            })?;
            fs::remove_file(source).map_err(|source_err| Error::Io {
                path: source.to_path_buf(),
                source: source_err,
            })
        }
        Err(err) => Err(Error::Io {
            path: source.to_path_buf(),
            source: err,
        }),
    }
}

/// Detect a cross-filesystem rename failure.
///
/// `ErrorKind::CrossesDevices` is stable only since Rust 1.85, while the
/// workspace MSRV is 1.75, so detect via the raw OS error instead: EXDEV (18)
/// on POSIX, `ERROR_NOT_SAME_DEVICE` (17) on Windows.
fn is_cross_devices(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(18) | Some(17))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn place_moves_archive_into_packs_layout() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let archive = staging.path().join("pack.tar.zst");
        std::fs::write(&archive, b"zstd-blob").unwrap();

        let result = place_pack(&PlacePackRequest {
            den_root: den.path().to_path_buf(),
            project_name: "My App".to_string(),
            source_archive: archive.clone(),
            timestamp: Some("20260804T155230Z".to_string()),
            output_name: None,
        })
        .unwrap();

        let rel = PathBuf::from("packs/2026/08/My-App__20260804T155230Z.tar.zst");
        assert_eq!(result.relative_path, rel);
        assert_eq!(result.absolute_path, den.path().join(&rel));
        assert_eq!(result.size_bytes, 9);
        assert!(result.absolute_path.is_file());
        assert!(!archive.exists(), "source should be consumed by the move");
    }

    #[test]
    fn place_generates_timestamp_when_absent() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let archive = staging.path().join("pack.tar.zst");
        std::fs::write(&archive, b"data").unwrap();

        let result = place_pack(&PlacePackRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_archive: archive,
            timestamp: None,
            output_name: None,
        })
        .unwrap();

        assert!(result.relative_path.starts_with("packs/"));
        assert!(result.absolute_path.is_file());
        let file_name = result.absolute_path.file_name().unwrap().to_string_lossy();
        assert!(file_name.starts_with("proj__"));
        assert!(file_name.ends_with(".tar.zst"));
    }

    #[test]
    fn place_creates_den_skeleton() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let archive = staging.path().join("pack.tar.zst");
        std::fs::write(&archive, b"x").unwrap();

        place_pack(&PlacePackRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_archive: archive,
            timestamp: Some("20260804T155230Z".to_string()),
            output_name: None,
        })
        .unwrap();

        assert!(den.path().join(".den-version").is_file());
        assert!(den.path().join("README.txt").is_file());
        assert!(den.path().join("secrets").is_dir());
    }

    #[test]
    fn place_rejects_escaping_timestamp() {
        let den = TempDir::new().unwrap();
        let staging = TempDir::new().unwrap();
        let archive = staging.path().join("pack.tar.zst");
        std::fs::write(&archive, b"data").unwrap();

        let err = place_pack(&PlacePackRequest {
            den_root: den.path().to_path_buf(),
            project_name: "proj".to_string(),
            source_archive: archive,
            timestamp: Some("../../evil".to_string()),
            output_name: None,
        })
        .unwrap_err();
        assert!(err.to_string().contains("would escape den root"));
    }
}
