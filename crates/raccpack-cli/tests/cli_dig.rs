//! Integration tests for the M3.4 CLI command `racc dig`.
//!
//! Spec: `docs/mvp/m3/m3.4-cli-dig.md` (§3 clap, §4 flow, §5 text/JSON output,
//! §6 tests, §8 DoD).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir`, so dig can never read or write the developer's real
//! `~/.config/raccpack`, `~/.cache/raccpack` or `~/.raccpack/den`.
//!
//! NOTE: the `dig` subcommand is being implemented in parallel by the Dev
//! agent. Until that lands these tests may not compile or pass; they encode the
//! behaviour from the spec (§6 tests + §8 DoD) and are stitched to the code at
//! acceptance. Assertions that depend on the exact human-output wording are
//! marked inline.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

/// A raw AWS access key used as fixture content. Must never appear in stdout —
/// dig only ever prints the masked form `AKIA…ST`.
const AWS_TOKEN: &str = "AKIAABCDEFGHIJKLMNOPQRST";

// --- Test helpers -----------------------------------------------------------

/// A self-contained test environment (same isolation pattern as cli_sniff.rs).
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

// --- Case 1: clap parse dig flags (behavioural check via --help) -------------

#[test]
fn dig_help_lists_all_dig_flags() {
    let h = harness();

    h.cmd()
        .args(["dig", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--no-content"))
        .stdout(predicate::str::contains("--repeated"))
        .stdout(predicate::str::contains("--fail-on"))
        .stdout(predicate::str::contains("--max-depth"));
}

// --- Case 2: `.env` (High only) vs --fail-on ---------------------------------

#[test]
fn dig_env_high_exits_zero_with_default_policy() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "FOO=bar\n");

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .success()
        .code(0);
}

#[test]
fn dig_env_high_exits_two_with_fail_on_high() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "FOO=bar\n");

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--fail-on", "high"])
        .assert()
        .failure()
        .code(2);
}

// --- Case 3: Critical findings fail with the default policy ------------------

#[test]
fn dig_critical_content_exits_two_with_default_policy() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .failure()
        .code(2);
}

#[test]
fn dig_critical_filename_exits_two_with_default_policy() {
    let h = harness();
    let root = h.scan_root();
    write(
        &root,
        "id_rsa",
        "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAA==\n",
    );

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .failure()
        .code(2);
}

// --- Case 4: --json produces a well-formed DigResult, no raw values ----------

#[test]
fn dig_json_output_is_well_formed_and_hides_raw_values() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "password=supersecret123\n");
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    let assert = h
        .cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--json", "--fail-on", "ignore"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("root").is_some(), "top-level `root` present");
    assert!(json.get("files").is_some(), "top-level `files` present");
    assert!(
        json.get("repeated").is_some(),
        "top-level `repeated` present"
    );
    assert!(
        json.get("duration_ms").is_some(),
        "top-level `duration_ms` present"
    );
    assert!(
        json.get("files_scanned").is_some(),
        "top-level `files_scanned` present"
    );

    assert_eq!(
        json["root"].as_str(),
        Some(root.to_str().expect("root is valid UTF-8")),
        "root is the scan root"
    );
    assert_eq!(json["files_scanned"].as_u64(), Some(2), "two files walked");

    let files = json["files"].as_array().expect("files must be an array");
    assert_eq!(files.len(), 2, "both fixtures must be findings");

    let env = files
        .iter()
        .find(|f| f["path"].as_str().is_some_and(|p| p.ends_with(".env")))
        .expect(".env finding is present");
    assert_eq!(env["risk"].as_str(), Some("High"), ".env stays High");

    let creds = files
        .iter()
        .find(|f| f["path"].as_str().is_some_and(|p| p.ends_with("creds.txt")))
        .expect("creds.txt finding is present");
    assert_eq!(
        creds["risk"].as_str(),
        Some("Critical"),
        "AWS content upgrades to Critical"
    );

    assert_eq!(
        json["repeated"]
            .as_array()
            .expect("repeated must be an array")
            .len(),
        0,
        "no --repeated flag means no repeated aggregation"
    );

    let stdout = stdout_str(&assert);
    assert!(
        !stdout.contains(AWS_TOKEN),
        "raw AWS token must not appear in stdout"
    );
    assert!(
        !stdout.contains("supersecret123"),
        "raw password must not appear in stdout"
    );
}

