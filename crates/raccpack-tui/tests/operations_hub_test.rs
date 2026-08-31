//! Integration tests for the Operations hub screen (state + key routing).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use raccpack_tui::app::operations::ALL_OPERATIONS;
use raccpack_tui::app::{App, Command, Focus, OperationKind, ViewId};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app_on_operations(selected: OperationKind) -> App {
    let mut app = App::new();
    app.current_view = ViewId::Operations;
    app.focus = Focus::Main;
    app.operations_state.selected = selected;
    app
}

fn with_project(app: &mut App) {
    app.sniff_state.projects = vec![raccpack_tui::app::sniff::ProjectRow {
        name: "proj".into(),
        language: None,
        frameworks: vec![],
        size_bytes: 0,
        is_git_repo: false,
        path: std::path::PathBuf::from("/tmp/proj"),
    }];
    app.sniff_state.table_state.select(Some(0));
}

#[test]
fn operations_offer_four_operations_in_order() {
    let labels: Vec<&str> = ALL_OPERATIONS.iter().map(|k| k.label()).collect();
    assert_eq!(labels, vec!["Pack", "Stash", "Rinse", "Raid"]);
}

#[test]
fn jk_and_arrows_move_the_selection_with_main_focus() {
    let mut app = app_on_operations(OperationKind::Pack);

    assert_eq!(app.handle_key(key(KeyCode::Char('j'))), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Stash);
    assert_eq!(app.handle_key(key(KeyCode::Down)), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Rinse);
    assert_eq!(app.handle_key(key(KeyCode::Char('k'))), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Stash);
    assert_eq!(app.handle_key(key(KeyCode::Up)), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Pack);
}

#[test]
fn g_and_g_jump_to_first_and_last_operation() {
    let mut app = app_on_operations(OperationKind::Pack);
    assert_eq!(app.handle_key(key(KeyCode::Char('G'))), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Raid);
    assert_eq!(app.handle_key(key(KeyCode::Char('g'))), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Pack);
}

#[test]
fn enter_activates_selected_operation_when_project_selected() {
    let mut app = app_on_operations(OperationKind::Raid);
    with_project(&mut app);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::OpenOperation);
}

#[test]
fn enter_without_selected_project_is_a_noop() {
    let mut app = app_on_operations(OperationKind::Raid);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::None);
    assert!(app.raid_flow.is_none());
}

#[test]
fn shortcut_keys_jump_the_selection() {
    let mut app = app_on_operations(OperationKind::Pack);
    for (code, expected) in [
        ('s', OperationKind::Stash),
        ('r', OperationKind::Rinse),
        ('d', OperationKind::Raid),
        ('p', OperationKind::Pack),
    ] {
        assert_eq!(app.handle_key(key(KeyCode::Char(code))), Command::None);
        assert_eq!(app.operations_state.selected, expected);
    }
}

#[test]
fn shortcuts_do_not_shadow_quit_or_help() {
    let mut app = app_on_operations(OperationKind::Pack);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Command::Quit);
    assert!(!app.running);

    let mut app = app_on_operations(OperationKind::Pack);
    assert_eq!(app.handle_key(key(KeyCode::Char('?'))), Command::None);
    assert!(app.help_visible, "? must still open help on Operations");
}

#[test]
fn sidebar_focus_moves_views_not_selection() {
    let mut app = App::new();
    app.current_view = ViewId::Operations;
    app.focus = Focus::Sidebar;

    assert_eq!(app.handle_key(key(KeyCode::Char('j'))), Command::None);
    assert_eq!(app.current_view, ViewId::Overview, "sidebar j cycles views");
    assert_eq!(
        app.operations_state.selected,
        OperationKind::Pack,
        "sidebar j must not move the operations selection"
    );

    assert_eq!(app.handle_key(key(KeyCode::Char('l'))), Command::None);
    assert_eq!(app.focus, Focus::Main, "sidebar Enter/Right focuses main");
}

#[test]
fn stub_notice_blocks_keys_until_dismissed() {
    let mut app = app_on_operations(OperationKind::Pack);
    app.operations_state.stub = Some(OperationKind::Pack);

    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('q'),
        KeyCode::Tab,
        KeyCode::Char('2'),
        KeyCode::Char('p'),
    ] {
        assert_eq!(
            app.handle_key(key(code)),
            Command::None,
            "{code:?} must be swallowed while the stub notice is open"
        );
    }
    assert!(app.running, "q must not quit while the stub notice is open");
    assert_eq!(app.current_view, ViewId::Operations);
    assert_eq!(app.operations_state.selected, OperationKind::Pack);

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::None);
    assert!(
        app.operations_state.stub.is_none(),
        "Esc dismisses the notice"
    );

    // After dismissal the screen behaves normally again.
    assert_eq!(app.handle_key(key(KeyCode::Char('j'))), Command::None);
    assert_eq!(app.operations_state.selected, OperationKind::Stash);
}

#[test]
fn squadron_view_change_after_stub_dismissal() {
    let mut app = app_on_operations(OperationKind::Stash);
    app.operations_state.stub = Some(OperationKind::Stash);
    assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::None);
    assert_eq!(app.handle_key(key(KeyCode::Char('2'))), Command::None);
    assert_eq!(app.current_view, ViewId::Projects);
}
