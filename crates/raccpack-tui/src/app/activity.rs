//! Activity layer — user-meaningful semantic event stream.
//!
//! Distinct from the `l` debug ring-buffer: activity records *what happened*
//! (scan complete, findings raised, an operation failed), never diagnostics.
//! This module owns the data model and the feeding policy (progress throttle);
//! rendering lives in `src/ui/widgets/activity.rs`.

use std::path::Path;

use raccpack_core::app::{OperationKind, ProgressEvent};

/// Maximum number of entries kept (newest first); older entries are trimmed
/// and counted for the `(n)` marker in the panel. Shared with the widget so
/// cap and display never drift.
pub const ACTIVITY_CAP: usize = 32;

/// Semantic kind of an activity entry. The glyph carries the meaning so the
/// stream survives NO_COLOR; colour only enhances it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    /// Success — operation completed cleanly.
    Ok,
    /// Warning — findings raised, degraded outcome, failure risk.
    Warn,
    /// Error — an operation failed.
    Error,
    /// Information — operation starts, progress steps, cache source.
    Info,
}

impl ActivityKind {
    /// The glyph that carries this entry's semantic meaning (NO_COLOR-safe).
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Ok => "✔",
            Self::Warn => "!",
            Self::Error => "✖",
            Self::Info => "·",
        }
    }
}

/// One activity row: semantic kind + human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub message: String,
}

/// Bounded, newest-first stream of [`ActivityEntry`]s.
///
/// Pushing past [`ACTIVITY_CAP`] trims the oldest entry and bumps `trimmed`,
/// which the panel renders as a `(n)` marker so history loss stays visible.
#[derive(Debug, Default)]
pub struct ActivityLog {
    entries: Vec<ActivityEntry>,
    trimmed: usize,
    /// Last progress line written (op, percent, message) — the state behind
    /// the ≥10-point / text-change throttle.
    last_progress: Option<(OperationKind, u8, String)>,
}

impl ActivityLog {
    /// Record one entry at the top of the stream (newest first).
    pub fn push(&mut self, kind: ActivityKind, message: impl Into<String>) {
        self.entries.insert(
            0,
            ActivityEntry {
                kind,
                message: message.into(),
            },
        );
        if self.entries.len() > ACTIVITY_CAP {
            self.entries.pop();
            self.trimmed += 1;
        }
    }

    /// Record a throttled progress entry (`info`): written only when `percent`
    /// stepped by ≥10 points **or** the status message changed.
    pub fn push_progress(&mut self, event: &ProgressEvent) {
        let keep_quiet =
            self.last_progress
                .as_ref()
                .is_some_and(|(last_op, last_percent, last_msg)| {
                    *last_op == event.operation
                        && u16::from(event.percent) < u16::from(*last_percent) + 10
                        && event.message == last_msg.as_str()
                });
        if keep_quiet {
            return;
        }
        self.last_progress = Some((event.operation, event.percent, event.message.clone()));
        self.push(
            ActivityKind::Info,
            format!("{} · {}%", op_label(event.operation), event.percent),
        );
    }

    /// Stored entries, newest first.
    pub fn entries(&self) -> &[ActivityEntry] {
        &self.entries
    }

    /// How many entries were trimmed beyond the cap (rendered as `(n)`).
    pub fn trimmed(&self) -> usize {
        self.trimmed
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Lowercase operation label for activity lines (`sniff · 40%`).
pub fn op_label(op: OperationKind) -> &'static str {
    match op {
        OperationKind::Sniff => "sniff",
        OperationKind::Dig => "dig",
        OperationKind::Stash => "stash",
        OperationKind::Rinse => "rinse",
        OperationKind::Pack => "pack",
        OperationKind::Raid => "raid",
    }
}

/// Short label for a project path in activity lines: the directory basename,
/// falling back to the full path when the basename is unavailable.
pub fn project_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_with(n: usize) -> ActivityLog {
        let mut log = ActivityLog::default();
        for i in 0..n {
            log.push(ActivityKind::Info, format!("entry {i}"));
        }
        log
    }

