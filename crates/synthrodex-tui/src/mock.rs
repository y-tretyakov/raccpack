use std::collections::HashMap;

#[allow(dead_code)]
pub struct MockX11 {
    windows: HashMap<u64, WindowState>,
    monitors: Vec<MonitorInfo>,
    event_queue: Vec<X11Event>,
}

pub struct WindowState {
    pub name: String,
    pub monitor_id: u32,
    pub workspace: String,
    pub workspace_id: u32,
}

pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub enum X11Event {
    WindowAdded(u64),
    WindowRemoved(u64),
    MonitorChanged(u64, u32),
}

impl Default for MockX11 {
    fn default() -> Self {
        Self::new()
    }
}

impl MockX11 {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            monitors: vec![MonitorInfo {
                id: 0,
                name: "eDP-1".into(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }],
            event_queue: Vec::new(),
        }
    }

    pub fn add_window(
        &mut self,
        id: u64,
        name: &str,
        monitor_id: u32,
        workspace: &str,
        workspace_id: u32,
    ) {
        self.windows.insert(
            id,
            WindowState {
                name: name.to_string(),
                monitor_id,
                workspace: workspace.to_string(),
                workspace_id,
            },
        );
        self.event_queue.push(X11Event::WindowAdded(id));
    }

    pub fn remove_window(&mut self, id: u64) {
        self.windows.remove(&id);
        self.event_queue.push(X11Event::WindowRemoved(id));
    }

    pub fn move_window(&mut self, id: u64, new_monitor: u32) {
        if let Some(w) = self.windows.get_mut(&id) {
            w.monitor_id = new_monitor;
            self.event_queue
                .push(X11Event::MonitorChanged(id, new_monitor));
        }
    }

    pub fn windows(&self) -> &HashMap<u64, WindowState> {
        &self.windows
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn drain_events(&mut self) -> Vec<X11Event> {
        std::mem::take(&mut self.event_queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_default_monitor() {
        let mock = MockX11::new();
        assert_eq!(mock.monitors.len(), 1);
        assert_eq!(mock.monitors[0].name, "eDP-1");
    }

    #[test]
    fn add_window_increments_count() {
        let mut mock = MockX11::new();
        mock.add_window(1, "browser", 0, "main", 1);
        assert_eq!(mock.window_count(), 1);
    }

    #[test]
    fn add_window_emits_event() {
        let mut mock = MockX11::new();
        mock.add_window(1, "browser", 0, "main", 1);
        let events = mock.drain_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], X11Event::WindowAdded(1)));
    }

    #[test]
    fn remove_window_decrements_count() {
        let mut mock = MockX11::new();
        mock.add_window(1, "browser", 0, "main", 1);
        mock.remove_window(1);
        assert_eq!(mock.window_count(), 0);
    }

    #[test]
    fn move_window_updates_monitor() {
        let mut mock = MockX11::new();
        mock.add_window(1, "browser", 0, "main", 1);
        mock.move_window(1, 1);
        assert_eq!(mock.windows()[&1].monitor_id, 1);
    }

    #[test]
    fn drain_events_clears_queue() {
        let mut mock = MockX11::new();
        mock.add_window(1, "browser", 0, "main", 1);
        let _ = mock.drain_events();
        assert!(mock.drain_events().is_empty());
    }
}
