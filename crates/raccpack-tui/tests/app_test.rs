use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use raccpack_tui::app::{App, Command, Focus, ViewId};
use raccpack_tui::ui::theme;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// ── 1. initial state ──────────────────────────────────────────────────────────

#[test]
fn initial_state_is_overview() {
    let app = App::new();
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "initial view must be Overview"
    );
    assert_eq!(app.focus, Focus::Sidebar, "initial focus must be Sidebar");
    assert!(!app.help_visible, "help must start hidden");
    assert!(app.running, "app must start running");
}

// ── 2. number_keys_navigate_to_views ──────────────────────────────────────────

#[test]
fn number_keys_navigate_to_views() {
    let mut app = App::new();

    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(
        app.current_view,
        ViewId::Projects,
        "'2' must select Projects"
    );

    app.handle_key(key(KeyCode::Char('3')));
    assert_eq!(
        app.current_view,
        ViewId::Findings,
        "'3' must select Findings"
    );

    app.handle_key(key(KeyCode::Char('4')));
    assert_eq!(
        app.current_view,
        ViewId::Operations,
        "'4' must select Operations"
    );

    app.handle_key(key(KeyCode::Char('1')));
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "'1' must select Overview"
    );
}

// ── 2b. Tab / Shift+Tab cycle views ───────────────────────────────────────────

#[test]
fn tab_cycles_views_forward() {
    let mut app = App::new();
    for expected in [
        ViewId::Projects,
        ViewId::Findings,
        ViewId::Operations,
        ViewId::Overview,
    ] {
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(
            app.current_view, expected,
            "Tab must advance to {expected:?}"
        );
    }
}

#[test]
fn shift_tab_cycles_views_backward() {
    let mut app = App::new();
    for expected in [
        ViewId::Operations,
        ViewId::Findings,
        ViewId::Projects,
        ViewId::Overview,
    ] {
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(
            app.current_view, expected,
            "Shift+Tab must go back to {expected:?}"
        );
    }
}

// ── 2c. sidebar j/k/arrows move between views ─────────────────────────────────

#[test]
fn sidebar_jk_move_between_views() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.current_view, ViewId::Projects);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.current_view, ViewId::Findings);
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.current_view, ViewId::Projects);
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.current_view, ViewId::Overview);
}

#[test]
fn sidebar_arrows_move_between_views() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.current_view, ViewId::Projects);
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.current_view, ViewId::Overview);
}

// ── 3. q_key_returns_quit ─────────────────────────────────────────────────────

#[test]
fn q_key_returns_quit() {
    let mut app = App::new();
    let cmd = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(cmd, Command::Quit, "'q' must return Command::Quit");
    assert!(!app.running, "running must be false after 'q'");
}

// ── 4. question_mark_toggles_help ─────────────────────────────────────────────

#[test]
fn question_mark_toggles_help() {
    let mut app = App::new();

    assert!(!app.help_visible, "help starts hidden");

    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.help_visible, "'?' must open help");

    app.handle_key(key(KeyCode::Char('?')));
    assert!(!app.help_visible, "'?' must toggle help off again");
}

// ── 5. esc_closes_help ────────────────────────────────────────────────────────

#[test]
fn esc_closes_help() {
    let mut app = App::new();
    app.help_visible = true;

    app.handle_key(key(KeyCode::Esc));
    assert!(!app.help_visible, "Esc must close help");
}

// ── 6. keys_ignored_when_help_open ────────────────────────────────────────────

#[test]
fn keys_ignored_when_help_open() {
    let mut app = App::new();
    app.help_visible = true;

    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "view must not change while help is visible"
    );
    assert!(app.help_visible, "help must remain open");
}

#[test]
fn tab_and_arrows_blocked_when_help_open() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.help_visible);

    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "navigation must be blocked while help is visible"
    );
    assert_eq!(
        app.focus,
        Focus::Sidebar,
        "focus must not change while help is visible"
    );
    assert!(app.help_visible, "help must stay open");
}

// ── 7. view_id relationship ───────────────────────────────────────────────────

