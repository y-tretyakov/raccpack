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
    match run_event_loop(&mut app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("TUI error: {e}");
            ExitCode::FAILURE
        }
    }
}
