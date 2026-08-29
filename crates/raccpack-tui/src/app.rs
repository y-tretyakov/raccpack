//! Application state, key mapping, and update logic.

use crossterm::event::{KeyCode, KeyEvent};

/// Active view in the main area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    Overview,
    Projects,
    Findings,
    Operations,
}

impl ViewId {
    /// Display name for the sidebar label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Projects => "Projects",
            Self::Findings => "Findings",
            Self::Operations => "Operations",
        }
    }

    /// Sidebar shortcut key digit.
    pub fn key(self) -> char {
        match self {
            Self::Overview => '1',
            Self::Projects => '2',
            Self::Findings => '3',
            Self::Operations => '4',
        }
    }

    /// Next view in cycle order.
    pub fn next(self) -> Self {
        match self {
            Self::Overview => Self::Projects,
            Self::Projects => Self::Findings,
            Self::Findings => Self::Operations,
            Self::Operations => Self::Overview,
        }
    }
}

/// Commands emitted by the update step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// No action required.
    None,
    /// Shutdown requested.
    Quit,
    /// Trigger sniff operation.
    Sniff,
    /// Trigger sniff with force refresh.
    SniffRefresh,
    /// Change scan root.
    ChangeScanRoot,
}

/// Top-level application state.
#[derive(Debug)]
pub struct App {
    pub current_view: ViewId,
    pub help_visible: bool,
    pub running: bool,
    /// State for the sniff screen.
    pub sniff_state: sniff::SniffScreenState,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application in its initial state.
    pub fn new() -> Self {
        Self {
            current_view: ViewId::Overview,
            help_visible: false,
            running: true,
            sniff_state: sniff::SniffScreenState::default(),
        }
    }

    /// Process a terminal key event and return the resulting command.
    pub fn handle_key(&mut self, key: KeyEvent) -> Command {
        if self.help_visible {
            match key.code {
                KeyCode::Esc => {
                    self.help_visible = false;
                    Command::None
                }
                KeyCode::Char('?') => {
                    self.help_visible = false;
                    Command::None
                }
                _ => Command::None,
            }
        } else {
            match key.code {
                KeyCode::Char('q') => {
                    self.running = false;
                    Command::Quit
                }
                KeyCode::Char('1') => {
                    self.current_view = ViewId::Overview;
                    Command::None
                }
                KeyCode::Char('2') => {
                    self.current_view = ViewId::Projects;
                    Command::None
                }
                KeyCode::Char('3') => {
                    self.current_view = ViewId::Findings;
                    Command::None
                }
                KeyCode::Char('4') => {
                    self.current_view = ViewId::Operations;
                    Command::None
                }
                KeyCode::Char('?') => {
                    self.help_visible = true;
                    Command::None
                }
                KeyCode::Char('r') if self.current_view == ViewId::Projects => {
                    Command::SniffRefresh
                }
                KeyCode::Char('o') if self.current_view == ViewId::Projects => {
                    Command::ChangeScanRoot
                }
                KeyCode::Char('j') | KeyCode::Down if self.current_view == ViewId::Projects => {
                    self.sniff_state.select_next();
                    Command::None
                }
                KeyCode::Char('k') | KeyCode::Up if self.current_view == ViewId::Projects => {
                    self.sniff_state.select_previous();
                    Command::None
                }
                KeyCode::Enter if self.current_view == ViewId::Projects => {
                    // Open dig for selected project (placeholder for now)
                    Command::None
                }
                _ => Command::None,
            }
        }
    }
}

pub mod sniff {
    use ratatui::widgets::TableState;
    use std::path::PathBuf;

    use raccpack_core::app::ProgressEvent;

    /// A single row in the sniff project table.
    #[derive(Debug, Clone)]
    pub struct ProjectRow {
        pub name: String,
        pub language: Option<String>,
        pub frameworks: Vec<String>,
        pub size_bytes: u64,
        pub is_git_repo: bool,
        pub path: PathBuf,
    }

    /// State for the sniff screen.
    #[derive(Debug, Default)]
    pub struct SniffScreenState {
        pub projects: Vec<ProjectRow>,
        pub total_size: u64,
        pub scan_root: PathBuf,
        pub is_loading: bool,
        pub error: Option<String>,
        pub last_refresh: Option<std::time::SystemTime>,
        pub from_cache: bool,
        pub table_state: TableState,
        pub progress: Option<ProgressEvent>,
    }

    impl SniffScreenState {
        pub fn select_next(&mut self) {
            let i = self.table_state.selected().unwrap_or(0);
            if i + 1 < self.projects.len() {
                self.table_state.select(Some(i + 1));
            }
        }

