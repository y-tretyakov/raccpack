use synthrodex_tui::batcher::{BatchRequest, BatchType, Batcher};
use synthrodex_tui::monitor::MonitorState;
use synthrodex_tui::nowbar::{NowPlaying, TimeProvider};
use synthrodex_tui::widgets::FocusGrid;

mod mock {
    #[allow(dead_code)]
    pub struct MockX11 {
        pub windows: std::collections::HashMap<u64, String>,
        pub monitor: u32,
    }

    impl MockX11 {
        pub fn new() -> Self {
            Self {
                windows: std::collections::HashMap::new(),
                monitor: 0,
            }
        }

        pub fn add_window(&mut self, id: u64, name: &str) {
            self.windows.insert(id, name.to_string());
        }

        #[allow(dead_code)]
        pub fn remove_window(&mut self, id: u64) {
            self.windows.remove(&id);
        }

        pub fn window_count(&self) -> usize {
            self.windows.len()
        }

        #[allow(dead_code)]
        pub fn get_monitor(&self) -> u32 {
            self.monitor
        }
    }
}

#[test]
fn integration_batcher_with_mock() {
    let mut mock = mock::MockX11::new();
    mock.add_window(1, "browser");
    mock.add_window(2, "terminal");
    mock.add_window(3, "editor");

    let mut batcher = Batcher::new(mock.window_count());

    let ids: Vec<u64> = mock.windows.keys().copied().collect();
    for id in &ids {
        batcher.queue_request(BatchRequest {
            request_type: BatchType::Workspace,
            window: *id,
        });
    }

    assert!(batcher.should_flush());
    let batch = batcher.flush();
    assert_eq!(batch.len(), 3);
}

#[test]
fn integration_monitor_with_mock() {
    let mut mock = mock::MockX11::new();
    mock.add_window(1, "browser");
    mock.add_window(2, "terminal");

    let mut monitor = MonitorState::new();

    let entries: Vec<(u64, String)> = mock
        .windows
        .iter()
        .map(|(id, name)| (*id, name.clone()))
        .collect();
    for (id, name) in &entries {
        monitor.update(name, 0, "main", 1, Some(*id as u32));
    }

    let apps = monitor.apps();
    assert_eq!(apps.len(), 2);
}

#[test]
fn integration_nowbar_with_mock() {
    let mut mock = mock::MockX11::new();
    mock.add_window(1, "player");

    let tp = TimeProvider::new();
    let mut np = NowPlaying::new();
    np.update(None);

    let bar = synthrodex_tui::nowbar::NowBar::new(&tp, &np);
    let rendered = bar.render_time();
    assert!(!rendered.is_empty());
}

#[test]
fn integration_focusgrid_with_mock() {
    let mut mock = mock::MockX11::new();
    mock.add_window(1, "browser");
    mock.add_window(2, "terminal");

    let mut grid = FocusGrid::new();
    grid.set_cell(0, 0, Some("browser".into()));
    grid.set_cell(0, 1, Some("terminal".into()));
    grid.set_focus(0, 0);

    assert_eq!(grid.focused_label(), Some("browser"));
}

#[test]
fn integration_batcher_flush_cycle() {
    let mut mock = mock::MockX11::new();
    mock.add_window(1, "browser");
    mock.add_window(2, "terminal");
    mock.add_window(3, "editor");

    let mut batcher = Batcher::new(3);

    batcher.queue_request(BatchRequest {
        request_type: BatchType::Workspace,
        window: 1,
    });
    batcher.queue_request(BatchRequest {
        request_type: BatchType::Workspace,
        window: 2,
    });
    assert!(!batcher.should_flush());

    batcher.queue_request(BatchRequest {
        request_type: BatchType::Workspace,
        window: 3,
    });
    assert!(batcher.should_flush());

    let batch = batcher.flush();
    assert_eq!(batch.len(), 3);
    assert_eq!(batcher.queue_len(), 0);
}

#[test]
fn integration_monitor_move_across_monitors() {
    let mut monitor = MonitorState::new();
    monitor.update("browser", 0, "main", 1, Some(1));
    assert_eq!(monitor.get("browser").unwrap().monitor_id, 0);

    monitor.update("browser", 1, "dev", 2, Some(1));
    assert_eq!(monitor.get("browser").unwrap().monitor_id, 1);

    let m0 = monitor.get_by_monitor(0);
    assert!(m0.is_empty());
    let m1 = monitor.get_by_monitor(1);
    assert_eq!(m1.len(), 1);
}
