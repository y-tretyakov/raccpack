//! Raid-level progress event planning and construction.
//!
//! A raid run reports exactly one completion event per **planned** phase
//! (enabled stash/rinse/pack in fixed order, then the implicit `"move"`), plus
//! one additional `"rollback"` completion event only when a commit failure
//! triggered a rollback. Disabled phases produce no event of their own, so
//! [`plan_phases`] yields indices that renumber around them.
//! [`overall_percent`] follows the spec formula
//! `(phase_index * 100 + percent) / phase_count`, clamped to 0..=100.

use crate::app::progress::{OperationKind, ProgressEvent, ProgressSink};

use super::RaidOptions;

/// Ordered phase names of a raid run: enabled stash/rinse/pack followed by
/// the implicit `"move"` phase.
pub(super) fn plan_phases(opts: &RaidOptions) -> Vec<&'static str> {
    let mut planned = Vec::with_capacity(4);
    if opts.stash.enabled {
        planned.push("stash");
    }
    if opts.rinse.enabled {
        planned.push("rinse");
    }
    if opts.pack.enabled {
        planned.push("pack");
    }
    planned.push("move");
    planned
}

/// Overall raid progress for a phase:
/// `(phase_index * 100 + percent) / phase_count`, clamped to 0..=100.
pub(super) fn overall_percent(phase_index: u32, phase_count: u32, percent: u8) -> u8 {
    if phase_count == 0 {
        return 0;
    }
    let numerator = phase_index
        .saturating_mul(100)
        .saturating_add(u32::from(percent));
    (numerator / phase_count).min(100) as u8
}

/// Emit the raid completion event for one planned phase.
///
/// `phase` must be present in `planned` (only enabled phases are planned);
/// an unknown phase emits nothing.
pub(super) fn emit_phase_event(
    progress: &mut dyn ProgressSink,
    planned: &[&'static str],
    phase: &str,
    phase_count: u32,
    message: impl Into<String>,
) {
    if let Some(index) = planned.iter().position(|p| *p == phase) {
        progress.emit(raid_event(phase, index as u32, phase_count, message));
    }
}

/// Emit a raid completion event for the implicit "rollback" phase (PR3).
///
/// Not part of [`plan_phases`] — an extra completion event emitted only when a
/// commit failure triggered a rollback. `phase_index == phase_count`, so the
/// spec formula clamps the overall percent to 100.
pub(super) fn emit_rollback_event(
    progress: &mut dyn ProgressSink,
    phase_count: u32,
    message: impl Into<String>,
) {
    progress.emit(raid_event("rollback", phase_count, phase_count, message));
}

/// Build a raid-level completion event for a planned phase.
fn raid_event(
    phase: &str,
    phase_index: u32,
    phase_count: u32,
    message: impl Into<String>,
) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Raid,
        phase: phase.to_string(),
        phase_index,
        phase_count,
        percent: 100,
        overall_percent: overall_percent(phase_index, phase_count, 100),
        message: message.into(),
        phase_complete: true,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::raid::{OrchestrationMode, PackPhaseOpts, RinsePhaseOpts, StashPhaseOpts};
    use crate::domain::SensitiveRisk;

    use super::*;

    fn options(stash: bool, rinse: bool, pack: bool) -> RaidOptions {
        RaidOptions {
            project: PathBuf::from("/tmp/p"),
            mode: OrchestrationMode::Atomic,
            stash: StashPhaseOpts {
                enabled: stash,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: rinse },
            pack: PackPhaseOpts {
                enabled: pack,
                deny_content_secrets: true,
            },
        }
    }

    struct TestSink(Vec<ProgressEvent>);

    impl ProgressSink for TestSink {
        fn emit(&mut self, event: ProgressEvent) {
            self.0.push(event);
        }
    }

    #[test]
    fn plan_phases_all_enabled_includes_move_last() {
        assert_eq!(
            plan_phases(&options(true, true, true)),
            vec!["stash", "rinse", "pack", "move"]
        );
    }

    #[test]
    fn plan_phases_disabled_phases_shift_indices() {
        assert_eq!(
            plan_phases(&options(false, true, true)),
            vec!["rinse", "pack", "move"]
        );
        assert_eq!(
            plan_phases(&options(true, false, true)),
            vec!["stash", "pack", "move"]
        );
        assert_eq!(
            plan_phases(&options(false, false, true)),
            vec!["pack", "move"]
        );
        assert_eq!(plan_phases(&options(false, false, false)), vec!["move"]);
    }

    #[test]
    fn overall_percent_follows_spec_formula() {
        assert_eq!(overall_percent(0, 4, 100), 25);
        assert_eq!(overall_percent(1, 4, 50), 37);
        assert_eq!(overall_percent(3, 4, 100), 100);
        assert_eq!(overall_percent(0, 3, 100), 33);
        assert_eq!(overall_percent(2, 3, 100), 100);
    }

    #[test]
    fn overall_percent_single_phase_is_phase_percent() {
        assert_eq!(overall_percent(0, 1, 0), 0);
        assert_eq!(overall_percent(0, 1, 100), 100);
    }

    #[test]
    fn overall_percent_clamps_to_100_and_zero_count_is_0() {
        assert_eq!(overall_percent(5, 4, 100), 100);
        assert_eq!(overall_percent(0, 0, 100), 0);
    }

    #[test]
    fn raid_event_helper_shape() {
        let event = raid_event("stash", 0, 4, "stashed 3 files");
        assert_eq!(event.operation, OperationKind::Raid);
        assert_eq!(event.phase, "stash");
        assert_eq!(event.phase_index, 0);
        assert_eq!(event.phase_count, 4);
        assert_eq!(event.percent, 100);
        assert!(event.phase_complete);
        assert_eq!(event.overall_percent, 25);
        assert_eq!(event.message, "stashed 3 files");

        let done = raid_event("move", 3, 4, "finalized staged artifacts");
        assert_eq!(done.phase_index, 3);
        assert!(done.phase_complete);
        assert_eq!(done.overall_percent, 100);
    }

    #[test]
    fn emit_phase_event_uses_planned_index_and_skips_unknown() {
        let planned = ["rinse", "pack", "move"];
        let mut sink = TestSink(Vec::new());
        emit_phase_event(&mut sink, &planned, "pack", 3, "packed 2 files");
        emit_phase_event(&mut sink, &planned, "stash", 3, "never emitted");

        assert_eq!(sink.0.len(), 1);
        let event = &sink.0[0];
        assert_eq!(event.phase, "pack");
        assert_eq!(event.phase_index, 1);
        assert_eq!(event.phase_count, 3);
        assert_eq!(event.overall_percent, 66);
    }
}