        pub fn select_previous(&mut self) {
            let i = self.table_state.selected().unwrap_or(0);
            if i > 0 {
                self.table_state.select(Some(i - 1));
            }
        }

        pub fn selected_project(&self) -> Option<&ProjectRow> {
            self.table_state
                .selected()
                .and_then(|i| self.projects.get(i))
        }

        pub fn set_loading(&mut self, loading: bool) {
            self.is_loading = loading;
            if loading {
                self.error = None;
            }
        }

        pub fn clear(&mut self) {
            self.projects.clear();
            self.total_size = 0;
            self.scan_root = PathBuf::new();
            self.is_loading = false;
            self.error = None;
            self.last_refresh = None;
            self.from_cache = false;
            self.table_state = TableState::default();
            self.progress = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::sniff::SniffScreenState;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn initial_state() {
        let app = App::new();
        assert_eq!(app.current_view, ViewId::Overview);
        assert!(!app.help_visible);
        assert!(app.running);
    }

    #[test]
    fn navigation_switches_views() {
        let mut app = App::new();

        assert_eq!(app.handle_key(key(KeyCode::Char('2'))), Command::None);
        assert_eq!(app.current_view, ViewId::Projects);

        assert_eq!(app.handle_key(key(KeyCode::Char('3'))), Command::None);
        assert_eq!(app.current_view, ViewId::Findings);

        assert_eq!(app.handle_key(key(KeyCode::Char('4'))), Command::None);
        assert_eq!(app.current_view, ViewId::Operations);

        assert_eq!(app.handle_key(key(KeyCode::Char('1'))), Command::None);
        assert_eq!(app.current_view, ViewId::Overview);
    }

    #[test]
    fn help_toggle() {
        let mut app = App::new();

        assert_eq!(app.handle_key(key(KeyCode::Char('?'))), Command::None);
        assert!(app.help_visible);

        assert_eq!(app.handle_key(key(KeyCode::Char('?'))), Command::None);
        assert!(!app.help_visible);
    }

    #[test]
    fn esc_closes_help() {
        let mut app = App::new();
        app.help_visible = true;

        assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::None);
        assert!(!app.help_visible);
    }

    #[test]
    fn q_quits() {
        let mut app = App::new();
        assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Command::Quit);
        assert!(!app.running);
    }

    #[test]
    fn navigation_blocked_when_help_open() {
        let mut app = App::new();
        app.help_visible = true;

        assert_eq!(app.handle_key(key(KeyCode::Char('2'))), Command::None);
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

        assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Command::None);
        assert!(app.running, "q must not quit while help is visible");
    }

    #[test]
    fn view_id_next_cycles() {
        assert_eq!(ViewId::Overview.next(), ViewId::Projects);
        assert_eq!(ViewId::Projects.next(), ViewId::Findings);
        assert_eq!(ViewId::Findings.next(), ViewId::Operations);
        assert_eq!(ViewId::Operations.next(), ViewId::Overview);
    }

    #[test]
    fn view_id_labels() {
        assert_eq!(ViewId::Overview.label(), "Overview");
        assert_eq!(ViewId::Projects.label(), "Projects");
        assert_eq!(ViewId::Findings.label(), "Findings");
        assert_eq!(ViewId::Operations.label(), "Operations");
    }

    #[test]
    fn view_id_keys() {
        assert_eq!(ViewId::Overview.key(), '1');
        assert_eq!(ViewId::Projects.key(), '2');
        assert_eq!(ViewId::Findings.key(), '3');
        assert_eq!(ViewId::Operations.key(), '4');
    }

    #[test]
    fn irrelevant_keys_are_noop() {
        let mut app = App::new();
        assert_eq!(app.handle_key(key(KeyCode::Char('x'))), Command::None);
        assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::None);
        assert_eq!(app.handle_key(key(KeyCode::Up)), Command::None);
    }

    #[test]
    fn sniff_keys_when_in_projects_view() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;

        assert_eq!(
            app.handle_key(key(KeyCode::Char('r'))),
            Command::SniffRefresh
        );
        assert_eq!(
            app.handle_key(key(KeyCode::Char('o'))),
            Command::ChangeScanRoot
        );
    }

    #[test]
    fn sniff_keys_ignored_in_other_views() {
        let mut app = App::new();
        app.current_view = ViewId::Overview;

        assert_eq!(app.handle_key(key(KeyCode::Char('r'))), Command::None);
        assert_eq!(app.handle_key(key(KeyCode::Char('o'))), Command::None);
    }

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
    }
}
