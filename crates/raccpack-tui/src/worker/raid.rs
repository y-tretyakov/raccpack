//! Raid worker operations: dry-run preview and commit with a passphrase.

use std::path::PathBuf;
use std::sync::mpsc;

use raccpack_core::app::{
    AgeIdentity, AppContext, OrchestrationMode, PackPhaseOpts, RaidOptions, RinsePhaseOpts,
    RunMode, StashPhaseOpts,
};
use raccpack_core::domain::SensitiveRisk;
use zeroize::Zeroizing;

use crate::app::raid::RaidFlowOptions;

use super::{build_config, TuiProgressSink, WorkerEvent, DRY_RUN_PASSPHRASE};

/// Worker-side raid options; mirrors [`RaidFlowOptions`] without the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidWorkerOpts {
    /// Keep source files after a Commit (`remove_sources=false`).
    pub keep_sources: bool,
    /// Skip the stash phase entirely.
    pub skip_stash: bool,
    /// Orchestration mode: Atomic (default) or FailFast.
    pub mode: OrchestrationMode,
}

impl From<RaidFlowOptions> for RaidWorkerOpts {
    fn from(opts: RaidFlowOptions) -> Self {
        Self {
            keep_sources: opts.keep_sources,
            skip_stash: opts.skip_stash,
            mode: opts.mode,
        }
    }
}

/// Passphrase carried from the UI to the worker. The inner material is
/// zeroized on drop and `Debug` prints only `[redacted]`.
pub struct WorkerPassphrase(Zeroizing<String>);

impl WorkerPassphrase {
    /// Wrap a passphrase into a zeroized, redacted value.
    pub fn new(passphrase: String) -> Self {
        Self(Zeroizing::new(passphrase))
    }

    /// Move an already-zeroized passphrase out of the flow without copying.
    pub fn from_zeroizing(passphrase: Zeroizing<String>) -> Self {
        Self(passphrase)
    }
}

impl std::fmt::Debug for WorkerPassphrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WorkerPassphrase")
            .field(&"[redacted]")
            .finish()
    }
}

/// Build raid options for the worker from the flow toggles. Stash phase uses
/// High min-risk (as in the CLI), mirrors the flow's keep/skip switches.
fn raid_options_for(project: PathBuf, opts: RaidWorkerOpts) -> RaidOptions {
    RaidOptions {
        project,
        mode: opts.mode,
        stash: StashPhaseOpts {
            enabled: !opts.skip_stash,
            min_risk: SensitiveRisk::High,
            remove_sources: !opts.keep_sources,
        },
        rinse: RinsePhaseOpts { enabled: true },
        pack: PackPhaseOpts {
            enabled: true,
            deny_content_secrets: true,
        },
    }
}

/// Raid preview: `RunMode::DryRun`, placeholder identity. Nothing is written
/// to the den; the identity is required by the stash phase but never used.
pub(super) fn run_raid_preview(
    project: PathBuf,
    den_dir: PathBuf,
    opts: RaidWorkerOpts,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    let config = build_config(project.clone(), den_dir);
    let ctx = match AppContext::from_config(config, RunMode::DryRun) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::RaidPreviewDone(Err(e.into())));
            return;
        }
    };
    let raid_opts = raid_options_for(project, opts);
    let identity = AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE.to_string()));
    let mut sink = super::RaidProgressSink::new(TuiProgressSink::new(event_tx.clone()));
    let result = raccpack_core::app::raid(&ctx, &raid_opts, Some(&identity), &mut sink);
    let _ = event_tx.send(WorkerEvent::RaidPreviewDone(result));
}

/// Raid commit: `RunMode::Commit`; the passphrase identity moves into the run.
pub(super) fn run_raid_commit(
    project: PathBuf,
    den_dir: PathBuf,
    opts: RaidWorkerOpts,
    passphrase: WorkerPassphrase,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    let config = build_config(project.clone(), den_dir);
    let ctx = match AppContext::from_config(config, RunMode::Commit) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::RaidDone(Err(e.into())));
            return;
        }
    };
    let raid_opts = raid_options_for(project, opts);
    let identity = AgeIdentity::Passphrase(passphrase.0);
    let mut sink = super::RaidProgressSink::new(TuiProgressSink::new(event_tx.clone()));
    let result = raccpack_core::app::raid(&ctx, &raid_opts, Some(&identity), &mut sink);
    let _ = event_tx.send(WorkerEvent::RaidDone(result));
}
