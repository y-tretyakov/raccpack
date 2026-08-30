//! Raid flow key routing tests.

use super::super::*;
use super::{key, preview_result, raid_flow_in};
use crossterm::event::KeyCode;

#[test]
fn raid_flow_blocks_every_key_when_open() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.raid_flow = Some(raid_flow_in(raid::FlowPhase::Preparing));

    for code in [
        KeyCode::Char('2'),
        KeyCode::Char('j'),
        KeyCode::Tab,
        KeyCode::Char('q'),
    ] {
        assert_eq!(
            app.handle_key(key(code)),
            Command::None,
            "{code:?} must be swallowed by the active flow"
        );
    }
    assert_eq!(app.current_view, ViewId::Projects, "view must not change");
    assert!(app.running, "q must not quit while the flow is open");
}

#[test]
fn raid_flow_y_confirm_returns_raid_run() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(raid::FlowPhase::Preview(preview_result())));

    assert_eq!(app.handle_key(key(KeyCode::Char('y'))), Command::RaidRun);
    assert!(
        app.raid_flow.is_some(),
        "flow stays open until the run ends"
    );
}

#[test]
fn raid_flow_toggles_update_options_not_commands() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(raid::FlowPhase::Preview(preview_result())));

    assert_eq!(app.handle_key(key(KeyCode::Char('K'))), Command::None);
    assert_eq!(app.handle_key(key(KeyCode::Char('S'))), Command::None);
    assert_eq!(app.handle_key(key(KeyCode::Char('m'))), Command::None);
    let opts = app.raid_flow.as_ref().unwrap().options;
    assert!(opts.keep_sources);
    assert!(opts.skip_stash);
    assert_eq!(opts.mode, raccpack_core::app::OrchestrationMode::FailFast);
}

#[test]
fn raid_flow_passphrase_confirm_stores_on_flow() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(
        raid::FlowPhase::Passphrase(Default::default()),
    ));

    for c in "s3cret".chars() {
        assert_eq!(app.handle_key(key(KeyCode::Char(c))), Command::None);
    }
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::None);
    for c in "s3cret".chars() {
        assert_eq!(app.handle_key(key(KeyCode::Char(c))), Command::None);
    }
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::RaidRun);
    assert_eq!(
        app.raid_flow
            .as_mut()
            .unwrap()
            .take_passphrase()
            .map(|p| p.to_string()),
        Some("s3cret".to_string())
    );
}

#[test]
fn esc_on_preview_cancels_flow_via_command() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(raid::FlowPhase::Preview(preview_result())));
    assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::RaidCancel);
}

#[test]
fn esc_on_done_closes_flow_in_app() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(raid::FlowPhase::Done(preview_result())));
    assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::None);
    assert!(app.raid_flow.is_none(), "Done/Esc closes the flow");
}

#[test]
fn app_debug_redacts_typed_passphrase() {
    let mut app = App::new();
    app.raid_flow = Some(raid_flow_in(
        raid::FlowPhase::Passphrase(Default::default()),
    ));
    for c in "hunter2-hunter2-ultra-secret".chars() {
        app.handle_key(key(KeyCode::Char(c)));
    }
    let debug = format!("{app:?}");
    assert!(
        !debug.contains("hunter2-hunter2"),
        "App Debug must not leak the typed passphrase: {debug}"
    );
}
