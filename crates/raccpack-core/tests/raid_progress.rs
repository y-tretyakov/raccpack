//! Integration tests for A3.2 — raid-level progress events.
//!
//! Spec: `docs/alpha/a3/a3.2-progress.md` (§3 contract, §3.1 phase planning,
//! §3.2 overall percent, §5 tests).
//!
//! Covers the raid progress contract end-to-end through the public `raid`
//! facade:
//! 1. All enabled: exactly one `OperationKind::Raid` completion event per
//!    planned phase in order `["stash", "rinse", "pack", "move"]`, with
//!    `phase_count == 4`, indices `0..3` and `overall_percent` 25/50/75/100;
//!    the last event is `move`, `overall_percent == 100`, `phase_complete`.
//! 2. Disabled stash: `phase_count == 3`, no event carries phase `"stash"`,
//!    indices renumber (`rinse` 0, `pack` 1, `move` 2).
//! 3. Stash failure (empty passphrase): a completion event for `stash` is
//!    still emitted, following phases emit completion events with the message
//!    `"not run due to prior failure"`, and `RaidResult.success == false`.
//! 4. `StashEmpty` is a no-op: the run continues (`rinse`/`pack`/`move`
//!    events present), no event message contains `"fail"`.
//! 5. No event message leaks the raw fixture secret value in any run
//!    (dry-run, commit, and stash-failure).
//!
//! The `overall_percent` clamp is left to Dev's unit tests; the integration
//! cases 1–3 exercise the formula through the full range (25/50/75/100 and
//! 33/66/100).
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no sleeps.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    raid, AgeIdentity, AppContext, OperationKind, PackPhaseOpts, ProgressEvent, ProgressSink,
    RaccConfig, RaidOptions, RaidResult, RinsePhaseOpts, RunMode, SensitiveRisk, StashPhaseOpts,
};
use tempfile::TempDir;
use zeroize::Zeroizing;

/// A long, distinctive password value used to prove no event leaks it.
const PASSWORD_VALUE: &str = "SUPERSECRETVALUE_raid_progress_4711";

/// Test passphrase for age encryption (must be non-empty).
const PASSPHRASE: &str = "raccpack a3.2 raid progress test passphrase";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Create a hermetic workspace root with `proj/` and `den/` sibling paths.
fn workspace() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().expect("create temp dir");
    let proj = temp.path().join("proj");
    let den = temp.path().join("den");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj, den)
}

