//! Application state, key mapping, and update logic.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

/// Which region of the UI currently owns list/arrow keys.
///
/// * `Sidebar` — `j`/`k`/arrows move between views.
/// * `Main` — `j`/`k`/arrows move the active screen's list (project table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Sidebar,
    Main,
}

impl Focus {
    /// Lowercase label used in the footer hint.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Main => "main",
        }
    }
}

/// Active view in the main area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    Overview,
    Projects,
    Findings,
    Operations,
}

/// Ordered list of every view, in cycle order.
pub const ALL_VIEWS: [ViewId; 4] = [
    ViewId::Overview,
    ViewId::Projects,
    ViewId::Findings,
    ViewId::Operations,
];

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

    /// Previous view in cycle order.
    pub fn prev(self) -> Self {
        match self {
            Self::Overview => Self::Operations,
            Self::Projects => Self::Overview,
            Self::Findings => Self::Projects,
            Self::Operations => Self::Findings,
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
    /// Dig the project selected on the Projects screen.
    Dig,
    /// Re-run dig for the project currently on the Findings screen.
    DigRefresh,
    /// Toggle content scanning and re-run dig for the current project.
    ToggleContentScan,
    /// Advance the min-risk filter (handled purely in-app).
    CycleRiskFilter,
    /// Return from the Findings screen to the Projects screen.
    BackToProjects,
    /// Open the raid flow (preview) for the selected project.
    RaidPreview,
    /// Confirm the raid preview / run; the passphrase is resolved in event.rs.
    RaidRun,
    /// Cancel the raid flow (n / Esc while previewing or entering the passphrase).
    RaidCancel,
}

/// Top-level application state.
#[derive(Debug)]
pub struct App {
    pub current_view: ViewId,
    pub focus: Focus,
    pub help_visible: bool,
    pub running: bool,
    /// State for the sniff screen.
    pub sniff_state: sniff::SniffScreenState,
    /// State for the dig screen.
    pub dig_state: dig::DigScreenState,
    /// Resolved den directory (flag > env > default `~/.raccpack/den`).
    pub den_dir: PathBuf,
    /// Whether to run a sniff refresh automatically once the loop starts.
    pub refresh_on_start: bool,
    /// Active raid modal flow, if any.
    pub raid_flow: Option<raid::RaidFlow>,
    /// User-meaningful semantic event stream (Activity panel).
    pub activity: activity::ActivityLog,
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
            // Start focused on the sidebar so `j`/`k`/arrows work immediately.
            current_view: ViewId::Overview,
            focus: Focus::Sidebar,
            help_visible: false,
            running: true,
            sniff_state: sniff::SniffScreenState::default(),
            dig_state: dig::DigScreenState::default(),
            den_dir: PathBuf::new(),
            refresh_on_start: false,
            raid_flow: None,
            activity: activity::ActivityLog::default(),
        }
    }

    /// Process a terminal key event and return the resulting command.
    pub fn handle_key(&mut self, key: KeyEvent) -> Command {
        // A raid modal takes precedence over the help overlay: once a flow is
        // active the help cannot be shown on top of it and `Esc`/`?` dismiss
        // only what the flow itself allows.
        if self.raid_flow.is_some() {
            self.help_visible = false;
        }

        if self.help_visible {
            return self.handle_key_help(key);
        }

        if let Some(cmd) = self.handle_key_raid_flow(key) {
            return cmd;
        }

        if let Some(cmd) = self.handle_key_content(key) {
            return cmd;
        }

        match self.focus {
            Focus::Sidebar => self.handle_key_sidebar(key),
            Focus::Main => self.handle_key_main(key),
        }
    }

    /// Keys while the raid modal is open: everything the flow does not consume
    /// is swallowed, so no key reaches the underlying screens. Returns `None`
    /// when no flow is active (keys fall through normally).
    fn handle_key_raid_flow(&mut self, key: KeyEvent) -> Option<Command> {
        let raid_cmd = self.raid_flow.as_mut()?.handle_key(key.code);
        match raid_cmd {
            None => Some(Command::None),
            Some(raid::RaidCommand::PreviewConfirm) | Some(raid::RaidCommand::Run) => {
                Some(Command::RaidRun)
            }
            Some(raid::RaidCommand::PreviewCancel) | Some(raid::RaidCommand::PassphraseCancel) => {
                Some(Command::RaidCancel)
            }
            Some(raid::RaidCommand::PassphraseConfirm(passphrase)) => {
                if let Some(flow) = self.raid_flow.as_mut() {
                    flow.store_confirmed(passphrase);
                }
                Some(Command::RaidRun)
            }
            Some(raid::RaidCommand::Close) => {
                self.raid_flow = None;
                Some(Command::None)
            }
        }
    }

    /// Keys while the help overlay is open: only `?` / `Esc` close it.
    fn handle_key_help(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                self.help_visible = false;
                Command::None
            }
            _ => Command::None,
        }
    }

    /// View-scoped content actions that work regardless of focus
    /// (while help is closed). Only surfaced when the current view owns them.
    fn handle_key_content(&mut self, key: KeyEvent) -> Option<Command> {
        if self.current_view == ViewId::Projects {
            return match key.code {
                KeyCode::Char('r') => Some(Command::SniffRefresh),
                KeyCode::Char('o') => Some(Command::ChangeScanRoot),
                // Uppercase `R` opens the raid flow for the selected row.
                KeyCode::Char('R') => {
                    if self.sniff_state.selected_project().is_some() {
                        Some(Command::RaidPreview)
                    } else {
                        Some(Command::None)
                    }
                }
                // Events go to the project that is selected in the table.
                KeyCode::Enter if self.focus == Focus::Main => {
                    if self.sniff_state.selected_project().is_some() {
                        Some(Command::Dig)
                    } else {
                        Some(Command::None)
                    }
                }
                // `v` cycles the Projects rendering mode: Cards → Table → Tree.
                KeyCode::Char('v') => {
                    self.sniff_state.mode = self.sniff_state.mode.next();
                    Some(Command::None)
                }
                // Sidebar Enter still means "move focus to main", not dig.
                KeyCode::Enter => None,
                _ => None,
            };
        }
        if self.current_view == ViewId::Findings {
            return match key.code {
                // `r` re-digs only when a project is scoped (guard in event.rs).
                KeyCode::Char('r') => Some(Command::DigRefresh),
                KeyCode::Char('c') => Some(Command::ToggleContentScan),
                // `f` is purely local state, no worker round-trip needed.
                KeyCode::Char('f') => {
                    self.dig_state.cycle_min_risk();
                    Some(Command::None)
                }
                _ => None,
            };
        }
        None
    }

    /// Keys while the sidebar owns list/arrow navigation.
    fn handle_key_sidebar(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.current_view = self.current_view.next();
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.current_view = self.current_view.prev();
                Command::None
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                self.focus = Focus::Main;
                Command::None
            }
            KeyCode::Char('h') | KeyCode::Left => Command::None,
            KeyCode::Tab => {
                self.current_view = self.current_view.next();
                Command::None
            }
            KeyCode::BackTab => {
                self.current_view = self.current_view.prev();
                Command::None
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
            KeyCode::Char('q') => {
                self.running = false;
                Command::Quit
            }
            KeyCode::Char('?') => {
                self.help_visible = true;
                Command::None
            }
            _ => Command::None,
        }
    }

    /// Keys while the main area owns list/arrow navigation.
    fn handle_key_main(&mut self, key: KeyEvent) -> Command {
        match key.code {
            // Esc on Findings returns to the project list (dig scope closes);
            // elsewhere it returns to the sidebar.
            KeyCode::Esc if self.current_view == ViewId::Findings => {
                self.current_view = ViewId::Projects;
                self.dig_state.leave();
                Command::BackToProjects
            }
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => {
                self.focus = Focus::Sidebar;
                Command::None
            }
            KeyCode::Char('l') | KeyCode::Right => Command::None,
            KeyCode::Char('j') | KeyCode::Down if self.current_view == ViewId::Projects => {
                self.sniff_state.select_next();
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up if self.current_view == ViewId::Projects => {
                self.sniff_state.select_previous();
                Command::None
            }
            KeyCode::Char('g') if self.current_view == ViewId::Projects => {
                self.sniff_state.select_first();
                Command::None
            }
            KeyCode::Char('G') if self.current_view == ViewId::Projects => {
                self.sniff_state.select_last();
                Command::None
            }
            KeyCode::Char('j') | KeyCode::Down if self.current_view == ViewId::Findings => {
                self.dig_state.select_next();
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up if self.current_view == ViewId::Findings => {
                self.dig_state.select_previous();
                Command::None
            }
            KeyCode::Char('g') if self.current_view == ViewId::Findings => {
                self.dig_state.select_first();
                Command::None
            }
            KeyCode::Char('G') if self.current_view == ViewId::Findings => {
                self.dig_state.select_last();
                Command::None
            }
            KeyCode::Tab => {
                self.current_view = self.current_view.next();
                Command::None
            }
            KeyCode::BackTab => {
                self.current_view = self.current_view.prev();
                Command::None
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
            KeyCode::Char('q') => {
                self.running = false;
                Command::Quit
            }
            KeyCode::Char('?') => {
                self.help_visible = true;
                Command::None
            }
            _ => Command::None,
        }
    }
}

pub mod activity;
pub mod activity_feed;
pub mod dig;
pub mod raid;

pub mod sniff {
    use ratatui::widgets::TableState;
    use std::path::PathBuf;

    use raccpack_core::app::ProgressEvent;

    /// Rendering mode for the Projects screen. Cards is the default; Table is
    /// the historical demoted view; Tree is a stub until V2-T1 lands.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ProjectsMode {
        #[default]
        Cards,
        Table,
        Tree,
    }

    impl ProjectsMode {
        /// Next mode in the `v` cycle: Cards → Table → Tree → Cards.
        pub fn next(self) -> Self {
            match self {
                Self::Cards => Self::Table,
                Self::Table => Self::Tree,
                Self::Tree => Self::Cards,
            }
        }

        /// Lowercase label for titles / reports.
        pub fn label(self) -> &'static str {
            match self {
                Self::Cards => "cards",
                Self::Table => "table",
                Self::Tree => "tree",
            }
        }
    }

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
        /// Active rendering mode (Cards default; toggled with `v`).
        pub mode: ProjectsMode,
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

        pub fn select_first(&mut self) {
            if !self.projects.is_empty() {
                self.table_state.select(Some(0));
            }
        }

        pub fn select_last(&mut self) {
            let n = self.projects.len();
            if n > 0 {
                self.table_state.select(Some(n - 1));
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

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
    fn tab_cycles_views_forward() {
        let mut app = App::new();
        let expected = [
            ViewId::Projects,
            ViewId::Findings,
            ViewId::Operations,
            ViewId::Overview,
        ];
        for &view in &expected {
            app.handle_key(key(KeyCode::Tab));
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
            app.handle_key(key(KeyCode::BackTab));
            assert_eq!(app.current_view, view);
        }
    }

    #[test]
    fn sidebar_arrows_change_view() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.current_view, ViewId::Projects);
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.current_view, ViewId::Overview);
    }

    #[test]
    fn sidebar_jk_change_view() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.current_view, ViewId::Projects);
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.current_view, ViewId::Overview);
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.current_view, ViewId::Findings);
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.current_view, ViewId::Projects);
    }

    #[test]
    fn focus_toggle_with_hjkl() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Sidebar);

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.focus, Focus::Main);

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.focus, Focus::Sidebar);
    }

    #[test]
    fn focus_toggle_with_arrows() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.focus, Focus::Main);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.focus, Focus::Sidebar);
    }

    #[test]
    fn main_focus_returns_to_sidebar_on_esc() {
        let mut app = App::new();
        app.focus = Focus::Main;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.focus, Focus::Sidebar);
    }

    #[test]
    fn sidebar_enter_activates_main() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Enter));
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
        app.handle_key(key(KeyCode::Char('j')));
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
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.current_view,
            ViewId::Projects,
            "row nav must not change view"
        );
        assert_eq!(app.sniff_state.selected_project().unwrap().name, "b");
        app.handle_key(key(KeyCode::Char('k')));
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

        app.handle_key(key(KeyCode::Char('G')));
        assert_eq!(app.sniff_state.selected_project().unwrap().name, "c");
        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.sniff_state.selected_project().unwrap().name, "a");
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
    fn view_id_prev_round_trips_next() {
        let views = [
            ViewId::Overview,
            ViewId::Projects,
            ViewId::Findings,
            ViewId::Operations,
        ];
        for &view in &views {
            assert_eq!(view.next().prev(), view, "next().prev() must round-trip");
            assert_eq!(view.prev().next(), view, "prev().next() must round-trip");
        }
    }

    #[test]
    fn view_id_prev_cycles() {
        assert_eq!(ViewId::Overview.prev(), ViewId::Operations);
        assert_eq!(ViewId::Projects.prev(), ViewId::Overview);
        assert_eq!(ViewId::Findings.prev(), ViewId::Projects);
        assert_eq!(ViewId::Operations.prev(), ViewId::Findings);
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
        assert_eq!(app.handle_key(key(KeyCode::Char('Z'))), Command::None);
        assert_eq!(app.current_view, ViewId::Overview);
        assert_eq!(app.focus, Focus::Sidebar);
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
        state.select_first();
        assert_eq!(state.selected_project().unwrap().name, "a");
        state.select_last();
        assert_eq!(state.selected_project().unwrap().name, "b");
    }

    fn project_row(name: &str, path: &str) -> sniff::ProjectRow {
        sniff::ProjectRow {
            name: name.to_string(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::from(path),
        }
    }

    #[test]
    fn enter_on_projects_digs_when_focus_main() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        app.focus = Focus::Main;
        app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
        app.sniff_state.table_state.select(Some(0));

        assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::Dig);
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
        assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::None);
    }

    #[test]
    fn sidebar_enter_on_projects_moves_focus_not_dig() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        app.focus = Focus::Sidebar;
        app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
        app.sniff_state.table_state.select(Some(0));

        assert_eq!(app.handle_key(key(KeyCode::Enter)), Command::None);
        assert_eq!(app.focus, Focus::Main, "sidebar Enter still focuses main");
    }

    #[test]
    fn esc_on_findings_returns_to_projects_and_clears_scope() {
        let mut app = App::new();
        app.current_view = ViewId::Findings;
        app.focus = Focus::Main;
        app.dig_state.project = Some(std::path::PathBuf::from("/tmp/a"));

        assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::BackToProjects);
        assert_eq!(app.current_view, ViewId::Projects);
        assert_eq!(app.dig_state.project, None, "scope is cleared on leave");
    }

    #[test]
    fn esc_on_projects_keeps_focus_to_sidebar() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        app.focus = Focus::Main;

        assert_eq!(app.handle_key(key(KeyCode::Esc)), Command::None);
        assert_eq!(app.focus, Focus::Sidebar);
        assert_eq!(app.current_view, ViewId::Projects);
    }

    #[test]
    fn findings_keys_map_to_commands() {
        let mut app = App::new();
        app.current_view = ViewId::Findings;

        assert_eq!(app.handle_key(key(KeyCode::Char('r'))), Command::DigRefresh);
        assert_eq!(
            app.handle_key(key(KeyCode::Char('c'))),
            Command::ToggleContentScan
        );
    }

    #[test]
    fn f_on_findings_cycles_risk_filter() {
        let mut app = App::new();
        app.current_view = ViewId::Findings;
        assert_eq!(app.dig_state.min_risk, dig::RiskFilter::ShowAll);

        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.dig_state.min_risk, dig::RiskFilter::OnlyCritical);
        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.dig_state.min_risk, dig::RiskFilter::MediumAndAbove);
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.dig_state.min_risk, dig::RiskFilter::ShowAll);
    }

    #[test]
    fn findings_keys_ignored_in_other_views() {
        let mut app = App::new();
        app.current_view = ViewId::Overview;
        assert_eq!(app.handle_key(key(KeyCode::Char('f'))), Command::None);
        assert_eq!(app.handle_key(key(KeyCode::Char('c'))), Command::None);
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
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.current_view, ViewId::Operations);
        assert_eq!(
            app.dig_state.selected_finding().unwrap().path,
            std::path::PathBuf::from("/a"),
            "sidebar j must not move the table selection"
        );

        // Main focus: j/k move rows, view unchanged.
        app.current_view = ViewId::Findings;
        app.focus = Focus::Main;
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.dig_state.selected_finding().unwrap().path,
            std::path::PathBuf::from("/b")
        );
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(
            app.dig_state.selected_finding().unwrap().path,
            std::path::PathBuf::from("/a")
        );
    }

    fn preview_result() -> raccpack_core::app::RaidResult {
        raccpack_core::app::RaidResult {
            project_path: std::path::PathBuf::from("/tmp/a"),
            stages: Vec::new(),
            stash: None,
            rinse: None,
            pack: None,
            den_artifacts: Vec::new(),
            success: true,
            dry_run: true,
            rolled_back: false,
            rollback_warnings: Vec::new(),
        }
    }

    fn raid_flow_in(phase: raid::FlowPhase) -> raid::RaidFlow {
        let mut flow = raid::RaidFlow::new(
            std::path::PathBuf::from("/tmp/a"),
            std::path::PathBuf::from("/tmp/den"),
            raid::RaidFlowOptions::default(),
        );
        flow.phase = phase;
        flow
    }

    #[test]
    fn r_uppercase_on_projects_with_selection_opens_raid_flow() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        app.sniff_state.projects = vec![project_row("a", "/tmp/a")];
        app.sniff_state.table_state.select(Some(0));

        assert_eq!(
            app.handle_key(key(KeyCode::Char('R'))),
            Command::RaidPreview
        );
        assert_eq!(app.current_view, ViewId::Projects);
    }

    #[test]
    fn r_uppercase_on_projects_without_selection_is_noop() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        assert_eq!(app.handle_key(key(KeyCode::Char('R'))), Command::None);
    }

    #[test]
    fn r_uppercase_ignored_in_other_views() {
        let mut app = App::new();
        app.current_view = ViewId::Findings;
        assert_eq!(app.handle_key(key(KeyCode::Char('R'))), Command::None);
        app.current_view = ViewId::Overview;
        assert_eq!(app.handle_key(key(KeyCode::Char('R'))), Command::None);
    }

    #[test]
    fn lowercase_r_stays_sniff_refresh_on_projects() {
        let mut app = App::new();
        app.current_view = ViewId::Projects;
        assert_eq!(
            app.handle_key(key(KeyCode::Char('r'))),
            Command::SniffRefresh
        );
    }

    #[test]
    fn v_cycles_projects_mode_in_projects_view() {
        use crate::app::sniff::ProjectsMode;

        let mut app = App::new();
        app.current_view = ViewId::Overview;
        // v outside Projects is inert.
        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.sniff_state.mode, ProjectsMode::Cards);

        app.current_view = ViewId::Projects;
        assert_eq!(
            app.sniff_state.mode,
            ProjectsMode::Cards,
            "Cards is default"
        );
        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.sniff_state.mode, ProjectsMode::Table);
        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.sniff_state.mode, ProjectsMode::Tree);
        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.sniff_state.mode, ProjectsMode::Cards);
    }

    #[test]
    fn projects_mode_next_cycles_back_to_cards() {
        use crate::app::sniff::ProjectsMode;
        assert_eq!(ProjectsMode::Cards.next(), ProjectsMode::Table);
        assert_eq!(ProjectsMode::Table.next(), ProjectsMode::Tree);
        assert_eq!(ProjectsMode::Tree.next(), ProjectsMode::Cards);
    }

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
}
