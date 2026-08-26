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
}

/// Top-level application state.
#[derive(Debug)]
pub struct App {
    pub current_view: ViewId,
    pub help_visible: bool,
    pub running: bool,
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
        }
    }

    /// Process a terminal key event and return the resulting command.
    pub fn handle_key(&mut self, key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Char('q') if !self.help_visible => {
                self.running = false;
                Command::Quit
            }
            KeyCode::Char('1') if !self.help_visible => {
                self.current_view = ViewId::Overview;
                Command::None
            }
            KeyCode::Char('2') if !self.help_visible => {
                self.current_view = ViewId::Projects;
                Command::None
            }
            KeyCode::Char('3') if !self.help_visible => {
                self.current_view = ViewId::Findings;
                Command::None
            }
            KeyCode::Char('4') if !self.help_visible => {
                self.current_view = ViewId::Operations;
                Command::None
            }
            KeyCode::Char('?') => {
                self.help_visible = !self.help_visible;
                Command::None
            }
            KeyCode::Esc if self.help_visible => {
                self.help_visible = false;
                Command::None
            }
            _ => Command::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
