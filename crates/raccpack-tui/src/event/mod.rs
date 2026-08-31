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

use crate::app::operations::OperationKind;
use crate::app::pack::{PackFlow, PackFlowOptions, PackFlowPhase};
use crate::app::raid::{FlowPhase, RaidFlow, RaidFlowOptions};
use crate::app::{App, Command};
use crate::worker::{WorkerEvent, WorkerMsg, WorkerPassphrase, DRY_RUN_PASSPHRASE};
use raccpack_core::app::OperationKind as CoreOperationKind;

/// Events dispatched into the application loop.
///
/// The largest variant carries a completed [`WorkerEvent`] result on the UI
/// channel; the enum is always moved, never cloned, so the size is fine.
#[allow(clippy::large_enum_variant)]
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
    let (worker_sender, worker_receiver) = crate::worker::spawn_worker();

    // If `--refresh` was requested on a Projects view, kick off a sniff refresh
    // immediately (before the first render).
    if app.refresh_on_start && app.current_view == crate::app::ViewId::Projects {
        handle_app_command(Command::SniffRefresh, &worker_sender, app);
    }

    // Dedicated reader thread — blocks on crossterm::event::read().
    // Clone keeps the original `ui_tx` alive for the worker bridge below.
    let reader_tx = ui_tx.clone();
    std::thread::spawn(move || event_reader(reader_tx));

    // Bridge worker events into the UI event channel.
    std::thread::spawn(move || worker_bridge(worker_receiver, ui_tx));

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
fn handle_app_command(cmd: Command, worker_tx: &mpsc::Sender<WorkerMsg>, app: &mut App) {
    match cmd {
        Command::Sniff | Command::SniffRefresh => {
            app.sniff_state.set_loading(true);
            let scan_root = app.sniff_state.scan_root.clone();
            let den_dir = app.den_dir.clone();

            let _ = worker_tx.send(WorkerMsg::Sniff {
                scan_root,
                den_dir,
                force_refresh: matches!(cmd, Command::SniffRefresh),
                detect_mode: None,
                max_depth: None,
            });
        }
        Command::Dig => {
            let Some(project) = app.sniff_state.selected_project().map(|p| p.path.clone()) else {
                return;
            };
            start_dig(project, app, worker_tx);
        }
        Command::DigRefresh => {
            let Some(project) = app.dig_state.project.clone() else {
                return;
            };
            start_dig(project, app, worker_tx);
        }
        Command::ToggleContentScan => {
            // Flip the flag once here, then re-dig with the new value when a
            // project is in scope.
            app.dig_state.scan_content = !app.dig_state.scan_content;
            if let Some(project) = app.dig_state.project.clone() {
                start_dig(project, app, worker_tx);
            }
        }
        Command::ChangeScanRoot => {
            // TODO: implement scan root change
        }
        Command::RaidPreview => {
            let Some(project) = app.sniff_state.selected_project().map(|p| p.path.clone()) else {
                return;
            };
            start_raid_preview(project, app, worker_tx);
        }
        Command::OpenOperation => {
            // The handler already guards on a selected project; this check is a
            // second line of defence so the dispatch layer never panics.
            let Some(project) = app.sniff_state.selected_project().map(|p| p.path.clone()) else {
                return;
            };
            match app.operations_state.selected {
                OperationKind::Raid => start_raid_preview(project, app, worker_tx),
                OperationKind::Pack => start_pack_preview(project, app, worker_tx),
                // Stash/Rinse have no real flow yet (T-03/T-04): opening them
                // only shows a notice, never a core operation.
                kind @ (OperationKind::Stash | OperationKind::Rinse) => {
                    app.operations_state.open_stub(kind);
                }
            }
        }
        Command::RaidRun => send_raid_run(app, worker_tx),
        Command::RaidCancel => {
            // Esc / n while previewing or entering the passphrase closes the
            // flow. The worker preview result (if any) is ignored: the guard
            // in handle_worker_event drops events for a closed flow.
            app.raid_flow = None;
        }
        Command::PackPreview => {
            let Some(project) = app.sniff_state.selected_project().map(|p| p.path.clone()) else {
                return;
            };
            start_pack_preview(project, app, worker_tx);
        }
        Command::PackRun => send_pack_run(app, worker_tx),
        Command::PackCancel => {
            app.pack_flow = None;
        }
        Command::PackDone => {
            app.pack_flow = None;
        }
        Command::Reveal => send_reveal(app, worker_tx),
        // Pure-in-app commands were already applied inside `App::handle_key`.
        Command::BackToProjects | Command::CycleRiskFilter => {}
        _ => {}
    }
}

