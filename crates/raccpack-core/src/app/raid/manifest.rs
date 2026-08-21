//! App-layer mapping for the post-commit raid manifest (A3.4).
//!
//! This is the single place that maps facade result types ([`RaidStageResult`],
//! [`StashResult`], [`PackResult`]) onto the den-owned
//! [`crate::den::DenManifest`] on-disk shape, then writes it after a successful
//! atomic commit. The den module itself stays free of app types.
//!
//! INVARIANTS:
//!
//! - The manifest is written only from the successful Atomic commit path and
//!   only when at least one artifact was placed (an empty artifact list writes
//!   nothing).
//! - Artifact paths in the manifest are relative to the den root; a path that
//!   does not live under the den is kept absolute (defensive, never a panic).
//! - Only raw-free data crosses into the manifest: paths, risks, sizes, and
//!   counters. No secrets, no passphrases.

use std::path::{Path, PathBuf};

use crate::app::context::AppContext;
use crate::app::pack::PackResult;
use crate::app::stash::StashResult;
use crate::den::{
    manifest_relative_path, project_slug, utc_timestamp_now, write_manifest, DenManifest,
    ManifestArtifacts, ManifestStage, MANIFEST_SCHEMA_VERSION,
};
use crate::domain::Result;

use super::{RaidOptions, RaidStageResult};

/// Write the post-commit raid manifest for a successful atomic run.
///
/// The artifact filename is `manifests/{yyyy}/{mm}/{slug}__{ts}__{raid_id}.json`,
/// consistent with the packs/secrets naming. Caller guarantees the commit
/// already placed at least one artifact and the run is not a dry run.
///
/// # Errors
///
/// Propagates [`crate::den::write_manifest`] errors (escaping relative path,
/// serialization, or filesystem failure).
pub(super) fn write_raid_manifest(
    ctx: &AppContext,
    opts: &RaidOptions,
    raid_id: &str,
    stash_result: &Option<StashResult>,
    pack_result: &Option<PackResult>,
    stages: &[RaidStageResult],
) -> Result<()> {
    let slug = project_slug(&opts.project.to_string_lossy());
    let ts = utc_timestamp_now();
    let rel = manifest_relative_path(&slug, &ts, raid_id);
    let manifest = build_raid_manifest(&ctx.paths.den_dir, &ts, stash_result, pack_result, stages);
    write_manifest(&ctx.paths.den_dir, &rel, &manifest)
}

/// Build the on-disk manifest from the facade result types.
///
/// Pure and testable: `den_root` is passed in so artifact paths can be
/// relativized without filesystem access.
fn build_raid_manifest(
    den_root: &Path,
    ts: &str,
    stash_result: &Option<StashResult>,
    pack_result: &Option<PackResult>,
    stages: &[RaidStageResult],
) -> DenManifest {
    DenManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: ts.to_string(),
        success: true,
        dry_run: false,
        stages: stages.iter().map(ManifestStage::from).collect(),
        artifacts: ManifestArtifacts {
            secrets_archive: stash_result
                .as_ref()
                .map(|result| den_relative(den_root, &result.archive_path)),
            project_pack: pack_result
                .as_ref()
                .map(|result| den_relative(den_root, &result.output)),
        },
        stash_manifest: stash_result
            .as_ref()
            .map(|result| result.manifest.clone())
            .unwrap_or_default(),
    }
}

impl From<&RaidStageResult> for ManifestStage {
    fn from(stage: &RaidStageResult) -> Self {
        Self {
            name: stage.name.clone(),
            success: stage.success,
            message: stage.message.clone(),
            skipped: stage.skipped,
        }
    }
}

/// Relativize an absolute artifact path against the den root.
///
/// Returns the path unchanged when it is not under the den (defensive — the
/// atomic commit always places artifacts under `den_dir`).
fn den_relative(den_root: &Path, abs: &Path) -> PathBuf {
    abs.strip_prefix(den_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| abs.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::pack::PackResult;
    use crate::app::stash::StashResult;
    use crate::domain::SensitiveRisk;
    use crate::secrets::StashManifestEntry;

    use super::*;

    #[test]
    fn build_maps_stages_and_relativizes_artifacts() {
        let den = PathBuf::from("/tmp/den");
        let stages = vec![
            RaidStageResult {
                name: "stash".to_string(),
                success: true,
                message: "stashed 1 files".to_string(),
                skipped: false,
            },
            RaidStageResult {
                name: "rinse".to_string(),
                success: true,
                message: "removed 2 directories".to_string(),
                skipped: false,
            },
            RaidStageResult {
                name: "pack".to_string(),
                success: true,
                message: "packed 5 files".to_string(),
                skipped: false,
            },
        ];
        let stash = Some(StashResult {
            archive_path: den.join("secrets/2026/08/proj__20260804T155230Z__secrets.age"),
            files_archived: 1,
            bytes_archived: 12,
            removed_sources: 0,
            dry_run: false,
            manifest: vec![StashManifestEntry {
                original_path: PathBuf::from("/tmp/proj/.env"),
                risk: SensitiveRisk::High,
                size_bytes: 12,
            }],
        });
        let pack = Some(PackResult {
            source: PathBuf::from("/tmp/proj"),
            output: den.join("packs/2026/08/proj__20260804T155230Z.tar.zst"),
            size_bytes: 100,
            file_count: 5,
            skipped_secret_files: 0,
            dry_run: false,
        });

        let manifest = build_raid_manifest(&den, "20260804T155230Z", &stash, &pack, &stages);

        assert_eq!(manifest.schema_version, MANIFEST_SCHEMA_VERSION);
        assert_eq!(manifest.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.created_at, "20260804T155230Z");
        assert!(manifest.success);
        assert!(!manifest.dry_run);

        assert_eq!(manifest.stages.len(), 3);
        assert_eq!(manifest.stages[0].name, "stash");
        assert!(manifest.stages[0].success);
        assert!(!manifest.stages[0].skipped);
        assert_eq!(manifest.stages[2].message, "packed 5 files");

        assert_eq!(
            manifest.artifacts.secrets_archive,
            Some(PathBuf::from(
                "secrets/2026/08/proj__20260804T155230Z__secrets.age"
            ))
        );
        assert_eq!(
            manifest.artifacts.project_pack,
            Some(PathBuf::from(
                "packs/2026/08/proj__20260804T155230Z.tar.zst"
            ))
        );

        assert_eq!(manifest.stash_manifest.len(), 1);
        assert_eq!(
            manifest.stash_manifest[0].original_path,
            PathBuf::from("/tmp/proj/.env")
        );
        assert_eq!(manifest.stash_manifest[0].risk, SensitiveRisk::High);
    }

    #[test]
    fn build_without_results_has_no_artifacts_and_empty_stages() {
        let manifest =
            build_raid_manifest(Path::new("/tmp/den"), "20260804T155230Z", &None, &None, &[]);

        assert!(manifest.artifacts.secrets_archive.is_none());
        assert!(manifest.artifacts.project_pack.is_none());
        assert!(manifest.stash_manifest.is_empty());
        assert!(manifest.stages.is_empty());
        assert!(manifest.success);
    }

    #[test]
    fn den_relative_keeps_outside_paths_absolute() {
        let den = Path::new("/tmp/den");
        assert_eq!(
            den_relative(den, Path::new("/tmp/den/packs/a.tar.zst")),
            PathBuf::from("packs/a.tar.zst")
        );
        assert_eq!(
            den_relative(den, Path::new("/elsewhere/a.age")),
            PathBuf::from("/elsewhere/a.age")
        );
    }
}
