use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;

use raccpack_tui::app::App;
use raccpack_tui::cli::Cli;
use raccpack_tui::event::{run_event_loop, TerminalGuard};

fn main() -> ExitCode {
    // 1. Parse argv before any terminal init.
    let cli = Cli::parse();
    // 2.-3. clap handles `--version`/`-V` and `--help`/`-h` by printing and
    //        exiting before `run()` is reached — no TerminalGuard involved.

    // 4. Refuse non-interactive invocation cleanly, before raw mode.
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        eprintln!("racc-tui requires an interactive terminal");
        return ExitCode::FAILURE;
    }

    // 5. Enter raw mode + alternate screen.
    let _guard = match TerminalGuard::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to initialize terminal: {e}");
            return ExitCode::FAILURE;
        }
    };

    // 6. Build App from defaults + CLI overrides.
    let mut app = App::new();

    // Default scan root: ~/DEV/PROJS if it exists, else current dir.
    let default_scan_root = dirs::home_dir()
        .map(|h| h.join("DEV/PROJS"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    app.sniff_state.scan_root = default_scan_root;

    apply_launch_args(&mut app, cli);

    match run_event_loop(&mut app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("TUI error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Apply parsed launch args onto the app. Pure wiring; no domain logic.
fn apply_launch_args(app: &mut App, cli: Cli) {
    if let Some(root) = cli.root {
        app.sniff_state.scan_root = root;
    }
    if let Some(den) = cli.den {
        app.den_dir = den;
    }
    // Resolve den dir: flag > env > default. (Flag already applied above if set.)
    if app.den_dir.as_os_str().is_empty() {
        app.den_dir = resolve_den_dir();
    }
    app.current_view = cli.view.to_view_id();
    app.refresh_on_start = cli.refresh;
    // `config` and `verbose` are parsed/stored on the Cli for later; nothing is
    // wired to the worker or ratatui yet.
    let _ = cli.config;
    let _ = cli.verbose;
}

/// Resolve the den directory from `RACCPACK_DEN`, falling back to
/// `~/.raccpack/den`.
fn resolve_den_dir() -> std::path::PathBuf {
    std::env::var("RACCPACK_DEN")
        .ok()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".raccpack/den"))
}
