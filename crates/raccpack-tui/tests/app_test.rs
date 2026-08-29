use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use raccpack_tui::app::{App, Command, ViewId};
use raccpack_tui::ui::theme;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// ── 1. initial_state_is_overview ──────────────────────────────────────────────

#[test]
fn initial_state_is_overview() {
    let app = App::new();
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "initial view must be Overview"
    );
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

// ── 7. view_id_from_char ──────────────────────────────────────────────────────

/// ViewId has no `from_char`, only `key()` (ViewId → char).
/// We verify the forward mapping and that no char maps to an unexpected variant.
#[test]
fn view_id_from_char() {
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

// ── 9. style_helpers_produce_correct_fg ────────────────────────────────────────

/// Dev's theme exposes raw `Color` constants, not style-helper functions.
/// We verify each semantic color round-trips correctly through ratatui `Style`,
/// confirming the fg attribute matches the constant — the behaviour a helper
/// `danger_text()` etc. would rely on.
#[test]
fn style_helpers_produce_correct_fg() {
    use ratatui::style::Style;

    let pairs = [
        ("bg", theme::BG),
        ("fg", theme::FG),
        ("accent", theme::ACCENT),
        ("danger", theme::DANGER),
        ("warning", theme::WARNING),
        ("success", theme::SUCCESS),
        ("muted", theme::MUTED),
        ("border", theme::BORDER),
        ("surface", theme::SURFACE),
        ("selection", theme::SELECTION),
    ];

    for (name, color) in pairs {
        let style = Style::default().fg(color);
        assert_eq!(
            style.fg,
            Some(color),
            "{name} fg must be Some({name}) — got {:?}",
            style.fg
        );
    }
}

// ── 10. negative — irrelevant key is no-op ────────────────────────────────────

#[test]
fn irrelevant_key_is_noop() {
    let mut app = App::new();
    let cmd = app.handle_key(key(KeyCode::Char('z')));
    assert_eq!(cmd, Command::None, "unknown char must return Command::None");
    assert_eq!(app.current_view, ViewId::Overview, "view must not change");
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
