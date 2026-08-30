//! Key handling for [`super::App`] — all of `handle_key*` routing lives here.
//!
//! A child module of `app`, so it has access to `App`'s private fields and to
//! the sibling screen-state modules (`sniff`, `dig`, `raid`, `activity`).

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::raid::RaidCommand;
use crate::app::{Command, Focus, ViewId};

impl super::App {
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
            Some(RaidCommand::PreviewConfirm) | Some(RaidCommand::Run) => Some(Command::RaidRun),
            Some(RaidCommand::PreviewCancel) | Some(RaidCommand::PassphraseCancel) => {
                Some(Command::RaidCancel)
            }
            Some(RaidCommand::PassphraseConfirm(passphrase)) => {
                if let Some(flow) = self.raid_flow.as_mut() {
                    flow.store_confirmed(passphrase);
                }
                Some(Command::RaidRun)
            }
            Some(RaidCommand::Close) => {
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
