use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Active,
    Inactive,
    Crashed,
}

#[derive(Debug, Clone)]
pub struct MonitorEntry {
    pub app_id: String,
    pub monitor_id: u32,
    pub state: AppState,
    pub workspace_name: String,
    pub workspace_id: u32,
    pub pid: Option<u32>,
}

pub struct MonitorState {
    entries: HashMap<String, MonitorEntry>,
    monitor_order: Vec<u32>,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            monitor_order: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        app_id: &str,
        monitor_id: u32,
        workspace_name: &str,
        workspace_id: u32,
        pid: Option<u32>,
    ) {
        let entry = self
            .entries
            .entry(app_id.to_string())
            .or_insert_with(|| MonitorEntry {
                app_id: app_id.to_string(),
                monitor_id,
                state: AppState::Inactive,
                workspace_name: workspace_name.to_string(),
                workspace_id,
                pid,
            });

        let old_monitor = entry.monitor_id;
        entry.monitor_id = monitor_id;
        entry.workspace_name = workspace_name.to_string();
        entry.workspace_id = workspace_id;
        entry.pid = pid;

        if old_monitor != monitor_id {
            entry.state = AppState::Active;
        }
    }

    pub fn set_active(&mut self, app_id: &str) {
        if let Some(entry) = self.entries.get_mut(app_id) {
            entry.state = AppState::Active;
        }
    }

    pub fn set_inactive(&mut self, app_id: &str) {
        if let Some(entry) = self.entries.get_mut(app_id) {
            entry.state = AppState::Inactive;
        }
    }

    pub fn set_crashed(&mut self, app_id: &str) {
        if let Some(entry) = self.entries.get_mut(app_id) {
            entry.state = AppState::Crashed;
        }
    }

    pub fn get(&self, app_id: &str) -> Option<&MonitorEntry> {
        self.entries.get(app_id)
    }

    pub fn get_by_monitor(&self, monitor_id: u32) -> Vec<&MonitorEntry> {
        self.entries
            .values()
            .filter(|e| e.monitor_id == monitor_id)
            .collect()
    }

    pub fn apps(&self) -> Vec<&MonitorEntry> {
        self.entries.values().collect()
    }

    pub fn set_monitor_order(&mut self, order: Vec<u32>) {
        self.monitor_order = order;
    }

    pub fn monitor_order(&self) -> &[u32] {
        &self.monitor_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_monitor_state_is_empty() {
        let state = MonitorState::new();
        assert!(state.entries.is_empty());
    }

    #[test]
    fn update_creates_entry() {
        let mut state = MonitorState::new();
        state.update("browser", 0, "main", 1, Some(1234));
        assert_eq!(state.get("browser").unwrap().monitor_id, 0);
    }

    #[test]
    fn update_moves_monitor() {
        let mut state = MonitorState::new();
        state.update("browser", 0, "main", 1, Some(1234));
        state.update("browser", 1, "dev", 2, Some(1234));
        assert_eq!(state.get("browser").unwrap().monitor_id, 1);
    }

    #[test]
    fn set_active_inactive() {
        let mut state = MonitorState::new();
        state.update("browser", 0, "main", 1, Some(1234));
        state.set_active("browser");
        assert_eq!(state.get("browser").unwrap().state, AppState::Active);
        state.set_inactive("browser");
        assert_eq!(state.get("browser").unwrap().state, AppState::Inactive);
    }

    #[test]
    fn get_by_monitor_filters() {
        let mut state = MonitorState::new();
        state.update("browser", 0, "main", 1, Some(1));
        state.update("terminal", 0, "main", 1, Some(2));
        state.update("editor", 1, "dev", 2, Some(3));
        let m0 = state.get_by_monitor(0);
        assert_eq!(m0.len(), 2);
        let m1 = state.get_by_monitor(1);
        assert_eq!(m1.len(), 1);
    }
}
