//! Integration tests for the A2.3 CLI command `racc rinse`.
//!
//! Spec: `docs/alpha/a2/a2.3-cli-rinse.md` (§3 clap, §4 flow, §5 tests,
//! §6 DoD).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir` (mirrors `cli_stash.rs`). The config file is empty, so rinse
//! falls back to the core defaults: `cleanup.enabled_strategies =
//! [rust, node, python]` and `scanner.max_depth = 6`. The den is irrelevant
//! for rinse (spec §3: `--den` ignored) and is deliberately not passed.
//!
//! The base [`Harness::cmd`] additionally removes `RACCPACK_PASSPHRASE` from
//! the child env (rinse never reads it, but the harness stays identical to
//! cli_stash.rs) and pins the child stdin to [`std::process::Stdio::null`] so
//! no run can ever block on a tty prompt.
//!
//! NOTE: the `rinse` subcommand is being implemented in parallel by the Dev
//! agent. Until that lands these tests cannot pass (clap rejects the unknown
//! subcommand); they encode the behaviour from the spec and are stitched to
//! the code at acceptance. Assertions that depend on the exact human-output
//! wording are marked inline.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

/// Byte count of the deterministic payload inside every trash dir. Non-zero,
/// so `bytes_freed >= 1` is guaranteed on any filesystem.
const TRASH_FILE_BYTES: usize = 4096;

// --- Test helpers -----------------------------------------------------------

/// A self-contained test environment (same isolation pattern as cli_stash.rs).
///
/// Kept alive for the whole test so the spawned `racc` processes can resolve
/// their env vars while the temp dirs still exist.
struct Harness {
    work: TempDir,
    cache_home: PathBuf,
    config_file: PathBuf,
}

/// Build a fresh harness: isolated HOME / XDG cache / empty config file.
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
    ///
    /// The child's `RACCPACK_PASSPHRASE` is always removed and its stdin is
    /// pinned to null, so no run can ever hang on an interactive prompt.
    fn cmd(&self) -> Command {
        let mut std_cmd = std::process::Command::cargo_bin("racc").expect("locate racc binary");
        std_cmd.stdin(std::process::Stdio::null());
        let mut cmd = Command::from_std(std_cmd);
        cmd.env("HOME", self.work.path())
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("RACCPACK_CONFIG", &self.config_file)
            .env_remove("RACCPACK_PASSPHRASE");
        cmd
    }

    /// A fresh, empty directory usable as a project source.
    fn projects_root(&self) -> PathBuf {
        let root = self.work.path().join("projects");
        fs::create_dir_all(&root).expect("create projects root");
        root
    }
}

/// Create a file, creating any parent directories first.
fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, content).expect("write fixture file");
}

/// Decode the captured stdout as UTF-8.
fn stdout_str(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

/// Parse the captured stdout as JSON.
fn parse_json(assert: &assert_cmd::assert::Assert) -> Value {
    serde_json::from_str(&stdout_str(assert)).expect("stdout must be valid JSON")
}

/// A deterministic payload of exactly `TRASH_FILE_BYTES` non-zero bytes.
fn trash_payload() -> String {
    "x".repeat(TRASH_FILE_BYTES)
}

/// A generic project with `node_modules/` and `target/` trash dirs, each
/// holding a fixed-size payload file. Both are matched by the default cleanup
/// strategies (node and rust).
fn trash_project(root: &Path) -> PathBuf {
    let app = root.join("app");
    write(&app, "node_modules/pkg/index.js", &trash_payload());
    write(&app, "target/debug/app", &trash_payload());
    app
}

/// Assert the top-level shape of the JSON `RinseResult` (spec §4): exactly the
/// `removed`, `bytes_freed` and `dry_run` fields, nothing else.
fn assert_rinse_result_shape(json: &Value) {
    let obj = json.as_object().expect("JSON output must be an object");
    assert_eq!(
        obj.len(),
        3,
        "RinseResult must have exactly three top-level fields"
    );
    assert!(obj.contains_key("removed"), "has `removed`");
    assert!(obj.contains_key("bytes_freed"), "has `bytes_freed`");
    assert!(obj.contains_key("dry_run"), "has `dry_run`");
}

/// Assert the shape of one `removed` entry (spec §4): path / strategy /
/// pattern_name / size_bytes.
fn assert_removed_entry_shape(entry: &Value) {
    assert!(entry["path"].is_string(), "removed[].path must be a string");
    assert!(
        entry["strategy"].is_string(),
        "removed[].strategy must be a string"
    );
    assert!(
        entry["pattern_name"].is_string(),
        "removed[].pattern_name must be a string"
    );
    assert!(
        entry["size_bytes"].is_u64(),
        "removed[].size_bytes must be a number"
    );
}

// --- Case 1: parse rinse flags (behavioural check via --help) ----------------

#[test]
fn rinse_help_lists_all_rinse_flags() {
    let h = harness();

    h.cmd()
        .args(["rinse", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--strategy"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--den"));
}

// --- Case 2: DryRun leaves the trash dirs in place ---------------------------

#[test]
fn rinse_dry_run_leaves_trash_dirs() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    // Default mode is DryRun (spec §1); no --yes is required.
    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .assert()
        .success();

    assert!(
        app.join("node_modules").is_dir(),
        "dry-run must leave node_modules/ in place"
    );
    assert!(
        app.join("target").is_dir(),
        "dry-run must leave target/ in place"
    );
}