// --- Case 5: --no-content is filename-only ------------------------------------

#[test]
fn dig_no_content_skips_content_only_findings() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));
    write(&root, ".env", "FOO=bar\n");

    // Without --no-content the AWS token inside creds.txt is Critical → exit 2.
    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .failure()
        .code(2);

    // With --no-content only the `.env` filename (High) remains → exit 0.
    let assert = h
        .cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--no-content", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let files = json["files"].as_array().expect("files must be an array");
    assert_eq!(files.len(), 1, "content-only finding is gone");
    assert!(
        files[0]["path"]
            .as_str()
            .expect("path is a string")
            .ends_with(".env"),
        "filename finding survives --no-content"
    );
}

// --- Case 6: --fail-on ignore never fails -------------------------------------

#[test]
fn dig_fail_on_ignore_exits_zero_even_with_critical() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--fail-on", "ignore"])
        .assert()
        .success()
        .code(0);
}

// --- Case 7: missing root ------------------------------------------------------

#[test]
fn dig_missing_root_fails_with_stderr() {
    let h = harness();

    h.cmd()
        .args(["dig", "--root", "/no/such/path"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

// --- Case 8: --repeated --------------------------------------------------------

#[test]
fn dig_repeated_detects_duplicated_secret_across_files() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "a.txt", &format!("{AWS_TOKEN}\n"));
    write(&root, "b.txt", &format!("{AWS_TOKEN}\n"));

    let with_flag = parse_json(
        &h.cmd()
            .args(["dig", "--root"])
            .arg(&root)
            .args(["--json", "--fail-on", "ignore", "--repeated"])
            .assert()
            .success(),
    );
    let repeated = with_flag["repeated"]
        .as_array()
        .expect("repeated must be an array");
    assert_eq!(repeated.len(), 1, "one value repeats across the two files");
    let group = &repeated[0];
    assert_eq!(
        group["count"].as_u64(),
        Some(2),
        "both files hold the value"
    );
    assert_eq!(group["risk"].as_str(), Some("Critical"));

    // repeated.paths order is deterministic but not contractual — check by set.
    let paths: Vec<&str> = group["paths"]
        .as_array()
        .expect("paths must be an array")
        .iter()
        .map(|p| p.as_str().expect("path is a string"))
        .collect();
    assert_eq!(paths.len(), 2, "both file paths are listed");
    assert!(
        paths.iter().any(|p| p.ends_with("a.txt")),
        "a.txt is listed"
    );
    assert!(
        paths.iter().any(|p| p.ends_with("b.txt")),
        "b.txt is listed"
    );

    // Without --repeated the aggregation stays empty.
    let without_flag = parse_json(
        &h.cmd()
            .args(["dig", "--root"])
            .arg(&root)
            .args(["--json", "--fail-on", "ignore"])
            .assert()
            .success(),
    );
    assert_eq!(
        without_flag["repeated"]
            .as_array()
            .expect("repeated must be an array")
            .len(),
        0,
        "no --repeated means no aggregation"
    );
}

#[test]
fn dig_repeated_human_output_mentions_repeated() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "a.txt", &format!("{AWS_TOKEN}\n"));
    write(&root, "b.txt", &format!("{AWS_TOKEN}\n"));

    let stdout = stdout_str(
        &h.cmd()
            .args(["dig", "--root"])
            .arg(&root)
            .args(["--repeated", "--fail-on", "ignore"])
            .assert()
            .success(),
    );
    assert!(
        stdout.contains("Repeated"),
        "human output signals repeated secrets"
    );
}

// --- Case 9: --project ----------------------------------------------------------

