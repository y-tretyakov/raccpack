//! Unit tests for the raid flow state machine and passphrase entry.

use super::*;

fn flow_started() -> RaidFlow {
    RaidFlow::new(
        PathBuf::from("/tmp/proj"),
        PathBuf::from("/tmp/den"),
        RaidFlowOptions::default(),
    )
}

fn done_result() -> RaidResult {
    RaidResult {
        project_path: PathBuf::from("/tmp/proj"),
        stages: Vec::new(),
        stash: None,
        rinse: None,
        pack: None,
        den_artifacts: Vec::new(),
        success: true,
        dry_run: false,
        rolled_back: false,
        rollback_warnings: Vec::new(),
    }
}

#[test]
fn options_default_atomic_with_stash_on_remove_sources() {
    let opts = RaidFlowOptions::default();
    assert!(!opts.keep_sources);
    assert!(!opts.skip_stash);
    assert_eq!(opts.mode, OrchestrationMode::Atomic);
}

#[test]
fn option_toggles_flip_independently() {
    let mut opts = RaidFlowOptions::default();
    opts.toggle_keep_sources();
    assert!(opts.keep_sources);
    opts.toggle_keep_sources();
    assert!(!opts.keep_sources);
    opts.toggle_skip_stash();
    assert!(opts.skip_stash);
    opts.toggle_mode();
    assert_eq!(opts.mode, OrchestrationMode::FailFast);
    opts.toggle_mode();
    assert_eq!(opts.mode, OrchestrationMode::Atomic);
}

#[test]
fn preview_enter_with_stash_asks_for_passphrase() {
    assert_eq!(
        flow_started().handle_key(KeyCode::Char('y')),
        Some(RaidCommand::PreviewConfirm)
    );
    assert_eq!(
        flow_started().handle_key(KeyCode::Enter),
        Some(RaidCommand::PreviewConfirm)
    );
}

#[test]
fn preview_n_esc_cancel_and_q_is_not_a_flow_key() {
    let mut flow = flow_started();
    assert_eq!(
        flow.handle_key(KeyCode::Char('n')),
        Some(RaidCommand::PreviewCancel)
    );
    assert_eq!(
        flow.handle_key(KeyCode::Esc),
        Some(RaidCommand::PreviewCancel)
    );
    assert_eq!(flow.handle_key(KeyCode::Char('q')), None);
    assert_eq!(flow.handle_key(KeyCode::Char('j')), None);
}

#[test]
fn preview_k_s_m_toggle_options() {
    let mut flow = flow_started();
    flow.handle_key(KeyCode::Char('K'));
    assert!(flow.options.keep_sources);
    flow.handle_key(KeyCode::Char('S'));
    assert!(flow.options.skip_stash);
    flow.handle_key(KeyCode::Char('m'));
    assert_eq!(flow.options.mode, OrchestrationMode::FailFast);
}

#[test]
fn skip_stash_enter_runs_directly_without_passphrase() {
    let mut flow = flow_started();
    flow.options.skip_stash = true;
    assert_eq!(flow.handle_key(KeyCode::Char('y')), Some(RaidCommand::Run));
    assert_eq!(flow.handle_key(KeyCode::Enter), Some(RaidCommand::Run));
}

#[test]
fn passphrase_two_inputs_confirm_on_match() {
    let mut flow = flow_started();
    flow.start_passphrase();
    for c in "s3cret".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    assert_eq!(flow.handle_key(KeyCode::Enter), None);
    assert!(matches!(
        flow.phase,
        FlowPhase::Passphrase(ref input) if input.step().is_confirm()
    ));
    for c in "s3cret".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    match flow.handle_key(KeyCode::Enter) {
        Some(RaidCommand::PassphraseConfirm(p)) => assert_eq!(*p, "s3cret"),
        other => panic!("expected confirmed passphrase, got {other:?}"),
    }
}

#[test]
fn passphrase_mismatch_resets_to_first_entry() {
    let mut flow = flow_started();
    flow.start_passphrase();
    for c in "abc".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    flow.handle_key(KeyCode::Enter);
    for c in "xyz".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    assert_eq!(flow.handle_key(KeyCode::Enter), None);
    match &flow.phase {
        FlowPhase::Passphrase(input) => {
            assert!(!input.step().is_confirm(), "must return to the first entry");
            assert_eq!(input.first_len(), 0, "first entry must be reset");
            assert!(input.error().is_some(), "mismatch must surface an error");
        }
        other => panic!("expected Passphrase phase, got {other:?}"),
    }
}

#[test]
fn passphrase_backspace_pops_active_input() {
    let mut flow = flow_started();
    flow.start_passphrase();
    for c in "ab".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    flow.handle_key(KeyCode::Backspace);
    flow.handle_key(KeyCode::Backspace);
    match &flow.phase {
        FlowPhase::Passphrase(input) => assert_eq!(input.first_len(), 0),
        other => panic!("expected Passphrase phase, got {other:?}"),
    }
}

#[test]
fn passphrase_esc_cancels() {
    let mut flow = flow_started();
    flow.start_passphrase();
    flow.handle_key(KeyCode::Char('x'));
    assert_eq!(
        flow.handle_key(KeyCode::Esc),
        Some(RaidCommand::PassphraseCancel)
    );
}

