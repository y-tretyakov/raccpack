//! Sniff screen state selection tests.

use super::super::*;
use crate::app::sniff::SniffScreenState;

#[test]
fn sniff_screen_state_selection() {
    let mut state = SniffScreenState {
        projects: vec![
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
        ],
        table_state: ratatui::widgets::TableState::default(),
        ..Default::default()
    };
    state.table_state.select(Some(0));
    assert_eq!(state.selected_project().unwrap().name, "a");
    state.select_next();
    assert_eq!(state.selected_project().unwrap().name, "b");
    state.select_previous();
    assert_eq!(state.selected_project().unwrap().name, "a");
    state.select_first();
    assert_eq!(state.selected_project().unwrap().name, "a");
    state.select_last();
    assert_eq!(state.selected_project().unwrap().name, "b");
}
