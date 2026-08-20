//! Facade use-case `pack`: archive a project tree into the den.
//!
//! [`pack`] validates the project root, prepares a staging file inside the den,
//! runs [`crate::archive::pack_tree`] with name-deny always on and optional
//! content-deny, and moves the finished archive into the `packs/{yyyy}/{mm}`
//! layout via [`crate::den::place_pack`] — the facade runs `ensure_den` once
//! itself and then calls the internal ensured variant, avoiding a redundant
//! second `ensure_den` inside the placement step. In `RunMode::DryRun` nothing
//! is written under the den and only the expected artifact path is reported.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::archive::{pack_tree, ContentDenyOptions, PackTreeOptions};
use crate::den::{
    create_dir_all, ensure_den, place_pack_ensured, project_slug, short_id, staging_pack_path,
    utc_timestamp_now, validate_output_name, PlacePackRequest,
};
use crate::domain::{Error, Result, SensitiveRisk};
use crate::scan::skip::SkipPolicy;
use crate::scan::walk::ensure_scan_root;

use super::context::AppContext;
use super::progress::{OperationKind, ProgressEvent, ProgressSink};

mod naming;

use naming::{artifact_rel, resolve_artifact_name};

/// Options controlling [`pack`].
#[derive(Debug, Clone)]
pub struct PackOptions {
    /// Directory to archive (must exist and be a directory).
    pub project: PathBuf,
    /// Optional custom artifact name (without `.tar.zst`); `None` → auto
    /// `{slug}__{ts}.tar.zst`.
    pub output_name: Option<String>,
    /// Scan file contents before packing and skip Critical-hit files when true.
    ///
    /// Defaults to **true**: a standalone `pack` never ships known secret
    /// values. Name-based deny is always active regardless of this flag.
    pub deny_content_secrets: bool,
    /// Optional zstd compression level; `None` uses the crate default (3).
    ///
    /// The facade contract says `None` → `config.advanced.zstd_level`, but that
    /// config field does not exist yet on MVP; this stage self-hosts a default
    /// of 3.
    pub zstd_level: Option<u32>,
    /// Stage-only mode for atomic raid: write `{dir}/pack.tar.zst` without
    /// placement into the den; `output` reports the final expected path and
    /// `size_bytes` is read from the staged archive. `None` → normal Commit
    /// placement.
    pub staging_dir: Option<PathBuf>,
    /// Paths omitted from the archive silently (not counted as secret denies).
    ///
    /// Used by the atomic raid to exclude files already stashed, mirroring the
    /// fail-fast commit where stash removed them before pack ran.
    pub exclude_files: Vec<PathBuf>,
}

impl Default for PackOptions {
    /// [`PackOptions::project`] is empty; the caller must set it before use.
    fn default() -> Self {
        Self {
            project: PathBuf::new(),
            output_name: None,
            deny_content_secrets: true,
            zstd_level: None,
            staging_dir: None,
            exclude_files: Vec::new(),
        }
    }
}

/// Outcome of a [`pack`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackResult {
    /// The archive's source directory.
    pub source: PathBuf,
    /// Final artifact path — the expected path in a dry run.
    pub output: PathBuf,
    /// Byte size of the finished archive (0 in a dry run).
    pub size_bytes: u64,
    /// Number of regular files included (0 in a dry run).
    pub file_count: usize,
    /// Number of files omitted by name/content deny rules (0 in a dry run).
    pub skipped_secret_files: usize,
    /// Whether the run was a dry run (no files written under the den).
    pub dry_run: bool,
}

