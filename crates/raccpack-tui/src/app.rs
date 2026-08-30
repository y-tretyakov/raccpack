//! Application state, key mapping, and update logic.

use std::path::PathBuf;

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
}

pub mod activity;
pub mod activity_feed;
pub mod dig;
pub mod raid;
pub mod sniff;

mod keys;

#[cfg(test)]
mod tests;
