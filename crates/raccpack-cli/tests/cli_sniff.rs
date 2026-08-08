//! Integration tests for the M2.4 CLI command `racc sniff`.
//!
//! Spec: `docs/mvp/m2/m2.4-cli-sniff.md` (§4 clap, §5 text/JSON output,
//! §6 exit codes, §8 tests, §10 DoD).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir`, so sniff can never read or write the developer's real
//! `~/.config/raccpack`, `~/.cache/raccpack` or `~/.raccpack/den`.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// A self-contained test environment.
///
/// Kept alive for the whole test so the spawned `racc` processes can resolve
/// their env vars while the temp dirs still exist.
struct Harness {
    work: TempDir,
    cache_home: PathBuf,
    config_file: PathBuf,
}

/// Build a fresh harness: isolated HOME / XDG cache / config file.
fn harness() -> Harness {
    let work = TempDir::new().expect("create work dir");
    let cache_home = work.path().join("xdg-cache");
    fs::create_dir_all(&cache_home).expect("create isolated cache home");
    let config_file = work.path().join("empty-config.toml");
    fs::write(&config_file, "").expect("write empty config file");
    Harness {
        work,
        cache_home,
        config_file,
    }
}

impl Harness {
    /// Base command with a deterministic, fully isolated environment.
    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("racc").expect("locate racc binary");
        cmd.env("HOME", self.work.path())
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("RACCPACK_CONFIG", &self.config_file);
        cmd
    }

    /// A fresh, empty directory usable as a scan root.
    fn scan_root(&self) -> PathBuf {
        let root = self.work.path().join("projects");
        fs::create_dir_all(&root).expect("create scan root");
        root
    }
}

/// Write a marker project under `root`: `app-rust` (Cargo.toml + `.git` dir)
/// and `app-node` (package.json). Each project has at least one non-empty file.
fn write_two_project_fixture(root: &Path) {
    let rust = root.join("app-rust");
    fs::create_dir_all(rust.join("src")).expect("create app-rust/src");
    fs::create_dir_all(rust.join(".git")).expect("create app-rust/.git");
    fs::write(rust.join("Cargo.toml"), "[package]\nname = \"app-rust\"\n")
        .expect("write Cargo.toml");
    fs::write(rust.join("src/main.rs"), "fn main() {}\n").expect("write src/main.rs");

    let node = root.join("app-node");
    fs::create_dir_all(&node).expect("create app-node");
    fs::write(node.join("package.json"), "{\"name\": \"app-node\"}\n").expect("write package.json");
    fs::write(node.join("index.js"), "console.log('hi')\n").expect("write index.js");
}

/// Decode the captured stdout as UTF-8.
fn stdout_str(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

/// Parse the captured stdout as JSON.
fn parse_json(assert: &assert_cmd::assert::Assert) -> Value {
    serde_json::from_str(&stdout_str(assert)).expect("stdout must be valid JSON")
}

/// Project names of a `SniffResult` JSON document, in output order.
fn project_names(json: &Value) -> Vec<&str> {
    json["report"]["projects"]
        .as_array()
        .expect("projects must be an array")
        .iter()
        .map(|p| p["name"].as_str().expect("project name is a string"))
        .collect()
}

// --- §8.1 case 1: text output ----------------------------------------------

#[test]
fn sniff_text_output_lists_project_names() {
    let h = harness();
    let root = h.scan_root();
    write_two_project_fixture(&root);

    h.cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("app-rust"))
        .stdout(predicate::str::contains("app-node"));
}

// --- §8.1 case 2: JSON output ----------------------------------------------

#[test]
fn sniff_json_output_is_a_well_formed_sniff_result() {
    let h = harness();
    let root = h.scan_root();
    write_two_project_fixture(&root);

    let assert = h
        .cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .arg("--json")
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("report").is_some(), "top-level `report` present");
    assert!(
        json.get("from_cache").is_some(),
        "top-level `from_cache` present"
    );
    assert!(
        json.get("duration_ms").is_some(),
        "top-level `duration_ms` present"
    );

    let report = &json["report"];
    assert!(report.get("root").is_some(), "report.root present");
    assert!(report.get("projects").is_some(), "report.projects present");
    assert!(
        report.get("total_size_bytes").is_some(),
        "report.total_size_bytes present"
    );
    assert_eq!(report["schema_version"].as_u64(), Some(1));

    let projects = report["projects"].as_array().expect("projects is an array");
    assert_eq!(projects.len(), 2, "fixture must yield exactly two projects");

    assert_eq!(
        json["from_cache"].as_bool(),
        Some(false),
        "a fresh isolated cache cannot produce a hit"
    );

    let rust = projects
        .iter()
        .find(|p| p["name"] == "app-rust")
        .expect("app-rust is present");
    assert_eq!(rust["is_git_repo"], true, ".git marker sets is_git_repo");
    assert_eq!(rust["stack"]["language"].as_str(), Some("Rust"));

    let node = projects
        .iter()
        .find(|p| p["name"] == "app-node")
        .expect("app-node is present");
    assert_eq!(node["is_git_repo"], false);
    assert_eq!(node["stack"]["language"].as_str(), Some("JavaScript"));
}

// --- §8.1 case 3: missing root path ----------------------------------------

