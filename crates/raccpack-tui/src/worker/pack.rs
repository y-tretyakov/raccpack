use std::path::PathBuf;
use std::sync::mpsc;

use raccpack_core::app::{AppContext, PackOptions, RunMode};

use crate::app::pack::PackFlowOptions;

use super::{build_config, PackProgressSink, TuiProgressSink, WorkerEvent};

/// Pack worker options; mirrors [`PackFlowOptions`] without the flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWorkerOpts {
    pub deny_content_secrets: bool,
    pub zstd_level: u32,
    pub output_name: Option<String>,
}

impl From<PackFlowOptions> for PackWorkerOpts {
    fn from(opts: PackFlowOptions) -> Self {
        Self {
            deny_content_secrets: opts.deny_content_secrets,
            zstd_level: opts.zstd_level,
            output_name: opts.output_name,
        }
    }
}

pub(super) fn run_pack_preview(
    project: PathBuf,
    den_dir: PathBuf,
    opts: PackWorkerOpts,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    let config = build_config(project.clone(), den_dir);
    let ctx = match AppContext::from_config(config, RunMode::DryRun) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::PackPreviewDone(Err(e.into())));
            return;
        }
    };
    let pack_opts = PackOptions {
        project,
        deny_content_secrets: opts.deny_content_secrets,
        zstd_level: Some(opts.zstd_level),
        output_name: opts.output_name,
        ..PackOptions::default()
    };
    let mut sink = PackProgressSink::new(TuiProgressSink::new(event_tx.clone()));
    let result = raccpack_core::app::pack(&ctx, &pack_opts, &mut sink);
    let _ = event_tx.send(WorkerEvent::PackPreviewDone(result));
}

pub(super) fn run_pack_commit(
    project: PathBuf,
    den_dir: PathBuf,
    opts: PackWorkerOpts,
    event_tx: mpsc::Sender<WorkerEvent>,
) {
    let config = build_config(project.clone(), den_dir);
    let ctx = match AppContext::from_config(config, RunMode::Commit) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::PackDone(Err(e.into())));
            return;
        }
    };
    let pack_opts = PackOptions {
        project,
        deny_content_secrets: opts.deny_content_secrets,
        zstd_level: Some(opts.zstd_level),
        output_name: opts.output_name,
        ..PackOptions::default()
    };
    let mut sink = PackProgressSink::new(TuiProgressSink::new(event_tx.clone()));
    let result = raccpack_core::app::pack(&ctx, &pack_opts, &mut sink);
    let _ = event_tx.send(WorkerEvent::PackDone(result));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_opts_flow_conversion_carries_output_name() {
        let mut flow_opts = PackFlowOptions::default();
        assert_eq!(PackWorkerOpts::from(flow_opts.clone()).output_name, None);

        flow_opts.set_output_name(Some("custom".to_string()));
        let worker = PackWorkerOpts::from(flow_opts);
        assert_eq!(worker.output_name.as_deref(), Some("custom"));
        assert!(worker.deny_content_secrets);
        assert_eq!(worker.zstd_level, 3);
    }
}
