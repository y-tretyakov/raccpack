//! Raid manifest JSON: the on-disk audit record written after a successful
//! atomic commit (A3.4).
//!
//! A [`DenManifest`] records the outcome of a completed raid run: per-stage
//! outcomes, the paths of the placed artifacts (relative to the den root), the
//! raw-free stash manifest, the tool version, the success / dry-run flags, and
//! the creation timestamp. It is written **only** after a successful commit
//! and **only** when at least one artifact was placed — never in a dry run and
//! never after a rollback.
//!
//! Naming is consistent with packs/secrets:
//! `manifests/{yyyy}/{mm}/{slug}__{ts}__{short_id}.json`, with the same
//! `yyyy`/`mm` fallback as [`super::names::pack_relative_path`].
//!
//! INVARIANTS:
//!
//! - The manifest never escapes the den: [`write_manifest`] rejects a relative
//!   path containing escaping components before any filesystem mutation.
//! - The manifest is **raw-free**: it carries only paths (relative to the
//!   den), risks, sizes, and counters — never secret material or passphrases.
//! - `schema_version` is [`MANIFEST_SCHEMA_VERSION`]; a future change to the
//!   on-disk shape must bump it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{Error, Result};
use crate::secrets::StashManifestEntry;

use super::layout::{create_dir_all, set_mode_best_effort};
use super::place::reject_escaping;

/// Current on-disk manifest schema version.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Audit record of one completed raid run, written to `den/manifests/…`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenManifest {
    /// Schema version of this manifest ([`MANIFEST_SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// raccpack version that produced the run (`CARGO_PKG_VERSION`).
    pub tool_version: String,
    /// UTC `YYYYMMDDThhmmssZ` timestamp of the run.
    pub created_at: String,
    /// Whether every enabled phase succeeded.
    pub success: bool,
    /// Whether the run was a dry run (a dry run never writes a manifest).
    pub dry_run: bool,
    /// Per-phase outcomes in run order.
    pub stages: Vec<ManifestStage>,
    /// Paths of placed artifacts, relative to the den root.
    pub artifacts: ManifestArtifacts,
    /// Raw-free stash manifest entries (empty when stash did not run).
    pub stash_manifest: Vec<StashManifestEntry>,
}

/// On-disk projection of one raid stage (mirrors `RaidStageResult` so the den
/// module does not depend on app types).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestStage {
    /// Phase name: `"stash"` | `"rinse"` | `"pack"` | `"move"`.
    pub name: String,
    /// Whether the phase succeeded.
    pub success: bool,
    /// Human-readable summary (never secret material).
    pub message: String,
    /// Whether the phase did not run (disabled or short-circuited).
    pub skipped: bool,
}

/// Paths of the artifacts placed by the run, relative to the den root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestArtifacts {
    /// `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age` when stash placed one.
    pub secrets_archive: Option<PathBuf>,
    /// `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst` when pack placed one.
    pub project_pack: Option<PathBuf>,
}

/// Relative den path for a raid manifest:
/// `manifests/{yyyy}/{mm}/{slug}__{ts}__{short_id}.json`.
///
/// `yyyy` = `ts[0..4]`, `mm` = `ts[4..6]`; a `ts` shorter than 6 chars falls
/// back to `"0000"` / `"00"` so this never panics (same rule as
/// [`super::names::pack_relative_path`]).
pub fn manifest_relative_path(slug: &str, ts: &str, short_id: &str) -> PathBuf {
    let yyyy = ts.get(..4).unwrap_or("0000");
    let mm = ts.get(4..6).unwrap_or("00");
    PathBuf::from("manifests")
        .join(yyyy)
        .join(mm)
        .join(format!("{slug}__{ts}__{short_id}.json"))
}

/// Write `manifest` to `den_root.join(rel)` as pretty JSON, mode `0o600`.
///
/// Rejects a relative path that would escape the den root, creates the parent
/// directories, serializes with `serde_json::to_string_pretty`, and writes the
/// file. A serialization failure maps to [`Error::Other`]; a filesystem
/// failure maps to [`Error::Io`] on the absolute target path.
///
/// # Errors
///
/// - [`Error::Other`] when `rel` would escape `den_root` or serialization fails.
/// - [`Error::Io`] on any filesystem failure (create dirs, write).
pub fn write_manifest(den_root: &Path, rel: &Path, manifest: &DenManifest) -> Result<()> {
    reject_escaping(rel)?;

    let abs = den_root.join(rel);
    if let Some(parent) = abs.parent() {
        create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(manifest).map_err(|source| Error::Other {
        message: format!("failed to serialize raid manifest: {source}"),
    })?;
    std::fs::write(&abs, json).map_err(|source| Error::Io {
        path: abs.clone(),
        source,
    })?;

    set_mode_best_effort(&abs, 0o600);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn sample_manifest() -> DenManifest {
        DenManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            tool_version: "0.3.0".to_string(),
            created_at: "20260804T155230Z".to_string(),
            success: true,
            dry_run: false,
            stages: vec![ManifestStage {
                name: "pack".to_string(),
                success: true,
                message: "packed 3 files".to_string(),
                skipped: false,
            }],
            artifacts: ManifestArtifacts {
                secrets_archive: None,
                project_pack: Some(PathBuf::from(
                    "packs/2026/08/my-api__20260804T155230Z.tar.zst",
                )),
            },
            stash_manifest: Vec::new(),
        }
    }

    #[test]
    fn manifest_path_derives_year_month() {
        assert_eq!(
            manifest_relative_path("my-api", "20260804T155230Z", "a1b2c3d4"),
            PathBuf::from("manifests/2026/08/my-api__20260804T155230Z__a1b2c3d4.json")
        );
    }

    #[test]
    fn manifest_path_never_panics_on_short_ts() {
        let path = manifest_relative_path("s", "12", "a1b2c3d4");
        assert!(path.to_string_lossy().starts_with("manifests/"));
    }

    #[test]
    fn write_manifest_roundtrips_schema_version() {
        let dir = TempDir::new().unwrap();
        let den = dir.path().join("den");
        let rel = manifest_relative_path("my-api", "20260804T155230Z", "a1b2c3d4");

        write_manifest(&den, &rel, &sample_manifest()).unwrap();

        let abs = den.join(&rel);
        assert!(abs.is_file());
        let parsed: DenManifest = serde_json::from_str(&fs::read_to_string(&abs).unwrap()).unwrap();
        assert_eq!(parsed.schema_version, MANIFEST_SCHEMA_VERSION);
        assert_eq!(parsed, sample_manifest());
    }

    #[test]
    fn write_manifest_rejects_escaping_rel_without_fs_effects() {
        let dir = TempDir::new().unwrap();

        let err =
            write_manifest(dir.path(), Path::new("../evil.json"), &sample_manifest()).unwrap_err();
        assert!(err.to_string().contains("would escape den root"));
        assert_eq!(dir.path().read_dir().unwrap().count(), 0, "nothing written");
    }
}
