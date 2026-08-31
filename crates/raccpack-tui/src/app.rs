//! Application state, key mapping, and update logic.

use std::path::PathBuf;

use crossterm::event::KeyEvent;

pub use nav::{Command, Focus, ViewId, ALL_VIEWS};
pub use operations::{OperationKind, OperationsScreenState};
pub use sniff::{ProjectRow, SniffScreenState};

pub mod dig;
pub mod handlers;
pub mod nav;
pub mod operations;
pub mod raid;
pub mod reveal;
pub mod sniff;

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
    /// State for the Operations hub screen.
    pub operations_state: operations::OperationsScreenState,
    /// Resolved den directory (flag > env > default `~/.raccpack/den`).
    pub den_dir: PathBuf,
    /// Whether to run a sniff refresh automatically once the loop starts.
    pub refresh_on_start: bool,
    /// Active raid modal flow, if any.
    pub raid_flow: Option<raid::RaidFlow>,
    /// Active reveal modal (opt-in, ephemeral), if any.
    pub reveal: Option<reveal::RevealModal>,
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
            operations_state: operations::OperationsScreenState::default(),
            den_dir: PathBuf::new(),
            refresh_on_start: false,
            raid_flow: None,
            reveal: None,
        }
    }

    /// Process a terminal key event and return the resulting command.
    pub fn handle_key(&mut self, key: KeyEvent) -> Command {
        // A raid or reveal modal takes precedence over the help overlay: once
        // a flow/modal is active, help cannot be shown on top of it and
        // `Esc`/`?` dismiss only what the modal itself allows.
        if self.raid_flow.is_some() || self.reveal.is_some() {
            self.help_visible = false;
        }

        if self.help_visible {
            return self.handle_key_help(key);
        }

        if let Some(cmd) = self.handle_key_reveal(key) {
            return cmd;
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
}

#[cfg(test)]
mod tests;