/// Open the raid flow for `project` and dispatch a dry-run preview to the
/// worker. No guard is needed beyond the missing-selection check: while a flow
/// is active the app blocks all other keys, so a second preview cannot start.
fn start_raid_preview(
    project: std::path::PathBuf,
    app: &mut App,
    worker_tx: &mpsc::Sender<WorkerMsg>,
) {
    let flow = RaidFlow::new(
        project.clone(),
        app.den_dir.clone(),
        RaidFlowOptions::default(),
    );
    let opts = flow.options.into();
    app.raid_flow = Some(flow);
    app.help_visible = false;
    let den_dir = app.den_dir.clone();
    let _ = worker_tx.send(WorkerMsg::RaidPreview {
        project,
        den_dir,
        opts,
    });
}

/// Open the pack flow for `project` and dispatch a dry-run preview to the
/// worker. No guard is needed beyond the missing-selection check: while a flow
/// is active the app blocks all other keys, so a second preview cannot start.
fn start_pack_preview(
    project: std::path::PathBuf,
    app: &mut App,
    worker_tx: &mpsc::Sender<WorkerMsg>,
) {
    let flow = PackFlow::new(
        project.clone(),
        app.den_dir.clone(),
        PackFlowOptions::default(),
    );
    let opts = flow.options.clone().into();
    app.pack_flow = Some(flow);
    app.help_visible = false;
    let den_dir = app.den_dir.clone();
    let _ = worker_tx.send(WorkerMsg::PackPreview {
        project,
        den_dir,
        opts,
    });
}

/// Dispatch the pack commit to the worker. No passphrase is needed at the
/// pack level (only stash encrypts secrets).
fn send_pack_run(app: &mut App, worker_tx: &mpsc::Sender<WorkerMsg>) {
    let Some(flow) = app.pack_flow.as_ref() else {
        return;
    };
    let project = flow.project.clone();
    let den_dir = flow.den_dir.clone();
    let opts = flow.options.clone().into();
    if let Some(flow) = app.pack_flow.as_mut() {
        flow.start_running();
    }
    let _ = worker_tx.send(WorkerMsg::PackRun {
        project,
        den_dir,
        opts,
    });
}

/// Dispatch the raid commit after resolving the passphrase.
///
/// Resolution order:
/// 1. passphrase confirmed in the modal (taken out of the flow);
/// 2. stash skipped → placeholder identity (stash phase is disabled);
/// 3. `RACCPACK_PASSPHRASE` env → run immediately;
/// 4. otherwise open the passphrase modal and wait for confirmation.
fn send_raid_run(app: &mut App, worker_tx: &mpsc::Sender<WorkerMsg>) {
    let flow = app.raid_flow.as_mut();
    let Some(flow) = flow else {
        return;
    };
    let Some(passphrase) = resolve_raid_passphrase(flow) else {
        return; // passphrase modal opened; the run starts on confirmation
    };
    let project = flow.project.clone();
    let opts = flow.options.into();
    flow.start_running();
    let den_dir = app.den_dir.clone();
    let _ = worker_tx.send(WorkerMsg::RaidRun {
        project,
        den_dir,
        opts,
        passphrase,
    });
}

/// Resolve the passphrase for a raid run (see [`send_raid_run`]); returns
/// `None` after moving the flow to the passphrase modal.
fn resolve_raid_passphrase(flow: &mut RaidFlow) -> Option<WorkerPassphrase> {
    if let Some(passphrase) = flow.take_passphrase() {
        return Some(WorkerPassphrase::from_zeroizing(passphrase));
    }
    if flow.options.skip_stash {
        return Some(WorkerPassphrase::new(DRY_RUN_PASSPHRASE.to_string()));
    }
    if let Ok(env) = std::env::var("RACCPACK_PASSPHRASE") {
        if !env.is_empty() {
            return Some(WorkerPassphrase::new(env));
        }
    }
    flow.start_passphrase();
    None
}

/// Send one dig run for `project` to the worker.
fn start_dig(project: std::path::PathBuf, app: &mut App, worker_tx: &mpsc::Sender<WorkerMsg>) {
    // A new dig invalidates any in-flight reveal; drop the modal (zeroizes the
    // secret if one was shown).
    app.reveal = None;
    app.dig_state.set_loading(true);
    app.dig_state.project = Some(project.clone());
    let den_dir = app.den_dir.clone();
    let scan_content = app.dig_state.scan_content;
    let _ = worker_tx.send(WorkerMsg::Dig {
        project,
        den_dir,
        scan_content,
    });
}

