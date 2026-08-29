use std::process::ExitCode;

use raccpack_tui::app::App;
use raccpack_tui::event::{run_event_loop, TerminalGuard};

fn main() -> ExitCode {
    let _guard = match TerminalGuard::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to initialize terminal: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut app = App::new();
    // Set default scan root to ~/DEV/PROJS if it exists, otherwise current dir
    let default_scan_root = dirs::home_dir()
        .map(|h| h.join("DEV/PROJS"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    app.sniff_state.scan_root = default_scan_root;

    match run_event_loop(&mut app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("TUI error: {e}");
            ExitCode::FAILURE
        }
    }
}
