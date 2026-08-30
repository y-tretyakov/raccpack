//! App key-routing tests: navigation, focus, per-view content keys.

use super::super::*;
use super::{key, project_row};

#[test]
fn initial_state() {
    let app = App::new();
    assert_eq!(app.current_view, ViewId::Overview);
    assert_eq!(app.focus, Focus::Sidebar);
    assert!(!app.help_visible);
    assert!(app.running);
}

#[test]
fn navigation_switches_views() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('2'))),
        Command::None
    );
    assert_eq!(app.current_view, ViewId::Projects);

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('3'))),
        Command::None
    );
    assert_eq!(app.current_view, ViewId::Findings);

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('4'))),
        Command::None
    );
    assert_eq!(app.current_view, ViewId::Operations);

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('1'))),
        Command::None
    );
    assert_eq!(app.current_view, ViewId::Overview);
}

#[test]
fn tab_cycles_views_forward() {
    let mut app = App::new();
    let expected = [
        ViewId::Projects,
        ViewId::Findings,
        ViewId::Operations,
        ViewId::Overview,
    ];
    for &view in &expected {
        app.handle_key(key(crossterm::event::KeyCode::Tab));
        assert_eq!(app.current_view, view);
    }
}

#[test]
fn backtab_cycles_views_backward() {
    let mut app = App::new();
    let expected = [
        ViewId::Operations,
        ViewId::Findings,
        ViewId::Projects,
        ViewId::Overview,
    ];
    for &view in &expected {
        app.handle_key(key(crossterm::event::KeyCode::BackTab));
        assert_eq!(app.current_view, view);
    }
}

#[test]
fn sidebar_arrows_change_view() {
    let mut app = App::new();
    app.handle_key(key(crossterm::event::KeyCode::Down));
    assert_eq!(app.current_view, ViewId::Projects);
    app.handle_key(key(crossterm::event::KeyCode::Up));
    assert_eq!(app.current_view, ViewId::Overview);
}

#[test]
fn sidebar_jk_change_view() {
    let mut app = App::new();
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(app.current_view, ViewId::Projects);
    app.handle_key(key(crossterm::event::KeyCode::Char('k')));
    assert_eq!(app.current_view, ViewId::Overview);
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(app.current_view, ViewId::Findings);
    app.handle_key(key(crossterm::event::KeyCode::Char('k')));
    assert_eq!(app.current_view, ViewId::Projects);
}

#[test]
fn focus_toggle_with_hjkl() {
    let mut app = App::new();
    assert_eq!(app.focus, Focus::Sidebar);

    app.handle_key(key(crossterm::event::KeyCode::Char('l')));
    assert_eq!(app.focus, Focus::Main);

    app.handle_key(key(crossterm::event::KeyCode::Char('h')));
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn focus_toggle_with_arrows() {
    let mut app = App::new();
    app.handle_key(key(crossterm::event::KeyCode::Right));
    assert_eq!(app.focus, Focus::Main);
    app.handle_key(key(crossterm::event::KeyCode::Left));
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn main_focus_returns_to_sidebar_on_esc() {
    let mut app = App::new();
    app.focus = Focus::Main;
    app.handle_key(key(crossterm::event::KeyCode::Esc));
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn sidebar_enter_activates_main() {
    let mut app = App::new();
    app.handle_key(key(crossterm::event::KeyCode::Enter));
    assert_eq!(app.focus, Focus::Main);
}

#[test]
fn projects_rows_move_only_when_focus_main() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.sniff_state.projects = vec![
        sniff::ProjectRow {
            name: "a".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
        sniff::ProjectRow {
            name: "b".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
    ];
    app.sniff_state.table_state.select(Some(0));

    // Sidebar focus: j/k move views, not rows.
    app.focus = Focus::Sidebar;
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(
        app.current_view,
        ViewId::Findings,
        "j in sidebar cycles views"
    );
    assert_eq!(
        app.sniff_state.selected_project().unwrap().name,
        "a",
        "sidebar j must not move the table selection"
    );

    // Main focus: j/k move rows, view unchanged.
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(
        app.current_view,
        ViewId::Projects,
        "row nav must not change view"
    );
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "b");
    app.handle_key(key(crossterm::event::KeyCode::Char('k')));
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "a");
}

#[test]
fn g_and_g_select_first_and_last_row() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;
    app.sniff_state.projects = vec![
        sniff::ProjectRow {
            name: "a".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
        sniff::ProjectRow {
            name: "b".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
        sniff::ProjectRow {
            name: "c".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
    ];
    app.sniff_state.table_state.select(Some(0));

    app.handle_key(key(crossterm::event::KeyCode::Char('G')));
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "c");
    app.handle_key(key(crossterm::event::KeyCode::Char('g')));
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "a");
}

#[test]
fn help_toggle() {
    let mut app = App::new();

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('?'))),
        Command::None
    );
    assert!(app.help_visible);

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('?'))),
        Command::None
    );
    assert!(!app.help_visible);
}