#[test]
fn sniff_missing_root_path_fails_with_stderr() {
    let h = harness();

    h.cmd()
        .args(["sniff", "--root", "/no/such/path"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

// --- §8.1 case 4: no scan_root at all --------------------------------------

#[test]
fn sniff_without_root_and_without_scan_root_fails_with_missing_scan_root() {
    let h = harness();

    h.cmd()
        .args(["sniff", "--config"])
        .arg(&h.config_file)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("scan_root"))
        .stderr(predicate::str::contains("--root"));
}

// --- §8.1 case 5: --force-refresh ------------------------------------------

#[test]
fn sniff_force_refresh_is_accepted_and_ignores_the_cache() {
    let h = harness();
    let root = h.scan_root();
    write_two_project_fixture(&root);

    let first = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .assert()
            .success(),
    );
    assert_eq!(first["from_cache"].as_bool(), Some(false));

    let cached = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .assert()
            .success(),
    );
    assert_eq!(
        cached["from_cache"].as_bool(),
        Some(true),
        "a second identical run must hit the cache"
    );
    assert_eq!(cached["report"], first["report"]);

    let forced = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .arg("--force-refresh")
            .assert()
            .success(),
    );
    assert_eq!(
        forced["from_cache"].as_bool(),
        Some(false),
        "force_refresh must bypass the cache"
    );
    assert_eq!(forced["report"], first["report"]);
}

// --- Bonus: --max-depth ------------------------------------------------------

#[test]
fn sniff_max_depth_excludes_nested_projects() {
    let h = harness();
    let root = h.scan_root();
    fs::create_dir_all(root.join("shallow")).expect("create shallow project");
    fs::write(root.join("shallow/Cargo.toml"), "[package]\n").expect("write shallow Cargo.toml");
    fs::create_dir_all(root.join("a/very/deep/nested")).expect("create deep project");
    fs::write(root.join("a/very/deep/nested/Cargo.toml"), "[package]\n")
        .expect("write deep Cargo.toml");

    let deep = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .assert()
            .success(),
    );
    let names_deep = project_names(&deep);
    assert!(
        names_deep.contains(&"nested"),
        "default max depth reaches the deep project"
    );

    let shallow = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .arg("--max-depth")
            .arg("1")
            .assert()
            .success(),
    );
    let names_shallow = project_names(&shallow);
    assert!(
        !names_shallow.contains(&"nested"),
        "--max-depth 1 must exclude the depth-4 project"
    );
    assert_eq!(
        names_shallow,
        vec!["shallow"],
        "only the depth-1 project remains"
    );
}

// --- Bonus: --root overrides config scan_root --------------------------------

#[test]
fn sniff_root_flag_overrides_config_scan_root() {
    let h = harness();

    let root_a = h.work.path().join("fixture-a");
    fs::create_dir_all(root_a.join("proj-a")).expect("create fixture-a/proj-a");
    fs::write(root_a.join("proj-a/Cargo.toml"), "[package]\n").expect("write fixture-a marker");

    let root_b = h.work.path().join("fixture-b");
    fs::create_dir_all(root_b.join("proj-b")).expect("create fixture-b/proj-b");
    fs::write(root_b.join("proj-b/package.json"), "{}").expect("write fixture-b marker");

    let cfg = h.work.path().join("override.toml");
    fs::write(
        &cfg,
        format!("[paths]\nscan_root = \"{}\"\n", root_a.display()),
    )
    .expect("write config with scan_root A");

    let json = parse_json(
        &h.cmd()
            .args(["sniff", "--config"])
            .arg(&cfg)
            .arg("--root")
            .arg(&root_b)
            .arg("--json")
            .assert()
            .success(),
    );

    assert_eq!(
        json["report"]["root"].as_str(),
        Some(root_b.to_str().expect("root_b is valid UTF-8")),
        "--root must win over config scan_root"
    );
    let names = project_names(&json);
    assert!(
        names.contains(&"proj-b"),
        "projects come from the --root override"
    );
    assert!(
        !names.contains(&"proj-a"),
        "config scan_root must be ignored"
    );
}

// --- Bonus: empty root -------------------------------------------------------

#[test]
fn sniff_empty_root_reports_zero_projects() {
    let h = harness();
    let root = h.scan_root();

    let text = stdout_str(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .assert()
            .success(),
    );
    assert!(
        text.contains("Projects: 0"),
        "text summary shows 0 projects"
    );

    let json = parse_json(
        &h.cmd()
            .args(["sniff", "--root"])
            .arg(&root)
            .arg("--json")
            .assert()
            .success(),
    );
    assert_eq!(
        json["report"]["projects"]
            .as_array()
            .expect("projects is an array")
            .len(),
        0
    );
}

// --- Bonus: help -------------------------------------------------------------

#[test]
fn sniff_help_lists_force_refresh_flag() {
    let h = harness();

    h.cmd()
        .arg("sniff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--force-refresh"));
}

// --- Bonus: --json is a global flag ------------------------------------------

#[test]
fn sniff_json_flag_is_accepted_before_the_subcommand() {
    let h = harness();
    let root = h.scan_root();
    write_two_project_fixture(&root);

    let json = parse_json(
        &h.cmd()
            .args(["--json", "sniff", "--root"])
            .arg(&root)
            .assert()
            .success(),
    );
    assert_eq!(
        project_names(&json),
        vec!["app-node", "app-rust"],
        "global --json must produce the same SniffResult JSON"
    );
}
