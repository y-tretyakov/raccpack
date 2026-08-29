//! Integration test for sniff screen.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use raccpack_tui::app::{App, Command, Focus, ViewId};
use raccpack_tui::worker::{spawn_worker, WorkerEvent, WorkerMsg};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn sniff_screen_loads_projects_from_fixture() {
    // Create a temporary directory with a fixture project structure
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create a Cargo.toml to make it a Rust project
    std::fs::write(
        project_dir.join("Cargo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Create a src directory
    std::fs::create_dir_all(project_dir.join("src")).unwrap();
    std::fs::write(project_dir.join("src/main.rs"), "fn main() {}").unwrap();

    // Spawn worker
    let (worker_tx, event_rx) = spawn_worker();

    // Send sniff command
    worker_tx
        .send(WorkerMsg::Sniff {
            scan_root: tmp.path().to_path_buf(),
            den_dir: std::path::PathBuf::from("/tmp/den"),
            force_refresh: true,
            detect_mode: None,
            max_depth: None,
        })
        .unwrap();

    // Wait for sniff to complete
    let event = event_rx.recv_timeout(Duration::from_secs(5));
    eprintln!("Received event: {:?}", event);
    let event = event.expect("Should receive event");
    eprintln!("Event type: {:?}", std::mem::discriminant(&event));

    // Wait for SniffDone event
    let mut sniff_done = None;
    for _ in 0..30 {
        let event = event_rx.recv_timeout(Duration::from_millis(500));
        eprintln!("Received event: {:?}", event);
        match event {
            Ok(WorkerEvent::SniffDone(result)) => {
                sniff_done = Some(result);
                break;
            }
            Ok(e) => {
                eprintln!("Received other event: {:?}", e);
                continue;
            }
            Err(e) => {
                eprintln!("Channel error: {:?}", e);
                continue;
            }
        };
    }
    let sniff_done = sniff_done.expect("Timeout waiting for SniffDone");
    assert!(sniff_done.is_ok());
    let sniff_result = sniff_done.unwrap();

    // Verify we found the test project
    assert!(!sniff_result.report.projects.is_empty());

    let project = sniff_result
        .report
        .projects
        .iter()
        .find(|p| p.name == "test-project")
        .expect("test-project should be discovered");

    // Verify project properties
    assert_eq!(project.stack.language.as_deref(), Some("Rust"));
    assert!(project.size_bytes > 0);
}

#[test]
fn app_handles_sniff_refresh_command() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;

    // `r` is view-scoped: it works even with sidebar focus on Projects.
    let cmd = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(cmd, Command::SniffRefresh);

    // Test that sniff keys are ignored in other views
    app.current_view = ViewId::Overview;
    let cmd = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(cmd, Command::None);
}

#[test]
fn app_handles_navigation_keys() {
    let mut app = App::new();
    app.current_view = ViewId::Projects;
    app.focus = Focus::Main;

    // Add some mock projects
    app.sniff_state.projects = vec![
        raccpack_tui::app::sniff::ProjectRow {
            name: "a".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
        raccpack_tui::app::sniff::ProjectRow {
            name: "b".into(),
            language: None,
            frameworks: vec![],
            size_bytes: 0,
            is_git_repo: false,
            path: std::path::PathBuf::new(),
        },
    ];
    app.sniff_state.table_state.select(Some(0));

    // Test j/k navigation (row movement requires Main focus)
    let cmd = app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(cmd, Command::None);
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "b");

    let cmd = app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(cmd, Command::None);
    assert_eq!(app.sniff_state.selected_project().unwrap().name, "a");
}