#[test]
fn esc_closes_help() {
    let mut app = App::new();
    app.help_visible = true;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Esc)),
        Command::None
    );
    assert!(!app.help_visible);
}

#[test]
fn q_quits() {
    let mut app = App::new();
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('q'))),
        Command::Quit
    );
    assert!(!app.running);
}

#[test]
fn navigation_blocked_when_help_open() {
    let mut app = App::new();
    app.help_visible = true;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('2'))),
        Command::None
    );
    assert_eq!(
        app.current_view,
        ViewId::Overview,
        "view must not change while help is visible"
    );
}

#[test]
fn q_blocked_when_help_open() {
    let mut app = App::new();
    app.help_visible = true;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('q'))),
        Command::None
    );
    assert!(app.running, "q must not quit while help is visible");
}

#[test]
fn irrelevant_keys_are_noop() {
    let mut app = App::new();
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('x'))),
        Command::None
    );
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('Z'))),
        Command::None
    );
    assert_eq!(app.current_view, ViewId::Overview);
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn sniff_keys_when_in_projects_view() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('r'))),
        Command::SniffRefresh
    );
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('o'))),
        Command::ChangeScanRoot
    );
}

#[test]
fn sniff_keys_ignored_in_other_views() {
    let mut app = App::new();
    app.current_view = ViewId::Overview;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('r'))),
        Command::None
    );
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('o'))),
        Command::None
    );
}

#[test]
fn enter_on_projects_digs_when_focus_main() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;
    app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
    app.sniff_state.table_state.select(Some(0));

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Enter)),
        Command::Dig
    );
    assert_eq!(
        app.current_view,
        ViewId::Projects,
        "dig does not switch views"
    );
}

#[test]
fn enter_on_projects_without_selected_row_is_noop() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;
    // No rows and no selection → nothing to dig.
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Enter)),
        Command::None
    );
}

#[test]
fn sidebar_enter_on_projects_moves_focus_not_dig() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Sidebar;
    app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
    app.sniff_state.table_state.select(Some(0));

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Enter)),
        Command::None
    );
    assert_eq!(app.focus, Focus::Main, "sidebar Enter still focuses main");
}

#[test]
fn esc_on_findings_returns_to_projects_and_clears_scope() {
    let mut app = App::new();
    app.current_view = ViewId::Findings;
    app.focus = Focus::Main;
    app.dig_state.project = Some(std::path::PathBuf::from("/tmp/a"));

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Esc)),
        Command::BackToProjects
    );
    assert_eq!(app.current_view, ViewId::Projects);
    assert_eq!(app.dig_state.project, None, "scope is cleared on leave");
}

#[test]
fn esc_on_projects_keeps_focus_to_sidebar() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Esc)),
        Command::None
    );
    assert_eq!(app.focus, Focus::Sidebar);
    assert_eq!(app.current_view, ViewId::Projects);
}

#[test]
fn findings_keys_map_to_commands() {
    let mut app = App::new();
    app.current_view = ViewId::Findings;

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('r'))),
        Command::DigRefresh
    );
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('c'))),
        Command::ToggleContentScan
    );
}

#[test]
fn f_on_findings_cycles_risk_filter() {
    let mut app = App::new();
    app.current_view = ViewId::Findings;
    assert_eq!(app.dig_state.min_risk, dig::RiskFilter::ShowAll);

    app.handle_key(key(crossterm::event::KeyCode::Char('f')));
    assert_eq!(app.dig_state.min_risk, dig::RiskFilter::OnlyCritical);
    app.handle_key(key(crossterm::event::KeyCode::Char('f')));
    app.handle_key(key(crossterm::event::KeyCode::Char('f')));
    assert_eq!(app.dig_state.min_risk, dig::RiskFilter::MediumAndAbove);
    app.handle_key(key(crossterm::event::KeyCode::Char('f')));
    assert_eq!(app.dig_state.min_risk, dig::RiskFilter::ShowAll);
}

