#![allow(dead_code)]

use std::time::{Duration, Instant};

pub struct PerfGuard {
    label: String,
    start: Instant,
    threshold: Duration,
}

impl PerfGuard {
    pub fn new(label: &str, threshold_ms: u64) -> Self {
        Self {
            label: label.to_string(),
            start: Instant::now(),
            threshold: Duration::from_millis(threshold_ms),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn is_slow(&self) -> bool {
        self.start.elapsed() > self.threshold
    }
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if elapsed > self.threshold {
            eprintln!(
                "[PERF] {} took {:?} (threshold {:?})",
                self.label, elapsed, self.threshold
            );
        }
    }
}

pub struct PerfStats {
    pub x11_avg_ms: f64,
    pub render_avg_ms: f64,
    pub fps: f64,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfStats {
    pub fn new() -> Self {
        Self {
            x11_avg_ms: 0.0,
            render_avg_ms: 0.0,
            fps: 0.0,
        }
    }

    pub fn update(&mut self, _x11_ms: f64, _render_ms: f64, _fps: f64) {
        // Stub: in real impl, use rolling average
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_guard_tracks_elapsed() {
        let guard = PerfGuard::new("test", 1000);
        std::thread::sleep(Duration::from_millis(10));
        assert!(guard.elapsed() >= Duration::from_millis(10));
        assert!(!guard.is_slow());
    }

    #[test]
    fn perf_stats_new_defaults() {
        let stats = PerfStats::new();
        assert_eq!(stats.x11_avg_ms, 0.0);
        assert_eq!(stats.render_avg_ms, 0.0);
        assert_eq!(stats.fps, 0.0);
    }

    #[test]
    fn perf_stats_update_no_panic() {
        let mut stats = PerfStats::new();
        stats.update(1.5, 2.0, 60.0);
    }
}
