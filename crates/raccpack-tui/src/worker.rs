//! Worker thread for running core operations asynchronously.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use raccpack_core::app::{
    AppContext, ProgressEvent, ProgressSink, RunMode, SniffOptions, SniffResult,
};
use raccpack_core::config::RaccConfig;
use raccpack_core::detect::DetectMode;
use raccpack_core::domain::Error;

/// Events sent from the worker thread to the UI thread.
#[derive(Debug)]
pub enum WorkerEvent {
    /// Progress update from core operation.
    Progress(ProgressEvent),
    /// Sniff completed with result.
    SniffDone(Result<SniffResult, Error>),
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
                    let config = build_config(scan_root.clone(), den_dir);
                    let ctx = AppContext::from_config(config, RunMode::DryRun);
                    if let Err(e) = ctx {
                        let _ = event_tx.send(WorkerEvent::SniffDone(Err(e.into())));
                        continue;
                    }
                    let ctx = ctx.unwrap();
                    let opts = SniffOptions {
                        force_refresh,
                        max_depth,
                        detect_mode,
                    };
                    let mut sink = TuiProgressSink::new(event_tx.clone());
                    let result = raccpack_core::app::sniff(&ctx, &opts, &mut sink);
                    let _ = event_tx.send(WorkerEvent::SniffDone(result));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn worker_can_spawn_and_receive_cancel() {
        let (worker_tx, event_rx) = spawn_worker();

        worker_tx.send(WorkerMsg::Cancel).unwrap();

        // Should receive Cancelled event
        let event = event_rx.recv_timeout(Duration::from_millis(500)).unwrap();
        assert!(matches!(event, WorkerEvent::Cancelled));
    }

    #[test]
    fn worker_can_spawn_and_receive_sniff_done() {
        let (worker_tx, event_rx) = spawn_worker();
        let scan_root = std::path::PathBuf::from("/tmp/nonexistent");
        let den_dir = std::path::PathBuf::from("/tmp/den");

        // This will fail because scan_root doesn't exist, but we should get a SniffDone event with error
        worker_tx
            .send(WorkerMsg::Sniff {
                scan_root,
                den_dir,
                force_refresh: true,
                detect_mode: None,
                max_depth: None,
            })
            .unwrap();

        // Should receive SniffDone event (with error)
        let event = event_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(event, WorkerEvent::SniffDone(_)));
    }
}
