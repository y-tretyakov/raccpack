//! Plain-text rendering of raid progress events for the CLI.
//!
//! [`CliProgress`] prints one `→ {phase}: {message}` line per raid phase
//! completion. Nested stash/rinse/pack facade events and in-flight (non
//! completion) updates are filtered out, so the human output stays a single
//! summary line per planned phase.

use raccpack_core::{OperationKind, ProgressEvent, ProgressSink};

/// A [`ProgressSink`] that prints raid phase-completion lines to stdout.
///
/// Holds no state, so it is trivially `Send`.
#[derive(Debug, Default)]
pub struct CliProgress;

impl ProgressSink for CliProgress {
    fn emit(&mut self, event: ProgressEvent) {
        if let Some(line) = render_event(&event) {
            print!("{line}");
        }
    }
}

/// Render a progress event as a `→ {phase}: {message}` line, or `None` when
/// the event is not a raid phase completion.
///
/// Only `operation == OperationKind::Raid && phase_complete` renders; any
/// other event (nested stash/rinse/pack, in-flight updates) returns `None`.
pub fn render_event(event: &ProgressEvent) -> Option<String> {
    if event.operation == OperationKind::Raid && event.phase_complete {
        Some(format!("→ {}: {}\n", event.phase, event.message))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_FIXTURE: &str = "AKIAEXAMPLE1234";

    fn raid_event(phase: &str, message: impl Into<String>) -> ProgressEvent {
        ProgressEvent {
            operation: OperationKind::Raid,
            phase: phase.to_string(),
            phase_index: 0,
            phase_count: 4,
            percent: 100,
            overall_percent: 25,
            message: message.into(),
            phase_complete: true,
        }
    }

    #[test]
    fn render_raid_completion_event() {
        let line =
            render_event(&raid_event("stash", "stashed 3 files")).expect("raid completion renders");
        assert_eq!(line, "→ stash: stashed 3 files\n");
    }

    #[test]
    fn render_ignores_non_complete_raid_event() {
        let mut event = raid_event("stash", "Encrypting…");
        event.phase_complete = false;
        assert!(render_event(&event).is_none());
    }

    #[test]
    fn render_ignores_non_raid_operation() {
        let mut event = raid_event("stash", "stashed 3 files");
        event.operation = OperationKind::Stash;
        assert!(render_event(&event).is_none());
    }

    #[test]
    fn render_line_never_contains_raw_fixture() {
        let line =
            render_event(&raid_event("stash", "stashed 3 files")).expect("raid completion renders");
        assert!(
            !line.contains(RAW_FIXTURE),
            "rendered line must never carry raw secret material"
        );
    }

    #[test]
    fn render_never_panics_on_any_event_shape() {
        for event in [
            raid_event("", ""),
            raid_event("stash", "смайлики 🙂 и unicode"),
            raid_event("move", "not run due to prior failure"),
        ] {
            let _ = render_event(&event);
        }
    }
}
