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
    /// Open the operation selected on the Operations hub: routes to the raid
    /// flow for Raid, or to a stub notice for Pack/Stash/Rinse (the real flows
    /// are planned stages; no core work is dispatched for a stub).
    OpenOperation,
    /// Confirm the raid preview / run; the passphrase is resolved in event.rs.
    RaidRun,
    /// Cancel the raid flow (n / Esc while previewing or entering the passphrase).
    RaidCancel,
    /// Dispatch an ephemeral reveal to the worker (confirmed on the modal).
    Reveal,
}