#[test]
fn findings_keys_ignored_in_other_views() {
    let mut app = App::new();
    app.current_view = ViewId::Overview;
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('f'))),
        Command::None
    );
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('c'))),
        Command::None
    );
}

#[test]
fn findings_rows_move_only_when_focus_main() {
    let mut app = App::new();
    app.current_view = ViewId::Findings;
    app.focus = Focus::Main;
    app.dig_state.all_findings = vec![
        dig::FindingRow {
            path: std::path::PathBuf::from("/a"),
            risk: raccpack_core::domain::SensitiveRisk::Critical,
            kind: ".env".to_string(),
            git_status: "tracked".to_string(),
        },
        dig::FindingRow {
            path: std::path::PathBuf::from("/b"),
            risk: raccpack_core::domain::SensitiveRisk::High,
            kind: "key".to_string(),
            git_status: String::new(),
        },
    ];
    app.dig_state.reapply_filter();

    // Sidebar focus: j cycles views, table selection untouched.
    app.focus = Focus::Sidebar;
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(app.current_view, ViewId::Operations);
    assert_eq!(
        app.dig_state.selected_finding().unwrap().path,
        std::path::PathBuf::from("/a"),
        "sidebar j must not move the table selection"
    );

    // Main focus: j/k move rows, view unchanged.
    app.current_view = ViewId::Findings;
    app.focus = Focus::Main;
    app.handle_key(key(crossterm::event::KeyCode::Char('j')));
    assert_eq!(
        app.dig_state.selected_finding().unwrap().path,
        std::path::PathBuf::from("/b")
    );
    app.handle_key(key(crossterm::event::KeyCode::Char('k')));
    assert_eq!(
        app.dig_state.selected_finding().unwrap().path,
        std::path::PathBuf::from("/a")
    );
}

#[test]
fn r_uppercase_on_projects_with_selection_opens_raid_flow() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
    app.sniff_state.table_state.select(Some(0));

    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('R'))),
        Command::RaidPreview
    );
    assert_eq!(app.current_view, ViewId::Projects);
}

#[test]
fn r_uppercase_on_projects_without_selection_is_noop() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('R'))),
        Command::None
    );
}

#[test]
fn r_uppercase_ignored_in_other_views() {
    let mut app = App::new();
    app.current_view = ViewId::Findings;
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('R'))),
        Command::None
    );
    app.current_view = ViewId::Overview;
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('R'))),
        Command::None
    );
}

#[test]
fn lowercase_r_stays_sniff_refresh_on_projects() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    assert_eq!(
        app.handle_key(key(crossterm::event::KeyCode::Char('r'))),
        Command::SniffRefresh
    );
}

#[test]
fn v_cycles_projects_mode_in_projects_view() {
    use crate::app::sniff::ProjectsMode;

    let mut app = App::new();
    app.current_view = ViewId::Overview;
    // v outside Projects is inert.
    app.handle_key(key(crossterm::event::KeyCode::Char('v')));
    assert_eq!(app.sniff_state.mode, ProjectsMode::Cards);

    app.current_view = ViewId::Projects;
    assert_eq!(
        app.sniff_state.mode,
        ProjectsMode::Cards,
        "Cards is default"
    );
    app.handle_key(key(crossterm::event::KeyCode::Char('v')));
    assert_eq!(app.sniff_state.mode, ProjectsMode::Table);
    app.handle_key(key(crossterm::event::KeyCode::Char('v')));
    assert_eq!(app.sniff_state.mode, ProjectsMode::Tree);
    app.handle_key(key(crossterm::event::KeyCode::Char('v')));
    assert_eq!(app.sniff_state.mode, ProjectsMode::Cards);
}

#[test]
fn projects_mode_next_cycles_back_to_cards() {
    use crate::app::sniff::ProjectsMode;
    assert_eq!(ProjectsMode::Cards.next(), ProjectsMode::Table);
    assert_eq!(ProjectsMode::Table.next(), ProjectsMode::Tree);
    assert_eq!(ProjectsMode::Tree.next(), ProjectsMode::Cards);
}
