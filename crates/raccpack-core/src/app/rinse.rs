//! Facade use-case `rinse`: remove build-artifact directories (A2.2).
//!
//! [`rinse`] discovers trash directories under `opts.target` matching the
//! selected cleanup strategies, reports their combined size, and — in
//! `RunMode::Commit` — deletes them. In `RunMode::DryRun` nothing is deleted;
//! the same found list and expected bytes are returned with `dry_run = true`.
//!
//! INVARIANTS:
//!
//! - **DryRun writes nothing**: no directory is created, removed, or mutated;
//!   `RinseResult::removed` is the full found list.
//! - **Commit removes only contained, non-symlink trash dirs**: every
//!   destructive delete is preceded by a canonicalized containment check
//!   (`is_path_under_root`) and symlink entries are skipped by
//!   `remove_trash_dir` returning `Ok(0)`.
//! - **Fail-fast partial failure**: the first removal error aborts the run;
//!   already-removed directories stay removed and are reported in
//!   `RinseResult::removed`.
//! - **Raw-free**: [`TrashDir`] carries only path, strategy, pattern name, and
//!   size — never file contents.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clean::{find_trash_dirs, remove_trash_dir, DetectTrashOptions, StrategyId, TrashDir};
use crate::config::RaccConfig;
use crate::domain::{Error, Result};
use crate::scan::is_path_under_root;

use super::context::AppContext;
use super::progress::{OperationKind, ProgressEvent, ProgressSink};

/// Options controlling [`rinse`].
#[derive(Debug, Clone)]
pub struct RinseOptions {
    /// Project or subtree root to scan for trash directories.
    pub target: PathBuf,
    /// Explicit strategy ids; `None` → `config.cleanup.enabled_strategies`.
    pub strategies: Option<Vec<String>>,
    /// Reserved: MVP has no custom patterns in config, so this is a no-op.
    pub include_custom_patterns: bool,
    /// Scan-only mode for atomic raid: report found dirs without deleting
    /// anything (removal is deferred to the raid commit). Ignored in DryRun.
    pub collect_only: bool,
}

impl Default for RinseOptions {
    /// [`RinseOptions::target`] is empty; the caller must set it before use.
    fn default() -> Self {
        Self {
            target: PathBuf::new(),
            strategies: None,
            include_custom_patterns: false,
            collect_only: false,
        }
    }
}

/// Outcome of a [`rinse`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RinseResult {
    /// DryRun: all found dirs; Commit: successfully removed dirs.
    pub removed: Vec<TrashDir>,
    /// Total bytes freed (recomputed at removal time in Commit).
    pub bytes_freed: u64,
    /// Whether the run was a dry run (nothing deleted).
    pub dry_run: bool,
}

/// Discover and (in Commit) remove trash directories under `opts.target`.
///
/// # DryRun
///
/// In `RunMode::DryRun` the found directories are returned unchanged with
/// `dry_run = true`; no directory is touched.
///
/// # Commit
///
/// In `RunMode::Commit` each found directory passes a canonicalized
/// containment check under `opts.target` and is removed with
/// [`remove_trash_dir`]; symlink entries are skipped. The first removal error
/// aborts the run; directories already removed stay removed.
///
/// # Errors
///
/// - Unknown strategy id in `opts.strategies` (or config) → [`Error::Config`].
/// - Missing / non-directory target → [`Error::PathNotFound`] /
///   [`Error::NotADirectory`].
/// - A trash dir not contained under the target → [`Error::PathOutsideTarget`].
/// - Any removal IO failure → [`Error::Io`].
pub fn rinse(
    ctx: &AppContext,
    opts: &RinseOptions,
    progress: &mut dyn ProgressSink,
) -> Result<RinseResult> {
    progress.emit(rinse_event(0, "Scanning for build artifacts…", false));

    let strategy_ids = resolve_strategy_ids(&opts.strategies, &ctx.config)?;
    let dirs = find_trash_dirs(&DetectTrashOptions {
        target: opts.target.clone(),
        strategy_ids,
        max_depth: ctx.config.scanner.max_depth,
        compute_size: true,
    })?;

    let bytes = dirs.iter().map(|dir| dir.size_bytes).sum();
    progress.emit(rinse_event(
        40,
        format!("Found {} directories ({})", dirs.len(), format_mib(bytes)),
        false,
    ));

    if opts.collect_only {
        return Ok(RinseResult {
            removed: dirs,
            bytes_freed: bytes,
            dry_run: ctx.mode.is_dry_run(),
        });
    }

    if ctx.mode.is_dry_run() {
        progress.emit(rinse_event(100, "Done", true));
        return Ok(RinseResult {
            removed: dirs,
            bytes_freed: bytes,
            dry_run: true,
        });
    }

    progress.emit(rinse_event(70, "Removing…", false));

    let (removed, freed) = remove_trash_dirs(&opts.target, &dirs)?;

    progress.emit(rinse_event(100, "Done", true));
    Ok(RinseResult {
        removed,
        bytes_freed: freed,
        dry_run: false,
    })
}

