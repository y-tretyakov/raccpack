//! Activity feed assembly — turns command/worker outcomes into activity lines.
//!
//! Owns *what happened* (glyph/kind + human message) for each operation, so the
//! event loop in `crate::event` stays a thin event→call dispatcher. Every
//! function here owns exactly one activity line; nothing here mutates screen
//! state or talks to the worker.

use std::path::Path;

use crate::app::activity::{project_label, ActivityKind, ActivityLog};
use crate::app::dig::DigScreenState;
use crate::ui::widgets::format_bytes;

/// Label of the dig-scoped project, or `-` when no project is in scope.
pub fn dig_project_label(dig_state: &DigScreenState) -> String {
    dig_state
        .project
        .as_deref()
        .map(project_label)
        .unwrap_or_else(|| "-".to_string())
}

/// Sniff refresh/start is dispatched to the worker.
pub fn push_sniff_started(log: &mut ActivityLog) {
    log.push(ActivityKind::Info, "sniff started");
}

/// A dig run is dispatched for `project`.
pub fn push_dig_started(log: &mut ActivityLog, project: &Path) {
    log.push(
        ActivityKind::Info,
        format!("dig {} started", project_label(project)),
    );
}

/// Sniff completed with `projects` found, `total_size_bytes` scanned, from cache.
pub fn push_scan_complete(
    log: &mut ActivityLog,
    projects: usize,
    total_size_bytes: u64,
    from_cache: bool,
) {
    log.push(
        ActivityKind::Ok,
        format!(
            "Scan complete · {projects} projects · {}{}",
            format_bytes(total_size_bytes),
            if from_cache { " (cache)" } else { "" }
        ),
    );
}

/// Sniff operation failed.
pub fn push_scan_failed(log: &mut ActivityLog) {
    log.push(ActivityKind::Error, "Scan failed");
}

/// Dig completed for `dig_state`'s project. Findings raise a warning.
pub fn push_dig_complete(log: &mut ActivityLog, dig_state: &DigScreenState, findings: usize) {
    let kind = if findings > 0 {
        ActivityKind::Warn
    } else {
        ActivityKind::Ok
    };
    let project = dig_project_label(dig_state);
    log.push(kind, format!("dig {project} · {findings} findings"));
}

/// Dig failed for `dig_state`'s project.
pub fn push_dig_failed(log: &mut ActivityLog, dig_state: &DigScreenState) {
    let project = dig_project_label(dig_state);
    log.push(ActivityKind::Error, format!("dig {project} failed"));
}

/// Raid run finished for `project`; `ok` selects the entry kind + wording.
pub fn push_raid_done(log: &mut ActivityLog, project: &Path, ok: bool) {
    let project = project_label(project);
    if ok {
        log.push(ActivityKind::Ok, format!("raid {project} · completed"));
    } else {
        log.push(ActivityKind::Error, format!("raid {project} · failed"));
    }
}
