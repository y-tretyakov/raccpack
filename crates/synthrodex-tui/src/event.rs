#![allow(dead_code)]

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    // Future: WorkerAdded, NowPlayingUpdate, etc.
}

pub struct EventLoop {
    tick_rate: Duration,
}

impl EventLoop {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn start(&self) -> mpsc::Receiver<AppEvent> {
        let (tx, rx) = mpsc::channel(100);
        let tick_rate = self.tick_rate;

        tokio::spawn(async move {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(CrosstermEvent::Key(key)) = event::read() {
                        let _ = tx.send(AppEvent::Key(key)).await;
                    }
                } else {
                    let _ = tx.send(AppEvent::Tick).await;
                }
            }
        });

        rx
    }
}
