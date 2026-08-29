//! Terminal lifecycle (RAII guard) and crossterm → AppEvent bridge.

use std::io;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{App, Command};
use crate::worker::{WorkerEvent, WorkerMsg};

/// Events dispatched into the application loop.
#[derive(Debug)]
pub enum AppEvent {
    /// User pressed a key.
    Key(KeyEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Shutdown requested via Ctrl-C.
    Quit,
    /// Event from worker thread (core operations).
    Worker(WorkerEvent),
}

/// RAII guard that enters alternate screen + raw mode on creation and
/// restores the original terminal state on drop. A panic hook is installed
/// once so the terminal is also restored on panic.
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Restore terminal even on panic (installed once).
        static PANIC_HOOK: OnceLock<()> = OnceLock::new();
        PANIC_HOOK.get_or_init(|| {
            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
                default_hook(info);
            }));
        });

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
    let (ui_tx, ui_rx) = mpsc::channel::<AppEvent>();

    // Spawn worker thread
    let (worker_sender, _worker_receiver) = crate::worker::spawn_worker();

    // Dedicated reader thread — blocks on crossterm::event::read().
    std::thread::spawn(move || event_reader(ui_tx));

    while app.running {
        match ui_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.running = false;
                } else {
                    let cmd = app.handle_key(key);
                    if matches!(cmd, Command::Quit) {
                        break;
                    }
                    // Handle sniff commands by sending to worker
                    handle_app_command(cmd, &worker_sender, app);
                }
            }
            Ok(AppEvent::Resize(w, h)) => {
                let _ = terminal.resize(ratatui::layout::Rect::new(0, 0, w, h));
            }
            Ok(AppEvent::Quit) => break,
            Ok(AppEvent::Worker(worker_event)) => {
                handle_worker_event(worker_event, app);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        crate::ui::layout::render(app, &mut terminal)?;
    }

    // Shutdown worker
    let _ = worker_sender.send(WorkerMsg::Cancel);

    Ok(())
}

/// Handle application commands that require worker interaction.
fn handle_app_command(cmd: Command, worker_tx: &mpsc::Sender<WorkerMsg>, app: &App) {
    match cmd {
        Command::Sniff | Command::SniffRefresh => {
            let scan_root = app.sniff_state.scan_root.clone();
            let den_dir = std::env::var("RACCPACK_DEN")
                .ok()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".raccpack/den"));

            let _ = worker_tx.send(WorkerMsg::Sniff {
                scan_root,
                den_dir,
                force_refresh: matches!(cmd, Command::SniffRefresh),
                detect_mode: None,
                max_depth: None,
            });
        }
        Command::ChangeScanRoot => {
            // TODO: implement scan root change
        }
        _ => {}
    }
}

/// Handle events from the worker thread.
fn handle_worker_event(event: WorkerEvent, app: &mut App) {
    match event {
        WorkerEvent::Progress(progress) => {
            app.sniff_state.progress = Some(progress);
            if !app.sniff_state.is_loading {
                app.sniff_state.set_loading(true);
            }
        }
        WorkerEvent::SniffDone(result) => {
            app.sniff_state.set_loading(false);
            app.sniff_state.progress = None;
            app.sniff_state.last_refresh = Some(std::time::SystemTime::now());

            match result {
                Ok(sniff_result) => {
                    app.sniff_state.from_cache = sniff_result.from_cache;
                    app.sniff_state.projects = sniff_result
                        .report
                        .projects
                        .into_iter()
                        .map(|p| crate::app::sniff::ProjectRow {
                            name: p.name,
                            language: p.stack.language,
                            frameworks: p.stack.frameworks,
                            size_bytes: p.size_bytes,
                            is_git_repo: p.is_git_repo,
                            path: p.path,
                        })
                        .collect();
                    app.sniff_state.total_size = sniff_result.report.total_size_bytes;
                    app.sniff_state.scan_root = sniff_result.report.root;

                    if app.sniff_state.table_state.selected().is_none()
                        && !app.sniff_state.projects.is_empty()
                    {
                        app.sniff_state.table_state.select(Some(0));
                    }
                }
                Err(e) => {
                    app.sniff_state.error = Some(e.to_string());
                }
            }
        }
        WorkerEvent::Cancelled => {
            app.sniff_state.set_loading(false);
            app.sniff_state.progress = None;
        }
    }
}

/// Blocking reader that translates crossterm events into `AppEvent`s.
fn event_reader(tx: mpsc::Sender<AppEvent>) {
    loop {
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    // Ignore key release/repeat events to avoid double-handling.
                    if key.kind == KeyEventKind::Press && tx.send(AppEvent::Key(key)).is_err() {
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