#[test]
fn passphrase_control_chars_are_ignored() {
    let mut flow = flow_started();
    flow.start_passphrase();
    flow.handle_key(KeyCode::Char('\u{7}'));
    match &flow.phase {
        FlowPhase::Passphrase(input) => assert_eq!(input.first_len(), 0),
        other => panic!("expected Passphrase phase, got {other:?}"),
    }
}

#[test]
fn running_blocks_every_key() {
    let mut flow = flow_started();
    flow.phase = FlowPhase::Running;
    for k in [
        KeyCode::Char('y'),
        KeyCode::Enter,
        KeyCode::Char('n'),
        KeyCode::Esc,
        KeyCode::Char('q'),
        KeyCode::Char('K'),
    ] {
        assert_eq!(
            flow.handle_key(k),
            None,
            "{k:?} must be swallowed while running"
        );
    }
}

#[test]
fn done_and_failed_close_on_enter_or_esc() {
    let mut done_flow = flow_started();
    done_flow.phase = FlowPhase::Done(done_result());
    assert_eq!(
        done_flow.handle_key(KeyCode::Enter),
        Some(RaidCommand::Close)
    );
    assert_eq!(done_flow.handle_key(KeyCode::Esc), Some(RaidCommand::Close));

    let mut failed_flow = flow_started();
    failed_flow.phase = FlowPhase::Failed("boom".to_string());
    assert_eq!(
        failed_flow.handle_key(KeyCode::Enter),
        Some(RaidCommand::Close)
    );
    assert_eq!(
        failed_flow.handle_key(KeyCode::Esc),
        Some(RaidCommand::Close)
    );
}

fn raid_progress(phase: &str, message: &str, overall: u8, complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: raccpack_core::app::OperationKind::Raid,
        phase: phase.to_string(),
        phase_index: 0,
        phase_count: 4,
        percent: 100,
        overall_percent: overall,
        message: message.to_string(),
        phase_complete: complete,
    }
}

#[test]
fn on_progress_builds_pipeline_and_tracks_done_phases() {
    let mut flow = flow_started();
    flow.start_running();
    flow.on_progress(&raid_progress("stash", "stashed 1 files", 25, true));
    assert_eq!(flow.overall_percent, 25);
    assert_eq!(flow.pipeline.len(), 4);
    assert_eq!(flow.pipeline[0].name, "stash");
    assert!(flow.pipeline[0].done);
    assert!(flow.pipeline[1].current, "rinse becomes the current phase");

    flow.on_progress(&raid_progress("rinse", "rinsed", 50, true));
    assert!(flow.pipeline[1].done);
    assert!(flow.pipeline[2].current, "pack becomes the current phase");
    assert_eq!(flow.message, "rinsed");
}

#[test]
fn on_progress_ignores_unknown_phases_but_updates_percent() {
    let mut flow = flow_started();
    flow.start_running();
    flow.on_progress(&raid_progress("rollback", "rolled back", 99, true));
    assert_eq!(flow.pipeline.len(), 4);
    assert!(!flow.pipeline.iter().any(|l| l.name == "rollback"));
    assert_eq!(flow.overall_percent, 99);
}

#[test]
fn skip_stash_on_progress_omits_stash_phase() {
    let mut flow = flow_started();
    flow.options.skip_stash = true;
    flow.start_running();
    flow.on_progress(&raid_progress("rinse", "rinsed", 33, true));
    let names: Vec<_> = flow.pipeline.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["rinse", "pack", "move"]);
}

#[test]
fn debug_output_redacts_typed_and_confirmed_passphrases() {
    let mut flow = flow_started();
    flow.start_passphrase();
    for c in "super-secret-passphrase".chars() {
        flow.handle_key(KeyCode::Char(c));
    }
    let debug_typed = format!("{flow:?}");
    assert!(
        !debug_typed.contains("super-secret-passphrase"),
        "typed passphrase must never appear in Debug: {debug_typed}"
    );
    assert!(debug_typed.contains("hidden"));

    flow.store_confirmed(Zeroizing::new("another-secret-value".to_string()));
    let debug_confirmed = format!("{flow:?}");
    assert!(
        !debug_confirmed.contains("another-secret-value"),
        "confirmed passphrase must never appear in Debug: {debug_confirmed}"
    );
}

#[test]
fn take_passphrase_returns_and_clears() {
    let mut flow = flow_started();
    assert_eq!(flow.take_passphrase(), None);
    flow.store_confirmed(Zeroizing::new("x".to_string()));
    let taken = flow.take_passphrase();
    assert_eq!(taken.map(|p| p.to_string()), Some("x".to_string()));
    assert_eq!(flow.take_passphrase(), None, "consume is one-shot");
}

#[test]
fn passphrase_input_debug_never_shows_values() {
    let mut input = PassphraseInput::new();
    for c in "P4ssw0rd!q7".chars() {
        input.push_char(c);
    }
    let debug = format!("{input:?}");
    assert!(
        !debug.contains("P4ssw0rd"),
        "typed value must never appear in Debug: {debug}"
    );
    assert!(debug.contains("hidden"));
}
