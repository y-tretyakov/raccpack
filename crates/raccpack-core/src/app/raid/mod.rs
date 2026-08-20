//! Facade use-case `raid`: orchestrate the project lifecycle.
//!
//! [`raid`] runs the enabled phases in fixed order — **stash → rinse → pack →
//! move** — under the orchestration mode selected by [`RaidOptions::mode`]:
//!
//! - [`OrchestrationMode::Atomic`] (default): the staging + deferred-destructive
//!   runner. Every intermediate artifact is written under one shared
//!   `den/staging/{raid_id}/` and moved into the den (stash `secrets/`, pack
//!   `packs/`) only in the commit phase; `remove_sources` and rinse deletes
//!   also run in the commit. WAL + rollback land in PR3, so
//!   [`RaidResult::rolled_back`] is always `false` and
//!   [`RaidResult::rollback_warnings`] always empty on this PR.
//! - [`OrchestrationMode::FailFast`] (legacy A3.1): stops at the first failing
//!   enabled phase; artifacts already placed stay in the den.
//!
//! Each phase delegates to its own facade ([`crate::app::stash`],
//! [`crate::app::rinse`], [`crate::app::pack`]); this module only orchestrates
//! and collects outcomes into [`RaidStageResult`]s.
//!
//! INVARIANTS:
//!
//! - **Fail-fast**: an `Err` from an enabled phase aborts the run; following
//!   enabled phases are `skipped` ("not run due to prior failure").
//!   [`crate::domain::Error::StashEmpty`] is a no-op ("nothing to stash") and
//!   the run continues.
//! - **Phase failure → `Ok(RaidResult { success: false, .. })`**, not `Err`:
//!   only **precondition** errors return `Err` (missing project path,
//!   missing/unsupported identity when stash is enabled).
//! - **DryRun safe**: sub-calls run under `ctx.mode`; nothing is created under
//!   the den and `remove_sources` never deletes.
//! - **Identity**: required only when `opts.stash.enabled`; otherwise ignored.
//! - **Progress events**: exactly one `OperationKind::Raid` completion event
//!   per planned phase (enabled stash/rinse/pack plus implicit `"move"`), with
//!   coherent indices/percent via the spec formula. Disabled phases emit
//!   nothing; fail-fast phases emit "not run due to prior failure". There is
//!   no start event — completion events are the only raid emissions.
//! - **Raw-free**: stage and event messages carry summaries only (counts,
//!   "disabled", "not run…", error `Display`), never raw secret material.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::{Result, SensitiveRisk};

use super::context::AppContext;
use super::pack::PackResult;
use super::progress::ProgressSink;
use super::rinse::RinseResult;
use super::stash::{AgeIdentity, StashResult};

mod atomic;
mod fail_fast;
mod progress;
mod stages;
mod staging;

use atomic::atomic_raid;
use fail_fast::fail_fast_raid;

/// Orchestration mode of a [`raid`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationMode {
    /// Staging + deferred destructive ops (default): every intermediate
    /// artifact lives under `den/staging/{raid_id}/` and is finalized in the
    /// commit phase; `remove_sources` and rinse deletes also happen in the
    /// commit. WAL + rollback land in PR3, so `rolled_back` is always `false`
    /// on this PR.
    Atomic,
    /// Legacy A3.1 behavior: stop at the first failing enabled phase, keep
    /// artifacts already placed in the den, no rollback.
    FailFast,
}

/// Message used for every phase short-circuited by an earlier failure.
const SKIPPED_MESSAGE: &str = "not run due to prior failure";

/// Phase-level options for the stash part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashPhaseOpts {
    /// Whether the stash phase runs.
    pub enabled: bool,
    /// Minimum risk to include (default [`SensitiveRisk::High`]).
    pub min_risk: SensitiveRisk,
    /// Delete source files after a successful Commit.
    pub remove_sources: bool,
}

/// Phase-level options for the rinse part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RinsePhaseOpts {
    /// Whether the rinse phase runs.
    pub enabled: bool,
}

/// Phase-level options for the pack part of a raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPhaseOpts {
    /// Whether the pack phase runs.
    pub enabled: bool,
    /// Skip files whose content is a Critical secret (default true).
    pub deny_content_secrets: bool,
}

/// Options controlling a full [`raid`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidOptions {
    /// Project to process; all phases operate on this tree.
    pub project: PathBuf,
    /// Orchestration mode (default [`OrchestrationMode::Atomic`]).
    pub mode: OrchestrationMode,
    /// Stash phase options.
    pub stash: StashPhaseOpts,
    /// Rinse phase options.
    pub rinse: RinsePhaseOpts,
    /// Pack phase options.
    pub pack: PackPhaseOpts,
}

impl Default for RaidOptions {
    fn default() -> Self {
        Self {
            project: PathBuf::new(), // caller must set
            mode: OrchestrationMode::Atomic,
            stash: StashPhaseOpts {
                enabled: true,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        }
    }
}

/// Outcome of a single logical phase within a [`raid`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidStageResult {
    /// Phase name: `"stash"` | `"rinse"` | `"pack"` | `"move"`.
    pub name: String,
    /// Whether the phase succeeded. `true` for ok and disabled (no-op);
    /// `false` for failed and skipped.
    pub success: bool,
    /// Human-readable summary, never containing secret material.
    pub message: String,
    /// Whether the phase did not run (disabled or short-circuited).
    pub skipped: bool,
}

