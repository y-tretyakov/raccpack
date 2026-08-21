use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn cli_init_writes_default_config_file() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg("init")
        .arg("--scan-root")
        .arg("/tmp/my_projects")
        .arg("--den")
        .arg("/tmp/my_den");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created config file:"));

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("config_version = 1"));
    assert!(content.contains("scan_root = \"/tmp/my_projects\""));
    assert!(content.contains("den_dir = \"/tmp/my_den\""));
    assert!(content.contains("https://y-tretyakov.github.io/raccpack/"));
}

#[test]
fn cli_init_fails_on_existing_config_without_force() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    fs::write(&config_path, "existing config").unwrap();

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config").arg(&config_path).arg("init");

    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("config file already exists"))
        .stderr(predicate::str::contains("Use --force"));
}

#[test]
fn cli_init_overwrites_with_force() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    fs::write(&config_path, "corrupt config content").unwrap();

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg("init")
        .arg("--force")
        .arg("--scan-root")
        .arg("/tmp/new_root");

    cmd.assert().success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("scan_root = \"/tmp/new_root\""));
}

#[test]
fn cli_init_with_ensure_den_creates_den_skeleton() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let den_path = tmp.path().join("vault");

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg("--den")
        .arg(&den_path)
        .arg("init")
        .arg("--ensure-den");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized den vault:"));

    assert!(den_path.join(".den-version").exists());
    assert!(den_path.join("README.txt").exists());
}

#[test]
fn cli_init_json_output() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let den_path = tmp.path().join("vault");

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg("--den")
        .arg(&den_path)
        .arg("--json")
        .arg("init")
        .arg("--ensure-den");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout must be valid JSON");

    assert_eq!(
        parsed["config_path"].as_str().unwrap(),
        config_path.to_str().unwrap()
    );
    assert_eq!(
        parsed["den_dir"].as_str().unwrap(),
        den_path.to_str().unwrap()
    );
}

#[test]
fn cli_init_help_lists_flags() {
    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("init").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--scan-root"))
        .stdout(predicate::str::contains("--ensure-den"));
}

#[test]
fn cli_init_creates_nested_parent_directories() {
    let tmp = TempDir::new().unwrap();
    let nested_config = tmp
        .path()
        .join("deep")
        .join("nested")
        .join("dir")
        .join("config.toml");

    let mut cmd = Command::cargo_bin("racc").unwrap();
    cmd.arg("--config").arg(&nested_config).arg("init");

    cmd.assert().success();
    assert!(nested_config.exists());
}
