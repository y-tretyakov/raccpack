mod batcher;
mod config;
mod event;
mod perf;
mod rofi;

use config::AppConfig;
use event::EventLoop;
use std::time::Duration;

fn main() -> Result<(), String> {
    let config = AppConfig::load(None)?;
    let event_loop = EventLoop::new(Duration::from_millis(50));
    let _rx = event_loop.start();

    // TODO: setup terminal, render loop
    eprintln!(
        "synthrodex-tui started (monitor_port={})",
        config.monitor_port
    );
    Ok(())
}
