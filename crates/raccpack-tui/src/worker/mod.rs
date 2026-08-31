//! Worker thread for running core operations asynchronously.
//!
//! Submodules: [`self::raid`] hosts the raid preview / commit runs.

pub mod pack;
pub mod raid;

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use raccpack_core::app::{
    AppContext, DigOptions, DigResult, OperationKind, ProgressEvent, ProgressSink, RunMode,
    SniffOptions, SniffResult,
};
use raccpack_core::config::RaccConfig;
use raccpack_core::detect::DetectMode;
use raccpack_core::domain::Error;
use raccpack_core::secrets::FindingRef;
use zeroize::Zeroizing;

use self::pack::{run_pack_commit, run_pack_preview};
use self::raid::{run_raid_commit, run_raid_preview};

pub use self::pack::PackWorkerOpts;
pub use self::raid::{RaidWorkerOpts, WorkerPassphrase};

/// Placeholder identity used where the stash phase is skipped or the run is a
/// dry run; the passphrase itself is never used and never logged.
pub(crate) const DRY_RUN_PASSPHRASE: &str = "unused-dry-run-passphrase";

/// Events sent from the worker thread to the UI thread.
#[derive(Debug)]
pub enum WorkerEvent {
    /// Progress update from core operation.
    Progress(ProgressEvent),
    /// Sniff completed with result.
    SniffDone(Result<SniffResult, Error>),
    /// Dig completed with result (never carries raw secrets).
    DigDone(Result<DigResult, Error>),
    /// Raid preview completed (dry run, nothing written).
    RaidPreviewDone(Result<raccpack_core::app::RaidResult, Error>),
    /// Raid commit completed.
    RaidDone(Result<raccpack_core::app::RaidResult, Error>),
    /// Pack preview completed (dry run, nothing written).
    PackPreviewDone(Result<raccpack_core::app::PackResult, Error>),
    /// Pack commit completed.
    PackDone(Result<raccpack_core::app::PackResult, Error>),
    /// Ephemeral reveal finished; carries the raw value zeroized, shown once.
    RevealReady(WorkerRevealSecret),
    /// Ephemeral reveal failed; no raw value is ever sent on failure.
    RevealFailed(Error),
    /// Operation cancelled.
    Cancelled,
}

/// Messages sent to the worker thread.
#[derive(Debug)]
pub enum WorkerMsg {
    /// Run sniff with given options.
    Sniff {
        scan_root: PathBuf,
        den_dir: PathBuf,
        force_refresh: bool,
        detect_mode: Option<DetectMode>,
        max_depth: Option<usize>,
    },
    /// Run dig for a single project directory.
    Dig {
        project: PathBuf,
        den_dir: PathBuf,
        scan_content: bool,
    },
    /// Run a dry-run raid preview for a project (writes nothing to the den).
    RaidPreview {
        project: PathBuf,
        den_dir: PathBuf,
        opts: RaidWorkerOpts,
    },
    /// Run a raid commit for a project.
    RaidRun {
        project: PathBuf,
        den_dir: PathBuf,
        opts: RaidWorkerOpts,
        passphrase: WorkerPassphrase,
    },
    /// Run a dry-run pack preview for a project.
    PackPreview {
        project: PathBuf,
        den_dir: PathBuf,
        opts: PackWorkerOpts,
    },
    /// Run a pack commit for a project.
    PackRun {
        project: PathBuf,
        den_dir: PathBuf,
        opts: PackWorkerOpts,
    },
    /// Reveal one secret value (opt-in). `dir_root` is the project root used
    /// for path containment; the reference pinpoints the value to reveal.
    Reveal {
        path: PathBuf,
        dir_root: PathBuf,
        reference: FindingRef,
    },
    /// Cancel current operation.
    Cancel,
}

/// Spawn the worker thread that executes core operations.
///
/// Returns a sender to communicate with the worker and a receiver for worker events.
pub fn spawn_worker() -> (mpsc::Sender<WorkerMsg>, mpsc::Receiver<WorkerEvent>) {
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerMsg>();
    let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();

    thread::spawn(move || {
        while let Ok(msg) = worker_rx.recv() {
            match msg {
                WorkerMsg::Sniff {
                    scan_root,
                    den_dir,
                    force_refresh,
                    detect_mode,
                    max_depth,
                } => {
                    let config = build_config(scan_root, den_dir);
                    let opts = SniffOptions {
                        force_refresh,
                        max_depth,
                        detect_mode,
                    };
                    run_sniff(config, opts, event_tx.clone());
                }
                WorkerMsg::Dig {
                    project,
                    den_dir,
                    scan_content,
                } => {
                    // Dig always targets one project directory; that directory
                    // becomes the resolved scan root for the run.
                    let config = build_config(project, den_dir);
                    let opts = DigOptions {
                        project: None,
                        find_repeated: false,
                        scan_content,
                        use_heuristics: None,
                    };
                    run_dig(config, opts, event_tx.clone());
                }
                WorkerMsg::RaidPreview {
                    project,
                    den_dir,
                    opts,
                } => {
                    run_raid_preview(project, den_dir, opts, event_tx.clone());
                }
                WorkerMsg::RaidRun {
                    project,
                    den_dir,
                    opts,
                    passphrase,
                } => {
                    run_raid_commit(project, den_dir, opts, passphrase, event_tx.clone());
                }
                WorkerMsg::PackPreview {
                    project,
                    den_dir,
                    opts,
                } => {
                    run_pack_preview(project, den_dir, opts, event_tx.clone());
                }
                WorkerMsg::PackRun {
                    project,
                    den_dir,
                    opts,
                } => {
                    run_pack_commit(project, den_dir, opts, event_tx.clone());
                }
                WorkerMsg::Reveal {
                    path,
                    dir_root,
                    reference,
                } => {
                    run_reveal(path, dir_root, reference, event_tx.clone());
                }
                WorkerMsg::Cancel => {
                    let _ = event_tx.send(WorkerEvent::Cancelled);
                }
            }
        }
    });

    (worker_tx, event_rx)
}

