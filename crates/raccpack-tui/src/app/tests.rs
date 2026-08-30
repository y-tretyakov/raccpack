//! Unit tests for `App` key routing and screen state (module `tests` of `app`).
//!
//! The test body is split into focused submodules (keys, views, sniff state,
//! raid flow) which share the helpers defined here.

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn project_row(name: &str, path: &str) -> sniff::ProjectRow {
    sniff::ProjectRow {
        name: name.to_string(),
        language: None,
        frameworks: vec![],
        size_bytes: 0,
        is_git_repo: false,
        path: std::path::PathBuf::from(path),
    }
}

fn preview_result() -> raccpack_core::app::RaidResult {
    raccpack_core::app::RaidResult {
        project_path: std::path::PathBuf::from("/tmp/a"),
        stages: Vec::new(),
        stash: None,
        rinse: None,
        pack: None,
        den_artifacts: Vec::new(),
        success: true,
        dry_run: true,
        rolled_back: false,
        rollback_warnings: Vec::new(),
    }
}

fn raid_flow_in(phase: raid::FlowPhase) -> raid::RaidFlow {
    let mut flow = raid::RaidFlow::new(
        std::path::PathBuf::from("/tmp/a"),
        std::path::PathBuf::from("/tmp/den"),
        raid::RaidFlowOptions::default(),
    );
    flow.phase = phase;
    flow
}

mod keys_tests;
mod raid_tests;
mod sniff_tests;
mod view_tests;