/// Pack `opts.project` into `ctx.paths.den_dir`.
///
/// # DryRun
///
/// In `RunMode::DryRun` nothing is created under the den: no `ensure_den`, no
/// staging, no uniqueness suffix. [`PackResult::output`] holds the expected
/// artifact path and [`PackResult::dry_run`] is true.
///
/// # Commit
///
/// In `RunMode::Commit` the den skeleton is ensured, the project is packed
/// into `den/staging/{short_id}/` and moved into
/// `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`. With
/// [`PackOptions::output_name`] set the filename becomes `{name}.tar.zst`
/// (still under `packs/{yyyy}/{mm}`).
///
/// With [`PackOptions::staging_dir`] set, Commit is stage-only: the archive is
/// written to `{staging_dir}/pack.tar.zst` without placement, and
/// [`PackResult::output`] reports the final expected path while
/// [`PackResult::size_bytes`] is read from the staged file (used by the atomic
/// raid, which defers placement to its commit phase). `ensure_den` is skipped
/// and the containment check still runs.
///
/// # Uniqueness
///
/// If the target artifact already exists (e.g. a second pack in the same
/// second), a short-id suffix is appended — to the timestamp in the auto-name
/// case, or to the custom filename in the [`PackOptions::output_name`] case.
/// The suffix is applied once; a still-existing target after that
/// (astronomically unlikely) fails with [`Error::Other`].
///
/// # Errors
///
/// - Missing / non-directory project → [`Error::PathNotFound`] /
///   [`Error::NotADirectory`] via [`ensure_scan_root`].
/// - Invalid [`PackOptions::output_name`] (empty, `.`/`..`, or containing
///   `/`, `\`, `\0`) → [`Error::Other`].
/// - Den placed inside the project tree (staging under the project) →
///   [`Error::Other`].
/// - Any den/pack IO failure → [`Error::Io`]; an incompatible den →
///   [`Error::DenVersion`].
pub fn pack(
    ctx: &AppContext,
    opts: &PackOptions,
    progress: &mut dyn ProgressSink,
) -> Result<PackResult> {
    let project = opts.project.clone();
    ensure_scan_root(&project)?;

    if let Some(name) = &opts.output_name {
        validate_output_name(name)?;
    }

    let den = ctx.paths.den_dir.clone();
    let slug = project_slug(
        &project
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| project.to_string_lossy().into_owned()),
    );

    progress.emit(pack_event(0, "Preparing pack…", false));

    let ts = utc_timestamp_now();
    let expected_abs = den.join(artifact_rel(&slug, &ts, opts.output_name.as_deref()));

    if ctx.mode.is_dry_run() {
        progress.emit(pack_event(100, "Done", true));
        return Ok(PackResult {
            source: project,
            output: expected_abs,
            size_bytes: 0,
            file_count: 0,
            skipped_secret_files: 0,
            dry_run: true,
        });
    }

    if opts.staging_dir.is_none() {
        ensure_den(&den)?;
    }

    let (ts, output_name) = resolve_artifact_name(&den, &slug, &ts, opts.output_name.as_deref())?;

    let staging = match &opts.staging_dir {
        Some(dir) => dir.join("pack.tar.zst"),
        None => staging_pack_path(&den, &short_id()),
    };
    create_dir_all(staging.parent().ok_or_else(|| Error::Other {
        message: "invalid den staging path".to_string(),
    })?)?;

    if staging.starts_with(&project) {
        return Err(Error::Other {
            message:
                "staging path lies inside the project tree; use a den directory outside the project"
                    .to_string(),
        });
    }

    progress.emit(pack_event(30, "Archiving…", false));

    let tree_opts = PackTreeOptions {
        policy: SkipPolicy::default_scan(),
        max_depth: PackTreeOptions::default().max_depth,
        zstd_level: opts.zstd_level.unwrap_or(3) as i32,
        deny_name_secrets: true,
        content_deny: ContentDenyOptions {
            enabled: opts.deny_content_secrets,
            min_risk: SensitiveRisk::Critical,
        },
        exclude_files: opts.exclude_files.clone(),
    };
    let tree = pack_tree(&project, &staging, &tree_opts).map_err(|err| {
        best_effort_staging_cleanup(&staging);
        err
    })?;

    if opts.staging_dir.is_some() {
        let output = den.join(artifact_rel(&slug, &ts, output_name.as_deref()));
        let size_bytes = fs::metadata(&staging)
            .map_err(|source| Error::Io {
                path: staging.clone(),
                source,
            })?
            .len();
        progress.emit(pack_event(100, "Done", true));
        return Ok(PackResult {
            source: project,
            output,
            size_bytes,
            file_count: tree.file_count,
            skipped_secret_files: tree.skipped_secret_files,
            dry_run: false,
        });
    }

    progress.emit(pack_event(80, "Moving to den…", false));

    let placed = place_pack_ensured(&PlacePackRequest {
        den_root: den,
        project_name: slug,
        source_archive: staging.clone(),
        timestamp: Some(ts),
        output_name,
    })
    .map_err(|err| {
        best_effort_staging_cleanup(&staging);
        err
    })?;

    if let Some(parent) = staging.parent() {
        let _ = fs::remove_dir(parent);
    }

    progress.emit(pack_event(100, "Done", true));

    Ok(PackResult {
        source: project,
        output: placed.absolute_path,
        size_bytes: placed.size_bytes,
        file_count: tree.file_count,
        skipped_secret_files: tree.skipped_secret_files,
        dry_run: false,
    })
}

/// Build a progress event for the single `"pack"` phase.
fn pack_event(percent: u8, message: impl Into<String>, phase_complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Pack,
        phase: "pack".to_string(),
        phase_index: 0,
        phase_count: 1,
        percent,
        overall_percent: percent,
        message: message.into(),
        phase_complete,
    }
}

/// Best-effort removal of the staging partial file and its parent directory.
///
/// Cleanup failures are ignored so the caller's original error is returned
/// unchanged on the error paths that invoke this.
fn best_effort_staging_cleanup(staging: &Path) {
    let _ = fs::remove_file(staging);
    if let Some(parent) = staging.parent() {
        let _ = fs::remove_dir(parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_options_default_denies_content() {
        let opts = PackOptions::default();
        assert!(opts.deny_content_secrets, "content deny must default to on");
        assert!(opts.output_name.is_none());
        assert!(opts.zstd_level.is_none());
        assert!(opts.staging_dir.is_none());
        assert!(opts.project.as_os_str().is_empty());
    }

    #[test]
    fn pack_event_helper_shape() {
        let event = pack_event(30, "Archiving…", false);
        assert_eq!(event.operation, OperationKind::Pack);
        assert_eq!(event.phase, "pack");
        assert_eq!(event.phase_index, 0);
        assert_eq!(event.phase_count, 1);
        assert_eq!(event.percent, 30);
        assert_eq!(event.overall_percent, 30);
        assert!(!event.phase_complete);
        assert_eq!(event.message, "Archiving…");

        let done = pack_event(100, "Done", true);
        assert!(done.phase_complete);
        assert_eq!(done.overall_percent, 100);
    }

    #[test]
    fn output_name_validation_rejects_dangerous_names() {
        for bad in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
            assert!(
                validate_output_name(bad).is_err(),
                "{bad:?} must be rejected"
            );
        }
        for good in ["my-api", "a.tar.zst", "name with spaces", "a.b-c_d"] {
            assert!(
                validate_output_name(good).is_ok(),
                "{good:?} must be accepted"
            );
        }
    }
}