/// Dispatch a confirmed ephemeral reveal to the worker.
///
/// The modal already holds the sensitive file path + reference; the resolved
/// project directory (`dig_state.project`) is passed as `dir_root` so the core
/// can enforce path containment. The worker replies with `RevealReady`
/// (zeroized value) or `RevealFailed`.
fn send_reveal(app: &mut App, worker_tx: &mpsc::Sender<WorkerMsg>) {
    let Some(dir_root) = app.dig_state.project.clone() else {
        return;
    };
    let Some(modal) = app.reveal.as_ref() else {
        return;
    };
    let path = modal.path.clone();
    let reference = modal.reference.clone();
    let _ = worker_tx.send(WorkerMsg::Reveal {
        path,
        dir_root,
        reference,
    });
}

/// Handle events from the worker thread.
fn handle_worker_event(event: WorkerEvent, app: &mut App) {
    match event {
        WorkerEvent::Progress(progress) => {
            match progress.operation {
                CoreOperationKind::Sniff => {
                    app.sniff_state.progress = Some(progress);
                    if !app.sniff_state.is_loading {
                        app.sniff_state.set_loading(true);
                    }
                }
                CoreOperationKind::Dig => {
                    app.dig_state.progress = Some(progress);
                    if !app.dig_state.is_loading {
                        app.dig_state.set_loading(true);
                    }
                }
                CoreOperationKind::Raid => {
                    if let Some(flow) = app.raid_flow.as_mut() {
                        if flow.phase == FlowPhase::Running {
                            flow.on_progress(&progress);
                        }
                    }
                }
                CoreOperationKind::Pack => {
                    if let Some(flow) = app.pack_flow.as_mut() {
                        if flow.phase == crate::app::pack::PackFlowPhase::Running {
                            flow.on_progress(progress.percent, &progress.message);
                        }
                    }
                }
                // Stash/rinse progress has no screen yet.
                _ => {}
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
        WorkerEvent::DigDone(result) => {
            app.dig_state.set_loading(false);
            app.dig_state.progress = None;
            app.dig_state.last_run = Some(std::time::SystemTime::now());

            match result {
                Ok(dig_result) => {
                    app.dig_state.set_dig_result(dig_result);
                    // Move the selection onto the first visible row.
                    app.dig_state.select_first();
                }
                Err(e) => {
                    app.dig_state.error = Some(e.to_string());
                }
            }
        }
        WorkerEvent::RaidPreviewDone(result) => {
            let Some(flow) = app.raid_flow.as_mut() else {
                return; // flow closed (cancelled) before the preview landed
            };
            flow.phase = match result {
                Ok(preview) => FlowPhase::Preview(preview),
                Err(e) => FlowPhase::Failed(e.to_string()),
            };
        }
        WorkerEvent::RaidDone(result) => {
            let Some(flow) = app.raid_flow.as_mut() else {
                return; // flow closed (cancelled) before the run finished
            };
            flow.phase = match result {
                Ok(done) => FlowPhase::Done(done),
                Err(e) => FlowPhase::Failed(e.to_string()),
            };
        }
        WorkerEvent::PackPreviewDone(result) => {
            let Some(flow) = app.pack_flow.as_mut() else {
                return; // flow closed (cancelled) before the preview landed
            };
            flow.phase = match result {
                Ok(preview) => PackFlowPhase::Preview(preview),
                Err(e) => PackFlowPhase::Failed(e.to_string()),
            };
        }
        WorkerEvent::PackDone(result) => {
            let Some(flow) = app.pack_flow.as_mut() else {
                return; // flow closed (cancelled) before the run finished
            };
            flow.phase = match result {
                Ok(done) => PackFlowPhase::Done(done),
                Err(e) => PackFlowPhase::Failed(e.to_string()),
            };
        }
        WorkerEvent::RevealReady(secret) => {
            // Only land the value if the modal is still open and expecting it.
            // Dropping the modal (or ignoring this event) zeroizes the secret.
            if let Some(modal) = app.reveal.as_mut() {
                if matches!(modal.phase, crate::app::reveal::RevealPhase::Revealing) {
                    modal.set_ready(secret);
                }
            }
        }
        WorkerEvent::RevealFailed(err) => {
            if let Some(modal) = app.reveal.as_mut() {
                if matches!(modal.phase, crate::app::reveal::RevealPhase::Revealing) {
                    modal.set_failed(err.to_string());
                }
            }
        }
        WorkerEvent::Cancelled => {
            app.sniff_state.set_loading(false);
            app.sniff_state.progress = None;
            app.dig_state.set_loading(false);
            app.dig_state.progress = None;
        }
    }
}

/// Bridges worker events into the UI event channel; exits when the worker is gone.
fn worker_bridge(receiver: mpsc::Receiver<WorkerEvent>, tx: mpsc::Sender<AppEvent>) {
    while let Ok(ev) = receiver.recv() {
        if tx.send(AppEvent::Worker(ev)).is_err() {
            break;
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

#[cfg(test)]
mod tests;
