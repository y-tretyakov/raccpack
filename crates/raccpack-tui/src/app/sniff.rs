//! Sniff (Projects) screen state and rendering-mode enum.
//!
//! Owns the project rows surfaced by a scan and the Projects view's rendering
//! mode (Cards default; Table demoted; Tree stub until V2-T1).

use std::path::PathBuf;

use raccpack_core::app::ProgressEvent;
use ratatui::widgets::TableState;

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
