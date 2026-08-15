//! Facade use-case `stash`: encrypt sensitive files into the den.
//!
//! [`stash`] selects sensitive files under `opts.target` (same rules as
//! `racc dig`), packs them into one tar, encrypts it with age using a
//! passphrase, and places the `.age` under `den/secrets/{yyyy}/{mm}`. In
//! `RunMode::DryRun` nothing is written and the expected artifact path is
//! reported; in `RunMode::Commit` the archive is written and — only with
//! `opts.remove_sources` — the originals are deleted after a successful
//! placement.
//!
//! INVARIANTS:
//!
//! - **Identity**: only `AgeIdentity::Passphrase` is accepted; recipients are
//!   [`Error::Unsupported`]. The passphrase is held in the zeroizing variant,
//!   borrowed (never cloned) for encryption, and never appears in results,
//!   errors, or `Debug` output.
//! - **DryRun writes nothing**: no `ensure_den`, no staging, no source
//!   removal; `StashResult::archive_path` is the expected final path.
//! - **Fail-safe order**: encrypt → place → (optionally) remove sources.
//!   A failed encrypt/place never removes sources. If removal fails midway
//!   the archive is already in the den and the error documents the partial
//!   state.
//! - **F-PATH-3**: the staging path lives under `den/staging/…` and is
//!   rejected by a canonicalized containment check before any directory is
//!   created, so a den nested inside the project leaves no staging leftovers.
//! - **Manifest is raw-free**: [`StashManifestEntry`]s carry only path, risk,
//!   and size — never file contents.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::den::{
    create_dir_all, ensure_den, place_secrets_archive_ensured, project_slug, secrets_relative_path,
    secrets_relative_path_token, short_id, utc_timestamp_now, validate_name_fragment,
    PlaceSecretsRequest,
};
use crate::domain::{Error, Result, SensitiveRisk};
use crate::scan::canonicalize_existing_prefix;
use crate::secrets::stash_batch::StashManifestEntry;
use crate::secrets::{
    remove_stash_sources, select_files_for_stash, write_stash_age, StashSelectOptions,
};

use super::context::AppContext;
use super::progress::{OperationKind, ProgressEvent, ProgressSink};

/// Options controlling [`stash`].
#[derive(Debug, Clone)]
pub struct StashOptions {
    /// Project or subtree root to scan for sensitive files.
    pub target: PathBuf,
    /// Explicit file list; `None` scans the whole `target` tree.
    pub only_files: Option<Vec<PathBuf>>,
    /// Minimum risk to include; default [`SensitiveRisk::High`].
    pub min_risk: SensitiveRisk,
    /// Delete the original files after a successful Commit (ignored in DryRun).
    pub remove_sources: bool,
    /// Optional name token replacing the timestamp in the artifact filename.
    pub batch_id: Option<String>,
}

impl Default for StashOptions {
    /// [`StashOptions::target`] is empty; the caller must set it before use.
    fn default() -> Self {
        Self {
            target: PathBuf::new(),
            only_files: None,
            min_risk: SensitiveRisk::High,
            remove_sources: false,
            batch_id: None,
        }
    }
}

/// Identity material for encrypting a stash archive.
///
/// A1 supports passphrases only; recipient identities fail with
/// [`Error::Unsupported`]. The passphrase is held zeroizing so no plain
/// `String` copy outlives a call; [`Debug`] output never prints its value.
#[derive(Clone)]
pub enum AgeIdentity {
    /// Scrypt passphrase (zeroized on drop).
    Passphrase(Zeroizing<String>),
    /// Public age recipients (not supported in A1).
    Recipients(Vec<String>),
}

impl std::fmt::Debug for AgeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgeIdentity::Passphrase(_) => write!(f, "Passphrase([redacted])"),
            AgeIdentity::Recipients(recipients) => {
                f.debug_tuple("Recipients").field(recipients).finish()
            }
        }
    }
}

/// Outcome of a [`stash`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashResult {
    /// Final artifact path — the expected path in a dry run.
    pub archive_path: PathBuf,
    /// Number of files archived.
    pub files_archived: usize,
    /// Sum of plaintext source sizes, in bytes.
    pub bytes_archived: u64,
    /// How many source files were deleted (always 0 in a dry run).
    pub removed_sources: usize,
    /// Whether the run was a dry run (nothing written or deleted).
    pub dry_run: bool,
    /// Raw-free manifest entries aligned with the archive.
    pub manifest: Vec<StashManifestEntry>,
}