/// Build an `AppContext` for a `raid` run: scan root = the project itself, and
/// the den is always an explicit TempDir path so the real `~/.raccpack/den` is
/// never touched.
fn ctx_for(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default()
        .with_scan_root(project_root)
        .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Default raid options for a project (all phases enabled).
fn raid_options(project: &Path) -> RaidOptions {
    RaidOptions {
        project: project.to_path_buf(),
        ..RaidOptions::default()
    }
}

/// Raid options with the stash phase disabled (rinse + pack enabled).
fn raid_options_stash_disabled(project: &Path) -> RaidOptions {
    RaidOptions {
        project: project.to_path_buf(),
        stash: StashPhaseOpts {
            enabled: false,
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

/// A passphrase identity backed by the shared test passphrase.
fn identity() -> AgeIdentity {
    AgeIdentity::Passphrase(Zeroizing::new(PASSPHRASE.to_string()))
}

/// An empty passphrase identity (age encryption must reject it).
fn empty_identity() -> AgeIdentity {
    AgeIdentity::Passphrase(Zeroizing::new(String::new()))
}

/// Sink that collects emitted events for assertions.
#[derive(Default)]
struct RecordingSink {
    events: Vec<ProgressEvent>,
}

impl ProgressSink for RecordingSink {
    fn emit(&mut self, event: ProgressEvent) {
        self.events.push(event);
    }
}

/// Run `raid` recording every progress event; returns the result and the
/// events. `identity` is passed through verbatim.
fn raid_recorded(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
) -> (RaidResult, Vec<ProgressEvent>) {
    let mut sink = RecordingSink::default();
    let result = raid(ctx, opts, identity, &mut sink).expect("raid should return a result");
    (result, sink.events)
}

/// The raid-level completion events: `OperationKind::Raid` and
/// `phase_complete`. Nested facade events (stash/rinse/pack emit their own
/// `OperationKind` events through the same sink) are filtered out here.
fn raid_completions(events: &[ProgressEvent]) -> Vec<&ProgressEvent> {
    events
        .iter()
        .filter(|e| e.operation == OperationKind::Raid && e.phase_complete)
        .collect()
}

/// A project fixture with a sensitive file, a trash dir and a normal file.
fn full_fixture(proj: &Path) -> Vec<PathBuf> {
    vec![
        write(proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n")),
        write(proj, "README.md", "# demo project\n"),
        write(proj, "node_modules/pkg/index.js", "module.exports = 1;\n"),
    ]
}

// --- Case 1: all enabled, DryRun ---------------------------------------------

#[test]
fn all_enabled_emits_one_completion_per_phase_with_overall_25_50_75_100() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);
    let opts = raid_options(&proj);

    let (result, events) = raid_recorded(&ctx, &opts, Some(&identity()));
    assert!(result.success, "dry run must succeed: {result:?}");

    let completions = raid_completions(&events);
    assert_eq!(
        completions.len(),
        4,
        "one completion event per planned phase, got {completions:?}"
    );

    let phases: Vec<&str> = completions.iter().map(|e| e.phase.as_str()).collect();
    assert_eq!(phases, vec!["stash", "rinse", "pack", "move"]);

    let indices: Vec<u32> = completions.iter().map(|e| e.phase_index).collect();
    assert_eq!(indices, vec![0, 1, 2, 3]);

    let overalls: Vec<u8> = completions.iter().map(|e| e.overall_percent).collect();
    assert_eq!(
        overalls,
        vec![25, 50, 75, 100],
        "overall = (phase_index*100 + percent) / phase_count with percent==100"
    );

    for event in &completions {
        assert_eq!(event.operation, OperationKind::Raid);
        assert_eq!(event.phase_count, 4, "move is implicit: 3 enabled + 1");
        assert!(
            event.phase_complete,
            "completion events must be marked done"
        );
        assert!(
            event.overall_percent <= 100,
            "overall_percent must be clamped to 100"
        );
    }

    let last = completions.last().expect("four completions");
    assert_eq!(last.phase, "move");
    assert_eq!(last.overall_percent, 100);
    assert!(last.phase_complete);
    assert_eq!(last.phase_index, last.phase_count - 1);
}

// --- Case 2: disabled stash ---------------------------------------------------

#[test]
fn disabled_stash_renumbers_indices_and_emits_no_stash_event() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);
    let opts = raid_options_stash_disabled(&proj);

    let (result, events) = raid_recorded(&ctx, &opts, None);
    assert!(
        result.success,
        "stash-disabled raid must succeed: {result:?}"
    );

    assert!(
        events.iter().all(|e| e.phase != "stash"),
        "no event may carry phase \"stash\" when stash is disabled: {events:?}"
    );

    let completions = raid_completions(&events);
    assert_eq!(
        completions.len(),
        3,
        "rinse + pack + move, got {completions:?}"
    );
    for event in &completions {
        assert_eq!(event.phase_count, 3, "2 enabled + move");
    }

    let phases: Vec<&str> = completions.iter().map(|e| e.phase.as_str()).collect();
    assert_eq!(phases, vec!["rinse", "pack", "move"]);

    let indices: Vec<u32> = completions.iter().map(|e| e.phase_index).collect();
    assert_eq!(indices, vec![0, 1, 2], "indices must renumber after stash");

    let overalls: Vec<u8> = completions.iter().map(|e| e.overall_percent).collect();
    assert_eq!(overalls, vec![33, 66, 100]);
}

// --- Case 3: stash failure → fail-fast ----------------------------------------

#[test]
fn stash_failure_emits_completions_with_not_run_messages() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = raid_options(&proj);

    let (result, events) = raid_recorded(&ctx, &opts, Some(&empty_identity()));
    assert!(
        !result.success,
        "a stash failure must fail the raid: {result:?}"
    );

    let completions = raid_completions(&events);
    assert_eq!(
        completions.len(),
        4,
        "every planned phase still emits a completion event: {completions:?}"
    );

    assert!(
        completions.iter().any(|e| e.phase == "stash"),
        "the failed phase must still emit its completion event"
    );

    for phase in ["rinse", "pack", "move"] {
        let event = completions
            .iter()
            .find(|e| e.phase == phase)
            .unwrap_or_else(|| panic!("missing completion event for {phase}"));
        assert!(
            event.message.contains("not run due to prior failure"),
            "{phase} must carry the fail-fast message, got {:?}",
            event.message
        );
    }
}

// --- Case 4: StashEmpty is a no-op, the run continues --------------------------

#[test]
fn stash_empty_continues_the_run_and_emits_no_fail() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, "README.md", "# clean project, no secrets\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = raid_options(&proj);

    let (result, events) = raid_recorded(&ctx, &opts, Some(&identity()));
    assert!(
        result.success,
        "an empty stash selection must not fail the raid: {result:?}"
    );

    let completions = raid_completions(&events);
    for phase in ["rinse", "pack", "move"] {
        assert!(
            completions.iter().any(|e| e.phase == phase),
            "the run must continue to {phase}: {completions:?}"
        );
    }

    let stash = completions
        .iter()
        .find(|e| e.phase == "stash")
        .unwrap_or_else(|| panic!("stash completion must be emitted: {completions:?}"));
    assert!(
        stash.message.contains("nothing to stash"),
        "stash must be a no-op, got {:?}",
        stash.message
    );

    for event in &events {
        assert!(
            !event.message.to_lowercase().contains("fail"),
            "no event may mention a failure in a StashEmpty run: {:?}",
            event.message
        );
    }
}

// --- Case 5: no raw secret value in any event message --------------------------

#[test]
fn no_event_message_leaks_the_raw_secret_in_any_run() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);

    let all_runs: Vec<Vec<ProgressEvent>> = {
        let dry = ctx_for(&proj, &den, RunMode::DryRun);
        let (_, dry_events) = raid_recorded(&dry, &raid_options(&proj), Some(&identity()));

        let commit = ctx_for(&proj, &den, RunMode::Commit);
        let (_, commit_events) = raid_recorded(&commit, &raid_options(&proj), Some(&identity()));

        let failing = ctx_for(&proj, &den, RunMode::Commit);
        let (_, fail_events) =
            raid_recorded(&failing, &raid_options(&proj), Some(&empty_identity()));

        vec![dry_events, commit_events, fail_events]
    };

    for (idx, events) in all_runs.iter().enumerate() {
        for event in events {
            assert!(
                !event.message.contains(PASSWORD_VALUE),
                "run {idx} leaked the raw secret in {:?}",
                event.message
            );
        }
    }
}