/// Remove every contained trash dir, returning the actually removed dirs and
/// the freed bytes.
///
/// Shared by the Commit path of [`rinse`] and the atomic raid commit: each
/// directory passes a canonicalized containment check under `target` before
/// [`remove_trash_dir`]; symlink entries are skipped. The first removal error
/// aborts the run; directories already removed stay removed.
pub(super) fn remove_trash_dirs(target: &Path, dirs: &[TrashDir]) -> Result<(Vec<TrashDir>, u64)> {
    let mut removed = Vec::new();
    let mut freed = 0u64;
    for dir in dirs {
        if !is_path_under_root(&dir.path, target)? {
            return Err(Error::PathOutsideTarget {
                path: dir.path.clone(),
            });
        }
        let freed_bytes = remove_trash_dir(&dir.path)?;
        freed = freed.saturating_add(freed_bytes);
        if freed_bytes > 0 {
            removed.push(dir.clone());
        }
    }
    Ok((removed, freed))
}

/// Parse the effective strategy ids, from `opts.strategies` when set, else
/// `config.cleanup.enabled_strategies`. Unknown ids fail with [`Error::Config`].
fn resolve_strategy_ids(
    opts_strategies: &Option<Vec<String>>,
    config: &RaccConfig,
) -> Result<Vec<StrategyId>> {
    let ids = match opts_strategies {
        Some(ids) => ids.clone(),
        None => config.cleanup.enabled_strategies.clone(),
    };
    ids.into_iter()
        .map(|id| {
            StrategyId::from_str_ignore_case(&id).ok_or_else(|| Error::Config {
                message: format!("unknown cleanup strategy `{id}`"),
            })
        })
        .collect()
}

/// Build a progress event for the single `"rinse"` phase.
fn rinse_event(percent: u8, message: impl Into<String>, phase_complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Rinse,
        phase: "rinse".to_string(),
        phase_index: 0,
        phase_count: 1,
        percent,
        overall_percent: percent,
        message: message.into(),
        phase_complete,
    }
}

/// Format a byte count as MiB with one decimal place, e.g. `"2.5 MiB"`.
fn format_mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rinse_options_default_shape() {
        let opts = RinseOptions::default();
        assert!(opts.target.as_os_str().is_empty());
        assert!(opts.strategies.is_none());
        assert!(!opts.include_custom_patterns);
        assert!(!opts.collect_only);
    }

    #[test]
    fn rinse_event_helper_shape() {
        let event = rinse_event(40, "Found 3 directories", false);
        assert_eq!(event.operation, OperationKind::Rinse);
        assert_eq!(event.phase, "rinse");
        assert_eq!(event.phase_index, 0);
        assert_eq!(event.phase_count, 1);
        assert_eq!(event.percent, 40);
        assert_eq!(event.overall_percent, 40);
        assert!(!event.phase_complete);
        assert_eq!(event.message, "Found 3 directories");

        let done = rinse_event(100, "Done", true);
        assert!(done.phase_complete);
        assert_eq!(done.overall_percent, 100);
    }

    #[test]
    fn format_mib_is_deterministic() {
        assert_eq!(format_mib(0), "0.0 MiB");
        assert_eq!(format_mib(2 * 1024 * 1024), "2.0 MiB");
        assert_eq!(format_mib(2 * 1024 * 1024 + 512 * 1024), "2.5 MiB");
        assert_eq!(format_mib(1024), "0.0 MiB");
    }
}