// --- Case 3: --strategy override restricts the matched dirs ------------------

#[test]
fn rinse_strategy_node_commit_removes_only_node_modules() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--strategy", "node", "--yes"])
        .assert()
        .success();

    assert!(
        !app.join("node_modules").exists(),
        "--strategy node --yes must remove node_modules/"
    );
    assert!(
        app.join("target").is_dir(),
        "rust target/ must stay (node strategy only)"
    );
}

#[test]
fn rinse_strategy_rust_commit_removes_only_target() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--strategy", "rust", "--yes"])
        .assert()
        .success();

    assert!(
        !app.join("target").exists(),
        "--strategy rust --yes must remove target/"
    );
    assert!(
        app.join("node_modules").is_dir(),
        "node node_modules/ must stay (rust strategy only)"
    );
}

#[test]
fn rinse_strategy_rust_and_node_commit_removes_both() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    // Both `--strategy` flags are honored (repeatable, spec §3).
    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--strategy", "rust", "--strategy", "node", "--yes"])
        .assert()
        .success();

    assert!(
        !app.join("node_modules").exists(),
        "combined --strategy must remove node_modules/"
    );
    assert!(
        !app.join("target").exists(),
        "combined --strategy must remove target/"
    );
}

// --- Case 4: --yes without --strategy applies the config defaults -------------

#[test]
fn rinse_commit_yes_applies_default_strategies() {
    let h = harness();
    let root = h.projects_root();
    let app = root.join("app");
    write(&app, "node_modules/pkg/index.js", &trash_payload());
    write(&app, "target/debug/app", &trash_payload());
    write(&app, "__pycache__/mod.cpython-311.pyc", &trash_payload());

    // Empty config file → core defaults rust + node + python (spec §3):
    // all three dirs must be matched and removed.
    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--yes"])
        .assert()
        .success();

    assert!(
        !app.join("node_modules").exists(),
        "default strategies must remove node_modules/"
    );
    assert!(
        !app.join("target").exists(),
        "default strategies must remove target/"
    );
    assert!(
        !app.join("__pycache__").exists(),
        "default strategies must remove __pycache__/"
    );
}

// --- Case 5: JSON output shape (dry-run and commit) ---------------------------

#[test]
fn rinse_json_dry_run_reports_removed_and_bytes() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    let assert = h
        .cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert_rinse_result_shape(&json);
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(true),
        "default run must report dry_run == true"
    );

    let removed = json["removed"].as_array().expect("removed is an array");
    assert!(!removed.is_empty(), "removed must list the candidates");
    for entry in removed {
        assert_removed_entry_shape(entry);
    }

    let by_pattern = |name: &str| {
        removed
            .iter()
            .find(|entry| entry["pattern_name"] == name)
            .unwrap_or_else(|| panic!("removed must contain `{name}`"))
    };
    assert_eq!(
        by_pattern("node_modules")["strategy"],
        "node",
        "node_modules must be tagged with the node strategy"
    );
    assert_eq!(
        by_pattern("target")["strategy"],
        "rust",
        "target must be tagged with the rust strategy"
    );

    let sum: u64 = removed
        .iter()
        .map(|entry| {
            entry["size_bytes"]
                .as_u64()
                .expect("size_bytes is a number")
        })
        .sum();
    assert_eq!(
        json["bytes_freed"].as_u64(),
        Some(sum),
        "dry-run bytes_freed must equal the sum of the removed sizes"
    );
    assert!(
        sum >= 1,
        "fixture must be sized so bytes_freed >= 1, got {sum}"
    );
}

