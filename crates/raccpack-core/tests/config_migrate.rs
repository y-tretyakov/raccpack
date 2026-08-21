//! Integration tests for config versioning and the migration chain.
//!
//! Spec: `docs/alpha/a4/a4.2-config-migrate-init.md`.
//!
//! Covers:
//! - `CURRENT_CONFIG_VERSION` constant (v1 in Alpha).
//! - `migrate_to_current` for missing version (v0) -> v1.
//! - `migrate_to_current` for v1 -> v1 (identity).
//! - `migrate_to_current` rejection of future versions (> `CURRENT_CONFIG_VERSION`).
//! - `RaccConfig::load_from_path` with v0, v1, and unsupported future versions.

use std::fs;
use std::path::PathBuf;

use raccpack_core::config::{migrate_to_current, ConfigError, CURRENT_CONFIG_VERSION};
use raccpack_core::RaccConfig;
use tempfile::TempDir;

/// Write a TOML file into `dir` and return its path.
fn write_toml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write toml file");
    path
}

// --- Case 1: CURRENT_CONFIG_VERSION constant --------------------------------

#[test]
fn config_current_version_is_one() {
    assert_eq!(CURRENT_CONFIG_VERSION, 1);
}

// --- Case 2: migrate_to_current with missing version (v0 -> v1) -------------

#[test]
fn config_migrate_v0_missing_version_adds_version_one() {
    let raw_toml = r#"
[paths]
scan_root = "/tmp/projects"
den_dir = "/tmp/den"

[scanner]
max_depth = 4
"#;
    let parsed: toml::Value = toml::from_str(raw_toml).expect("parse valid toml");
    let migrated = migrate_to_current(parsed).expect("migration should succeed for v0");

    let table = migrated.as_table().expect("migrated value is a table");
    assert_eq!(
        table.get("config_version").and_then(|v| v.as_integer()),
        Some(1)
    );

    // Existing fields are preserved
    let paths = table.get("paths").and_then(|v| v.as_table()).unwrap();
    assert_eq!(
        paths.get("scan_root").and_then(|v| v.as_str()),
        Some("/tmp/projects")
    );
    assert_eq!(
        paths.get("den_dir").and_then(|v| v.as_str()),
        Some("/tmp/den")
    );

    let scanner = table.get("scanner").and_then(|v| v.as_table()).unwrap();
    assert_eq!(
        scanner.get("max_depth").and_then(|v| v.as_integer()),
        Some(4)
    );
}

// --- Case 3: migrate_to_current with v1 (identity) --------------------------

#[test]
fn config_migrate_v1_identity() {
    let raw_toml = r#"
config_version = 1

[paths]
scan_root = "~/projects"

[scanner]
max_depth = 8
"#;
    let parsed: toml::Value = toml::from_str(raw_toml).expect("parse valid toml");
    let migrated = migrate_to_current(parsed).expect("v1 should pass without error");

    let table = migrated.as_table().expect("migrated value is a table");
    assert_eq!(
        table.get("config_version").and_then(|v| v.as_integer()),
        Some(1)
    );
    let scanner = table.get("scanner").and_then(|v| v.as_table()).unwrap();
    assert_eq!(
        scanner.get("max_depth").and_then(|v| v.as_integer()),
        Some(8)
    );
}

// --- Case 4: migrate_to_current with future version fails -------------------

#[test]
fn config_migrate_future_version_fails_with_incompatible_version() {
    let raw_toml = r#"
config_version = 2

[paths]
scan_root = "/tmp/future"
"#;
    let parsed: toml::Value = toml::from_str(raw_toml).expect("parse valid toml");
    let err = migrate_to_current(parsed).expect_err("future config_version must fail");

    match &err {
        ConfigError::IncompatibleVersion { found, current } => {
            assert_eq!(*found, 2);
            assert_eq!(*current, CURRENT_CONFIG_VERSION);
        }
        other => panic!("expected IncompatibleVersion error, got: {other:?}"),
    }

    assert!(
        err.suggestion().is_some(),
        "incompatible version error should have a suggestion"
    );
}

// --- Case 5: RaccConfig::load_from_path with missing config_version ---------

#[test]
fn config_load_from_path_missing_version_migrates_and_loads() {
    let temp = TempDir::new().unwrap();
    let path = write_toml(
        &temp,
        "v0_config.toml",
        r#"
[paths]
scan_root = "/tmp/projects"
den_dir = "/tmp/den"

[scanner]
max_depth = 5

[cleanup]
enabled_strategies = ["rust", "python"]
"#,
    );

    let config = RaccConfig::load_from_path(&path).expect("v0 config should load seamlessly");
    assert_eq!(config.config_version, 1);
    assert_eq!(config.paths.scan_root.as_deref(), Some("/tmp/projects"));
    assert_eq!(config.paths.den_dir.as_deref(), Some("/tmp/den"));
    assert_eq!(config.scanner.max_depth, 5);
    assert_eq!(config.cleanup.enabled_strategies, vec!["rust", "python"]);
}

// --- Case 6: RaccConfig::load_from_path with explicit config_version = 1 ---

#[test]
fn config_load_from_path_version_one_loads() {
    let temp = TempDir::new().unwrap();
    let path = write_toml(
        &temp,
        "v1_config.toml",
        r#"
config_version = 1

[paths]
scan_root = "/tmp/projects"
"#,
    );

    let config = RaccConfig::load_from_path(&path).expect("v1 config should load seamlessly");
    assert_eq!(config.config_version, 1);
    assert_eq!(config.paths.scan_root.as_deref(), Some("/tmp/projects"));
}

// --- Case 7: RaccConfig::load_from_path with future version fails -----------

#[test]
fn config_load_from_path_future_version_fails() {
    let temp = TempDir::new().unwrap();
    let path = write_toml(
        &temp,
        "future_config.toml",
        r#"
config_version = 99

[paths]
scan_root = "/tmp/projects"
"#,
    );

    let err = RaccConfig::load_from_path(&path).expect_err("future config version must fail load");
    match err {
        ConfigError::IncompatibleVersion { found, current } => {
            assert_eq!(found, 99);
            assert_eq!(current, CURRENT_CONFIG_VERSION);
        }
        other => panic!("expected IncompatibleVersion error, got: {other:?}"),
    }
}