#[test]
fn dig_project_limits_scan_to_one_subdirectory() {
    let h = harness();
    let root = h.scan_root();
    let proj_a = root.join("app-a");
    let proj_b = root.join("app-b");
    write(&proj_a, ".env", "FOO=bar\n");
    write(&proj_b, "creds.txt", &format!("{AWS_TOKEN}\n"));

    let json = parse_json(
        &h.cmd()
            .args(["dig", "--root"])
            .arg(&root)
            .args(["--project"])
            .arg(&proj_a)
            .args(["--json", "--fail-on", "ignore"])
            .assert()
            .success(),
    );

    assert_eq!(
        json["root"].as_str(),
        Some(proj_a.to_str().expect("proj_a is valid UTF-8")),
        "--project becomes the scan root"
    );

    let files = json["files"].as_array().expect("files must be an array");
    assert_eq!(files.len(), 1, "only findings from the selected project");
    assert!(
        files[0]["path"]
            .as_str()
            .expect("path is a string")
            .ends_with(".env"),
        "the selected project's .env is the only finding"
    );
}

// --- Case 10: --max-depth --------------------------------------------------------

#[test]
fn dig_max_depth_limits_how_deep_secrets_are_searched() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "FOO=bar\n");
    write(&root, "nested/creds.txt", &format!("{AWS_TOKEN}\n"));

    // Default depth reaches the nested Critical file → exit 2.
    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .failure()
        .code(2);

    // --max-depth 1 only reaches files directly in root: .env (High) stays,
    // nested/creds.txt is out of reach → exit 0 with the default policy.
    let assert = h
        .cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--max-depth", "1", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let files = json["files"].as_array().expect("files must be an array");
    assert_eq!(files.len(), 1, "nested secret is not scanned at depth 1");
    assert!(
        files[0]["path"]
            .as_str()
            .expect("path is a string")
            .ends_with(".env"),
        "only the depth-1 .env remains"
    );
}

// --- Case 11: human output ---------------------------------------------------------

#[test]
fn dig_human_output_has_expected_sections_and_no_raw() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "FOO=bar\n");
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    let stdout = stdout_str(
        &h.cmd()
            .args(["dig", "--root"])
            .arg(&root)
            .args(["--fail-on", "ignore"])
            .assert()
            .success(),
    );
    // Wording is taken from spec §5; Dev may adjust the exact phrasing.
    assert!(stdout.contains("Dig root:"), "human header shows the root");
    assert!(
        stdout.contains("Files scanned:"),
        "human summary shows the scanned file count"
    );
    assert!(stdout.contains("RISK"), "human table has a RISK column");
    assert!(
        !stdout.contains(AWS_TOKEN),
        "raw AWS token must not appear in human output"
    );
}

// --- Case 12: policy-trigger stderr ------------------------------------------------

#[test]
fn dig_policy_trigger_reports_on_stderr_for_human_mode() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    // stderr wording per spec §4 step 9 ("Sensitive findings triggered exit
    // policy"); falls back to any non-empty stderr if Dev rewords it.
    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("policy"));
}

#[test]
fn dig_json_mode_keeps_stdout_clean_on_policy_failure() {
    let h = harness();
    let root = h.scan_root();
    write(&root, "creds.txt", &format!("{AWS_TOKEN}\n"));

    let assert = h
        .cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--json"])
        .assert()
        .failure()
        .code(2);

    // stdout must still be a well-formed DigResult document.
    let json = parse_json(&assert);
    let files = json["files"]
        .as_array()
        .expect("stdout must stay valid JSON");
    assert_eq!(files.len(), 1, "the critical finding is still reported");
}

// --- Bonus ------------------------------------------------------------------------

#[test]
fn dig_json_flag_is_accepted_before_the_subcommand() {
    let h = harness();
    let root = h.scan_root();
    write(&root, ".env", "FOO=bar\n");

    let json = parse_json(
        &h.cmd()
            .args(["--json", "dig", "--root"])
            .arg(&root)
            .assert()
            .success(),
    );
    assert!(
        json.get("files").is_some(),
        "global --json must produce the DigResult JSON"
    );
}

#[test]
fn dig_project_missing_directory_fails_with_stderr() {
    let h = harness();
    let root = h.scan_root();

    h.cmd()
        .args(["dig", "--root"])
        .arg(&root)
        .args(["--project", "/no/such/project"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());
}