#[test]
fn view_id_keys() {
    assert_eq!(ViewId::Overview.key(), '1');
    assert_eq!(ViewId::Projects.key(), '2');
    assert_eq!(ViewId::Findings.key(), '3');
    assert_eq!(ViewId::Operations.key(), '4');

    // round-trip: key → navigation → same ViewId
    let mut app = App::new();
    for (ch, expected) in [
        ('2', ViewId::Projects),
        ('3', ViewId::Findings),
        ('4', ViewId::Operations),
        ('1', ViewId::Overview),
    ] {
        app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(
            app.current_view, expected,
            "'{ch}' must map to {expected:?}"
        );
    }

    // '5' has no ViewId variant — should be a no-op
    app.handle_key(key(KeyCode::Char('5')));
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "'5' must not change view"
    );
}

#[test]
fn view_id_prev_and_next_round_trip() {
    for view in [
        ViewId::Overview,
        ViewId::Projects,
        ViewId::Findings,
        ViewId::Operations,
    ] {
        assert_eq!(view.next().prev(), view, "next().prev() round-trip");
        assert_eq!(view.prev().next(), view, "prev().next() round-trip");
    }
}

// ── 7b. focus movement ────────────────────────────────────────────────────────

#[test]
fn h_and_l_toggle_focus() {
    let mut app = App::new();
    assert_eq!(app.focus, Focus::Sidebar);

    app.handle_key(key(KeyCode::Char('l')));
    assert_eq!(app.focus, Focus::Main, "'l' must focus Main");

    app.handle_key(key(KeyCode::Char('h')));
    assert_eq!(app.focus, Focus::Sidebar, "'h' must focus Sidebar");
}

#[test]
fn left_and_right_arrows_toggle_focus() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.focus, Focus::Main, "Right must focus Main");
    app.handle_key(key(KeyCode::Left));
    assert_eq!(app.focus, Focus::Sidebar, "Left must focus Sidebar");
}

// ── 7c. projects table rows (Focus::Main) ─────────────────────────────────────

#[test]
fn projects_jk_move_rows_without_changing_view() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;
    app.sniff_state.projects = vec![
        raccpack_tui::app::sniff::ProjectRow {
            name: "a".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
        raccpack_tui::app::sniff::ProjectRow {
            name: "b".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
    ];
    app.sniff_state.table_state.select(Some(0));

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(
        app.sniff_state.selected_project().unwrap().name,
        "b",
        "'j' (Main focus) must move to the next row"
    );
    assert_eq!(
        app.current_view,
        ViewId::Projects,
        "row movement must not change view"
    );

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(
        app.sniff_state.selected_project().unwrap().name,
        "a",
        "'k' (Main focus) must move to the previous row"
    );
}

// ── 8. nocturnal_theme_colors_are_distinct ─────────────────────────────────────

#[test]
fn nocturnal_theme_colors_are_distinct() {
    // Pairwise distinctness among the semantic palette
    assert_ne!(
        theme::BG,
        theme::FG,
        "background must differ from foreground"
    );
    assert_ne!(theme::ACCENT, theme::MUTED, "accent must differ from muted");
    assert_ne!(
        theme::DANGER,
        theme::SUCCESS,
        "danger must differ from success"
    );
    assert_ne!(
        theme::ACCENT,
        theme::DANGER,
        "accent must differ from danger"
    );
    assert_ne!(
        theme::WARNING,
        theme::SUCCESS,
        "warning must differ from success"
    );
    assert_ne!(
        theme::SURFACE,
        theme::SELECTION,
        "surface must differ from selection"
    );
}

// ── 10. negative — irrelevant key is no-op ────────────────────────────────────

#[test]
fn irrelevant_key_is_noop() {
    let mut app = App::new();
    let cmd = app.handle_key(key(KeyCode::Char('z')));
    assert_eq!(cmd, Command::None, "unknown char must return Command::None");
    assert_eq!(app.current_view, ViewId::Overview, "view must be unchanged");
    assert_eq!(app.focus, Focus::Sidebar, "focus must be unchanged");
    assert!(!app.help_visible, "help must not toggle");
    assert!(app.running, "running must remain true");
}

// ── bonus: q_blocked_when_help_open ────────────────────────────────────────────

#[test]
fn q_blocked_when_help_open() {
    let mut app = App::new();
    app.help_visible = true;

    let cmd = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(cmd, Command::None, "q must be ignored when help is open");
    assert!(app.running, "running must stay true");
}