/// Stash sensitive files under `opts.target` into `ctx.paths.den_dir`.
///
/// # DryRun
///
/// In `RunMode::DryRun` nothing is created under the den: no `ensure_den`, no
/// staging, no source removal. [`StashResult::archive_path`] holds the
/// expected artifact path and [`StashResult::dry_run`] is true. An empty
/// selection is still [`Error::StashEmpty`] (mirrors Commit, so a dry run
/// already surfaces "nothing to stash" before any encryption).
///
/// # Commit
///
/// In `RunMode::Commit` the den skeleton is ensured, selected files are
/// encrypted into `den/staging/{short_id}/secrets.age` and moved to
/// `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age` (the token becomes
/// `opts.batch_id` when set). With `opts.remove_sources` the originals are
/// deleted only after the archive landed in the den.
///
/// # Errors
///
/// - `AgeIdentity::Recipients` → [`Error::Unsupported`]; empty passphrase →
///   [`Error::Encrypt`].
/// - Empty selection → [`Error::StashEmpty`].
/// - Missing / non-directory target → [`Error::PathNotFound`] /
///   [`Error::NotADirectory`].
/// - Invalid [`StashOptions::batch_id`] → [`Error::Other`].
/// - Staging path (canonicalized) inside the project tree → [`Error::Other`].
/// - Any den/encrypt IO failure → [`Error::Io`]; incompatible den →
///   [`Error::DenVersion`].
pub fn stash(
    ctx: &AppContext,
    opts: &StashOptions,
    identity: &AgeIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<StashResult> {
    let passphrase = match identity {
        AgeIdentity::Passphrase(pass) => {
            if pass.is_empty() {
                return Err(Error::Encrypt {
                    message: "passphrase must not be empty".to_string(),
                });
            }
            pass
        }
        AgeIdentity::Recipients(_) => {
            return Err(Error::Unsupported {
                feature: "age recipient identities".to_string(),
            });
        }
    };

    if let Some(batch_id) = &opts.batch_id {
        validate_name_fragment(batch_id, "stash batch id")?;
    }

    progress.emit(stash_event(0, "Selecting sensitive files…", false));

    let select_opts = StashSelectOptions {
        target: opts.target.clone(),
        only_files: opts.only_files.clone(),
        min_risk: opts.min_risk,
        scan_content: true,
    };
    let entries = select_files_for_stash(&select_opts)?;
    if entries.is_empty() {
        return Err(Error::StashEmpty {
            message: "no files matched the current min-risk threshold".into(),
        });
    }

    let slug = project_slug(&opts.target.to_string_lossy());
    let ts = utc_timestamp_now();
    let rel = match &opts.batch_id {
        Some(batch_id) => secrets_relative_path_token(&slug, &ts, batch_id),
        None => secrets_relative_path(&slug, &ts),
    };
    let expected_abs = ctx.paths.den_dir.join(&rel);

    if ctx.mode.is_dry_run() {
        progress.emit(stash_event(100, "Done", true));
        return Ok(StashResult {
            archive_path: expected_abs,
            files_archived: entries.len(),
            bytes_archived: entries.iter().map(|e| e.size_bytes).sum(),
            removed_sources: 0,
            dry_run: true,
            manifest: entries
                .iter()
                .map(|e| StashManifestEntry {
                    original_path: e.path.clone(),
                    risk: e.risk,
                    size_bytes: e.size_bytes,
                })
                .collect(),
        });
    }

    let short = short_id();
    let staging = ctx
        .paths
        .den_dir
        .join("staging")
        .join(&short)
        .join("secrets.age");

    let resolved_staging = canonicalize_existing_prefix(&staging)?;
    let resolved_target = canonicalize_existing_prefix(&opts.target)?;
    if resolved_staging.starts_with(&resolved_target) {
        return Err(Error::Other {
            message:
                "staging path lies inside the project tree; use a den directory outside the project"
                    .to_string(),
        });
    }

    ensure_den(&ctx.paths.den_dir)?;

    let staging_dir = staging.parent().ok_or_else(|| Error::Other {
        message: "invalid den staging path".to_string(),
    })?;
    create_dir_all(staging_dir)?;

    progress.emit(stash_event(30, "Encrypting archive…", false));

    let batch = write_stash_age(&entries, &staging, passphrase).map_err(|err| {
        best_effort_staging_cleanup(&staging);
        err
    })?;

    progress.emit(stash_event(70, "Saving to den…", false));

    let placed = place_secrets_archive_ensured(&PlaceSecretsRequest {
        den_root: ctx.paths.den_dir.clone(),
        project_name: slug,
        source_age: staging.clone(),
        timestamp: Some(ts),
        batch_id: opts.batch_id.clone(),
    })
    .map_err(|err| {
        best_effort_staging_cleanup(&staging);
        err
    })?;

    if let Some(parent) = staging.parent() {
        let _ = fs::remove_dir(parent);
    }

    let mut removed = 0usize;
    if opts.remove_sources {
        progress.emit(stash_event(90, "Removing sources…", false));
        removed = remove_stash_sources(&batch.manifest)?;
    }

    progress.emit(stash_event(100, "Done", true));

    Ok(StashResult {
        archive_path: placed.absolute_path,
        files_archived: batch.files_archived,
        bytes_archived: batch.bytes_archived,
        removed_sources: removed,
        dry_run: false,
        manifest: batch.manifest,
    })
}

/// Build a progress event for the single `"stash"` phase.
fn stash_event(percent: u8, message: impl Into<String>, phase_complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Stash,
        phase: "stash".to_string(),
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
fn best_effort_staging_cleanup(staging: &std::path::Path) {
    let _ = fs::remove_file(staging);
    if let Some(parent) = staging.parent() {
        let _ = fs::remove_dir(parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stash_options_default_min_risk_is_high() {
        let opts = StashOptions::default();
        assert_eq!(opts.min_risk, SensitiveRisk::High);
        assert!(!opts.remove_sources);
        assert!(opts.batch_id.is_none());
        assert!(opts.only_files.is_none());
        assert!(opts.target.as_os_str().is_empty());
    }

    #[test]
    fn stash_event_helper_shape() {
        let event = stash_event(30, "Encrypting archive…", false);
        assert_eq!(event.operation, OperationKind::Stash);
        assert_eq!(event.phase, "stash");
        assert_eq!(event.phase_index, 0);
        assert_eq!(event.phase_count, 1);
        assert_eq!(event.percent, 30);
        assert_eq!(event.overall_percent, 30);
        assert!(!event.phase_complete);
        assert_eq!(event.message, "Encrypting archive…");

        let done = stash_event(100, "Done", true);
        assert!(done.phase_complete);
        assert_eq!(done.overall_percent, 100);
    }
}