    fn progress(op: OperationKind, percent: u8, message: &str) -> ProgressEvent {
        ProgressEvent {
            operation: op,
            phase: "x".to_string(),
            phase_index: 0,
            phase_count: 1,
            percent,
            overall_percent: percent,
            message: message.to_string(),
            phase_complete: false,
        }
    }

    #[test]
    fn kinds_have_distinct_glyphs() {
        let glyphs: Vec<&str> = [
            ActivityKind::Ok,
            ActivityKind::Warn,
            ActivityKind::Error,
            ActivityKind::Info,
        ]
        .iter()
        .map(|k| k.glyph())
        .collect();
        assert_eq!(glyphs, vec!["✔", "!", "✖", "·"]);
    }

    #[test]
    fn pushes_newest_first() {
        let mut log = ActivityLog::default();
        log.push(ActivityKind::Ok, "first");
        log.push(ActivityKind::Warn, "second");
        assert_eq!(log.entries()[0].message, "second");
        assert_eq!(log.entries()[1].message, "first");
    }

    #[test]
    fn cap_trims_oldest_and_counts() {
        let log = log_with(ACTIVITY_CAP + 3);
        assert_eq!(log.entries().len(), ACTIVITY_CAP);
        assert_eq!(log.trimmed(), 3);
        // The three oldest (entry 0..2) are gone; the tail survives.
        assert_eq!(
            log.entries().last().map(|e| e.message.as_str()),
            Some("entry 3")
        );
    }

    #[test]
    fn empty_log_has_no_entries() {
        let log = ActivityLog::default();
        assert!(log.is_empty());
        assert_eq!(log.trimmed(), 0);
    }

    #[test]
    fn progress_throttles_fine_steps_and_same_text() {
        let mut log = ActivityLog::default();
        log.push_progress(&progress(OperationKind::Sniff, 0, "scan"));
        log.push_progress(&progress(OperationKind::Sniff, 5, "scan"));
        log.push_progress(&progress(OperationKind::Sniff, 7, "scan"));
        assert_eq!(log.entries().len(), 1, "no step < 10 and same text → quiet");
    }

    #[test]
    fn progress_writes_on_ten_point_step() {
        let mut log = ActivityLog::default();
        log.push_progress(&progress(OperationKind::Sniff, 0, "scan"));
        log.push_progress(&progress(OperationKind::Sniff, 10, "scan"));
        assert_eq!(log.entries().len(), 2);
        assert_eq!(log.entries()[0].message, "sniff · 10%");
    }

    #[test]
    fn progress_writes_on_text_change() {
        let mut log = ActivityLog::default();
        log.push_progress(&progress(OperationKind::Sniff, 0, "scan"));
        log.push_progress(&progress(OperationKind::Sniff, 4, "detect"));
        assert_eq!(log.entries().len(), 2);
        assert_eq!(log.entries()[0].message, "sniff · 4%");
    }

    #[test]
    fn progress_resets_between_operations() {
        let mut log = ActivityLog::default();
        log.push_progress(&progress(OperationKind::Sniff, 0, "scan"));
        log.push_progress(&progress(OperationKind::Dig, 0, "dig"));
        assert_eq!(log.entries().len(), 2, "op change must always be recorded");
        assert!(
            log.entries()[0].message.starts_with("dig ·"),
            "latest line must carry the new op"
        );
    }

    #[test]
    fn op_labels_cover_every_operation() {
        for op in [
            OperationKind::Sniff,
            OperationKind::Dig,
            OperationKind::Stash,
            OperationKind::Rinse,
            OperationKind::Pack,
            OperationKind::Raid,
        ] {
            assert!(!op_label(op).is_empty(), "{op:?} must have a label");
        }
    }

    #[test]
    fn project_label_prefers_basename() {
        assert_eq!(project_label(Path::new("/tmp/srv")), "srv");
        assert_eq!(project_label(Path::new("/just-a/path")), "path");
    }
}
