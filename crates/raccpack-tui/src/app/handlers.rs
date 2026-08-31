use crossterm::event::{KeyCode, KeyEvent};

use crate::app::operations::OperationKind;
use crate::app::pack;
use crate::app::raid;
use crate::app::reveal;
use crate::app::App;
use crate::app::{Command, Focus, ViewId};

impl App {
    /// Keys while the reveal modal is open. Everything the modal does not
    /// consume is swallowed, so no key reaches the underlying screens. Returns
    /// `None` when no modal is active (keys fall through normally).
    pub(crate) fn handle_key_reveal(&mut self, key: KeyEvent) -> Option<Command> {
        let modal = self.reveal.as_mut()?;
        match modal.handle_key(key.code) {
            reveal::RevealCommand::Confirm => Some(Command::Reveal),
            reveal::RevealCommand::Close => {
                // Dropping the modal zeroizes any in-flight secret.
                self.reveal = None;
                Some(Command::None)
            }
            reveal::RevealCommand::None => Some(Command::None),
        }
    }

    /// Keys while the raid modal is open: everything the flow does not consume
    /// is swallowed, so no key reaches the underlying screens. Returns `None`
    /// when no flow is active (keys fall through normally).
    pub(crate) fn handle_key_raid_flow(&mut self, key: KeyEvent) -> Option<Command> {
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

    /// Keys while the pack modal is open: everything the flow does not consume
    /// is swallowed. Returns `None` when no flow is active.
    pub(crate) fn handle_key_pack_flow(&mut self, key: KeyEvent) -> Option<Command> {
        let cmd = self.pack_flow.as_mut()?.handle_key(key.code)?;
        match cmd {
            pack::PackCommand::PreviewConfirm => Some(Command::PackRun),
            pack::PackCommand::PreviewCancel | pack::PackCommand::Close => {
                self.pack_flow = None;
                Some(Command::None)
            }
            pack::PackCommand::Run => Some(Command::PackRun),
        }
    }

    /// Keys while the help overlay is open: only `?` / `Esc` close it.
    pub(crate) fn handle_key_help(&mut self, key: KeyEvent) -> Command {
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
    pub(crate) fn handle_key_content(&mut self, key: KeyEvent) -> Option<Command> {
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
                // `v` opens the confirm step for an opt-in reveal, only when the
                // selected row carries a safe content reference.
                KeyCode::Char('v') => {
                    if let Some(row) = self.dig_state.selected_finding() {
                        if let Some(reference) = row.content_ref.clone() {
                            self.reveal =
                                Some(reveal::RevealModal::new(row.path.clone(), reference));
                        }
                    }
                    Some(Command::None)
                }
                _ => None,
            };
        }
        None
    }

    /// Keys while the sidebar owns list/arrow navigation.
    pub(crate) fn handle_key_sidebar(&mut self, key: KeyEvent) -> Command {
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
    pub(crate) fn handle_key_main(&mut self, key: KeyEvent) -> Command {
        // A stub notice (Pack/Stash/Rinse placeholder) blocks the Operations
        // screen until dismissed, mirroring how the raid flow and reveal modal
        // own their keys. Esc / Enter dismiss; everything else is swallowed.
        if self.current_view == ViewId::Operations && self.operations_state.stub.is_some() {
            return match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.operations_state.stub = None;
                    Command::None
                }
                _ => Command::None,
            };
        }

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
            KeyCode::Char('j') | KeyCode::Down if self.current_view == ViewId::Operations => {
                self.operations_state.select_next();
                Command::None
            }
            KeyCode::Char('k') | KeyCode::Up if self.current_view == ViewId::Operations => {
                self.operations_state.select_previous();
                Command::None
            }
            KeyCode::Char('g') if self.current_view == ViewId::Operations => {
                self.operations_state.select_first();
                Command::None
            }
            KeyCode::Char('G') if self.current_view == ViewId::Operations => {
                self.operations_state.select_last();
                Command::None
            }
            // Activate the highlighted operation; requires a sniff-selected
            // project (routing happens in event.rs).
            KeyCode::Enter if self.current_view == ViewId::Operations => {
                if self.sniff_state.selected_project().is_some() {
                    Command::OpenOperation
                } else {
                    Command::None
                }
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
            // Operation shortcut keys (`p`/`s`/`r`/`d`) jump the selection;
            // any other char on Operations stays a no-op. Quit and help are
            // handled by the arms above, so shortcuts never shadow them.
            KeyCode::Char(c) if self.current_view == ViewId::Operations => {
                if let Some(kind) = OperationKind::from_key(c) {
                    self.operations_state.selected = kind;
                }
                Command::None
            }
            _ => Command::None,
        }
    }
}