#[test]
fn rinse_json_commit_reports_actual_removal() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    let assert = h
        .cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--yes", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert_rinse_result_shape(&json);
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(false),
        "--yes must report dry_run == false"
    );

    let removed = json["removed"].as_array().expect("removed is an array");
    assert!(!removed.is_empty(), "removed must list the deleted dirs");
    for entry in removed {
        assert_removed_entry_shape(entry);
    }
    let freed = json["bytes_freed"]
        .as_u64()
        .expect("bytes_freed is a number");
    assert!(freed >= 1, "bytes_freed must be >= 1: {freed}");

    assert!(
        !app.join("node_modules").exists(),
        "commit must physically delete node_modules/"
    );
    assert!(
        !app.join("target").exists(),
        "commit must physically delete target/"
    );
}

// --- Case 6: unknown --strategy id → exit 1 -----------------------------------

#[test]
fn rinse_unknown_strategy_fails_and_deletes_nothing() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    // `--strategy foo` is a free-form string, so clap accepts it; the core
    // rejects the unknown id with exit 1 (spec §1.1 "негативные").
    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--strategy", "foo", "--yes"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());

    assert!(
        app.join("node_modules").exists() && app.join("target").exists(),
        "a failed run must delete nothing"
    );
}

// --- Case 7: --dry-run wins over --yes ----------------------------------------

#[test]
fn rinse_dry_run_wins_over_yes() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    // --dry-run overrides --yes (spec §3); nothing is deleted.
    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--dry-run", "--yes"])
        .assert()
        .success();

    assert!(
        app.join("node_modules").is_dir(),
        "--dry-run must win over --yes (node_modules stays)"
    );
    assert!(
        app.join("target").is_dir(),
        "--dry-run must win over --yes (target stays)"
    );
}

// --- Case 8: missing --project is rejected ------------------------------------

#[test]
fn rinse_missing_project_is_rejected() {
    let h = harness();

    // `--project` is required (spec §3), so clap must reject the invocation.
    // Assert `.failure()` plus a non-empty stderr so the test stays stable
    // across clap versions (mirrors cli_stash.rs / cli_pack.rs).
    h.cmd()
        .args(["rinse", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// --- Case 9: --project . from the project cwd ---------------------------------

#[test]
fn rinse_project_dot_from_cwd_commits() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    // `racc rinse --project . --yes` with cwd == project (spec §1.1).
    h.cmd()
        .current_dir(&app)
        .args(["rinse", "--project", ".", "--yes"])
        .assert()
        .success();

    assert!(
        !app.join("node_modules").exists(),
        "--project . must remove node_modules/"
    );
    assert!(
        !app.join("target").exists(),
        "--project . must remove target/"
    );
}

// --- Case 10: a symlink to an external dir is never followed ------------------

#[cfg(unix)]
#[test]
fn rinse_commit_leaves_external_symlink_untouched() {
    use std::os::unix::fs::symlink;

    let h = harness();
    let root = h.projects_root();
    let app = root.join("app");
    write(&app, "src/main.rs", "fn main() {}\n");

    // The symlink target lives OUTSIDE the project and holds a marker file:
    // following it during cleanup would delete an external tree (spec wiki).
    let outside = root.join("outside");
    write(&outside, "marker.txt", "external-marker\n");
    symlink(&outside, app.join("node_modules")).expect("create symlink");

    h.cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--yes"])
        .assert()
        .success();

    let meta = fs::symlink_metadata(app.join("node_modules"))
        .expect("the symlink must still exist after commit");
    assert!(
        meta.file_type().is_symlink(),
        "the matched entry must remain a symlink"
    );
    assert!(
        outside.join("marker.txt").is_file(),
        "the external marker file must survive the commit"
    );
}

// --- Case 11: human output wording (dry-run and commit) -----------------------

#[test]
fn rinse_human_dry_run_wording() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    let assert = h
        .cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    // Wording from spec §4; Dev may adjust the exact phrasing.
    assert!(stdout.contains("Rinse (dry-run)"), "dry-run header");
    assert!(
        stdout.contains("node_modules"),
        "dry-run lists node_modules"
    );
    assert!(stdout.contains("target"), "dry-run lists target");
    assert!(stdout.contains("nothing deleted"), "dry-run footer");
}

#[test]
fn rinse_human_commit_wording() {
    let h = harness();
    let app = trash_project(&h.projects_root());

    let assert = h
        .cmd()
        .args(["rinse", "--project"])
        .arg(&app)
        .args(["--yes"])
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    // Wording from spec §4; Dev may adjust the exact phrasing.
    assert!(stdout.contains("Rinse complete"), "commit header");
    assert!(stdout.contains("Removed"), "commit reports removed count");
}