/// Build a minimal config for the given paths.
fn build_config(scan_root: PathBuf, den_dir: PathBuf) -> RaccConfig {
    let mut config = RaccConfig::default();
    config.paths.scan_root = Some(scan_root.to_string_lossy().to_string());
    config.paths.den_dir = Some(den_dir.to_string_lossy().to_string());
    config
}

fn run_sniff(config: RaccConfig, opts: SniffOptions, event_tx: mpsc::Sender<WorkerEvent>) {
    let ctx = match AppContext::from_config(config, RunMode::DryRun) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::SniffDone(Err(e.into())));
            return;
        }
    };
    let mut sink = TuiProgressSink::new(event_tx.clone());
    let result = raccpack_core::app::sniff(&ctx, &opts, &mut sink);
    let _ = event_tx.send(WorkerEvent::SniffDone(result));
}

fn run_dig(config: RaccConfig, opts: DigOptions, event_tx: mpsc::Sender<WorkerEvent>) {
    let ctx = match AppContext::from_config(config, RunMode::DryRun) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::DigDone(Err(e.into())));
            return;
        }
    };
    let mut sink = TuiProgressSink::new(event_tx.clone());
    let result = raccpack_core::app::dig(&ctx, &opts, &mut sink);
    let _ = event_tx.send(WorkerEvent::DigDone(result));
}

/// The raw revealed secret, zeroized on drop and `Debug`-redacted.
///
/// This is the only payload that may carry a raw secret value from the worker
/// to the UI, and it does so transiently: it lives only in the reveal modal's
/// `Ready` phase and is dropped (zeroized) the moment the modal closes.
pub struct WorkerRevealSecret(Zeroizing<String>);

impl WorkerRevealSecret {
    /// Wrap a revealed value into a zeroized, redacted payload.
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Borrow the raw value for a single modal render.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for WorkerRevealSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkerRevealSecret(**)")
    }
}

/// Run one opt-in reveal in the worker thread.
///
/// The core re-reads the file fresh and returns an [`EphemeralSecret`] only on
/// an exact hash match; the value is copied into a zeroized payload and sent to
/// the UI. On failure, only the error is forwarded — never a value.
fn run_reveal(
    path: PathBuf,
    dir_root: PathBuf,
    reference: FindingRef,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    match raccpack_core::secrets::reveal_finding(&path, &dir_root, &reference) {
        Ok(secret) => {
            let payload = WorkerRevealSecret::new(secret.expose().to_string());
            let _ = event_tx.send(WorkerEvent::RevealReady(payload));
        }
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::RevealFailed(e));
        }
    }
}

/// Progress sink that forwards events to the UI via a channel.
pub struct TuiProgressSink {
    tx: mpsc::Sender<WorkerEvent>,
}

impl TuiProgressSink {
    pub fn new(tx: mpsc::Sender<WorkerEvent>) -> Self {
        Self { tx }
    }
}

impl ProgressSink for TuiProgressSink {
    fn emit(&mut self, event: ProgressEvent) {
        let _ = self.tx.send(WorkerEvent::Progress(event));
    }
}

/// Progress sink that forwards only `OperationKind::Raid` events to the UI,
/// wrapping the shared [`TuiProgressSink`]. Sub-facades (stash/rinse/pack)
/// emit their own operation kinds; the raid screen listens to raid events only.
pub struct RaidProgressSink {
    inner: TuiProgressSink,
}

impl RaidProgressSink {
    pub fn new(inner: TuiProgressSink) -> Self {
        Self { inner }
    }
}

impl ProgressSink for RaidProgressSink {
    fn emit(&mut self, event: ProgressEvent) {
        if event.operation == OperationKind::Raid {
            self.inner.emit(event);
        }
    }
}

/// Progress sink that forwards only `OperationKind::Pack` events to the UI,
/// wrapping the shared [`TuiProgressSink`].
pub struct PackProgressSink {
    inner: TuiProgressSink,
}

impl PackProgressSink {
    pub fn new(inner: TuiProgressSink) -> Self {
        Self { inner }
    }
}

impl ProgressSink for PackProgressSink {
    fn emit(&mut self, event: ProgressEvent) {
        if event.operation == OperationKind::Pack {
            self.inner.emit(event);
        }
    }
}

#[cfg(test)]
mod tests;