/// Full outcome of a [`raid`] orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidResult {
    /// The project path the run operated on.
    pub project_path: PathBuf,
    /// Stage outcomes in run order (successful / failed / skipped).
    pub stages: Vec<RaidStageResult>,
    /// Stash sub-result when the stash phase ran.
    pub stash: Option<StashResult>,
    /// Rinse sub-result when the rinse phase ran.
    pub rinse: Option<RinseResult>,
    /// Pack sub-result when the pack phase ran.
    pub pack: Option<PackResult>,
    /// Paths of artifacts placed into the den (empty in a dry run).
    pub den_artifacts: Vec<PathBuf>,
    /// `true` iff every enabled phase succeeded.
    pub success: bool,
    /// Whether the run was a dry run (nothing written).
    pub dry_run: bool,
    /// `true` iff a failed run was rolled back to the pre-raid state.
    /// Always `false` on this PR — rollback logic lands in PR3.
    #[serde(default)]
    pub rolled_back: bool,
    /// Non-fatal warnings collected while rolling back.
    /// Always empty on this PR — rollback logic lands in PR3.
    #[serde(default)]
    pub rollback_warnings: Vec<String>,
}

/// Orchestrate `stash → rinse → pack → move` for `opts.project`.
///
/// Dispatches to the runner selected by [`RaidOptions::mode`]:
/// [`OrchestrationMode::Atomic`] (default; shared staging + deferred
/// destructive ops, rollback in PR3) or [`OrchestrationMode::FailFast`]
/// (legacy A3.1 semantics).
///
/// # Errors (preconditions only)
///
/// - Empty project path → [`crate::domain::Error::Other`].
/// - `opts.stash.enabled` without an identity → [`crate::domain::Error::Other`].
/// - `opts.stash.enabled` with [`AgeIdentity::Recipients`] →
///   [`crate::domain::Error::Unsupported`].
pub fn raid(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidResult> {
    match opts.mode {
        OrchestrationMode::Atomic => atomic_raid(ctx, opts, identity, progress),
        OrchestrationMode::FailFast => fail_fast_raid(ctx, opts, identity, progress),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn raid_default_options_have_all_phases_enabled() {
        let opts = RaidOptions::default();
        assert!(opts.stash.enabled);
        assert!(opts.rinse.enabled);
        assert!(opts.pack.enabled);
        assert_eq!(opts.stash.min_risk, SensitiveRisk::High);
        assert!(opts.stash.remove_sources);
        assert!(opts.pack.deny_content_secrets);
        assert!(opts.project.as_os_str().is_empty());
    }

    #[test]
    fn raid_default_mode_is_atomic() {
        assert_eq!(RaidOptions::default().mode, OrchestrationMode::Atomic);
    }

    #[test]
    fn enabled_phase_count_ignores_disabled() {
        let all = RaidOptions::default();
        assert_eq!(fail_fast::enabled_phase_count(&all), 3);

        let pack_only = RaidOptions {
            project: PathBuf::from("/tmp/p"),
            mode: OrchestrationMode::Atomic,
            stash: StashPhaseOpts {
                enabled: false,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: false },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        };
        assert_eq!(fail_fast::enabled_phase_count(&pack_only), 1);
    }

    #[test]
    fn raid_dispatch_atomic_delegates_to_fail_fast_on_dry_run() {
        use std::fs;

        use tempfile::TempDir;
        use zeroize::Zeroizing;

        use crate::app::context::{AppContext, RunMode};
        use crate::app::stash::AgeIdentity;
        use crate::app::NullProgress;
        use crate::config::RaccConfig;

        let temp = TempDir::new().expect("temp dir");
        let proj = temp.path().join("proj");
        let den = temp.path().join("den");
        fs::create_dir_all(&proj).expect("create project dir");

        let config = RaccConfig::default()
            .with_scan_root(&proj)
            .with_den_dir(&den);
        let ctx =
            AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config");
        let identity = AgeIdentity::Passphrase(Zeroizing::new("smoke".to_string()));

        let atomic_opts = RaidOptions {
            project: proj.clone(),
            mode: OrchestrationMode::Atomic,
            ..RaidOptions::default()
        };
        let fail_fast_opts = RaidOptions {
            project: proj.clone(),
            mode: OrchestrationMode::FailFast,
            ..RaidOptions::default()
        };

        let mut progress = NullProgress;
        let atomic = raid(&ctx, &atomic_opts, Some(&identity), &mut progress)
            .expect("atomic raid should succeed");
        let fail_fast = raid(&ctx, &fail_fast_opts, Some(&identity), &mut progress)
            .expect("fail-fast raid should succeed");

        assert_eq!(
            atomic, fail_fast,
            "PR2 DryRun: atomic must delegate to the fail-fast runner"
        );
        assert!(atomic.success);
        assert!(!atomic.rolled_back);
        assert!(atomic.rollback_warnings.is_empty());
    }

    #[test]
    fn raid_result_serializes_rollback_fields_with_defaults() {
        let result = RaidResult {
            project_path: PathBuf::from("/tmp/p"),
            stages: Vec::new(),
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: Vec::new(),
            success: true,
            dry_run: true,
            rolled_back: false,
            rollback_warnings: Vec::new(),
        };
        let value = serde_json::to_value(&result).expect("serialize RaidResult");
        assert_eq!(value["rolled_back"], false);
        assert_eq!(value["rollback_warnings"], serde_json::json!([]));
    }
}
