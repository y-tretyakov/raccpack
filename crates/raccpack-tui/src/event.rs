//! Terminal lifecycle (RAII guard) and crossterm → AppEvent bridge.

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{App, Command};

/// Events dispatched into the application loop.
#[derive(Debug)]
pub enum AppEvent {
    /// User pressed a key.
    Key(KeyEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Shutdown requested via Ctrl-C.
    Quit,
}

/// RAII guard that enters alternate screen + raw mode on creation and
/// restores the original terminal state on drop. A panic hook is installed
/// so the terminal is also restored on panic.
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        ratatui::Terminal::new(CrosstermBackend::new(stdout))?;

        // Restore terminal even on panic.
        std::panic::set_hook(Box::new(|info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            eprintln!("{info}");
        }));

        Ok(Self { _private: () })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Run the main event → update → render loop until the app quits.
pub fn run_event_loop(app: &mut App) -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // Dedicated reader thread — blocks on crossterm::event::read().
    std::thread::spawn(move || event_reader(tx));

    while app.running {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.running = false;
                } else {
                    let cmd = app.handle_key(key);
                    if matches!(cmd, Command::Quit) {
                        break;
                    }
                }
            }
            Ok(AppEvent::Resize(w, h)) => {
                let _ = terminal.resize(ratatui::layout::Rect::new(0, 0, w, h));
            }
            Ok(AppEvent::Quit) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        crate::ui::layout::render(app, &mut terminal)?;
    }

    Ok(())
}

/// Blocking reader that translates crossterm events into `AppEvent`s.
fn event_reader(tx: mpsc::Sender<AppEvent>) {
    loop {
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(w, h)) => {
                    let _ = tx.send(AppEvent::Resize(w, h));
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }
}
