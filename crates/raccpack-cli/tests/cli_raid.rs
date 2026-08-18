//! Integration tests for the A3.2 CLI command `racc raid`.
//!
//! Spec: `docs/alpha/a3/a3.2-progress.md` (§4 CLI progress UX, §5 tests) and
//! the A3.2 Test brief (mandatory cases).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir`. The den is always passed explicitly via `--den` so the real
//! `~/.raccpack/den` is never touched.
//!
//! The base [`Harness::cmd`] removes `RACCPACK_PASSPHRASE` from the child env
//! and pins the child stdin to [`std::process::Stdio::null`], so no run can
//! ever block on an interactive tty prompt.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

/// Distinctive secret value used in the fixture. It must NEVER appear in any
/// output — human or JSON — of a `racc raid` run.
const SECRET_VALUE: &str = "SUPER-SECRET-RAID-PROGRESS-VALUE";

/// Age binary header (first bytes of every age-encrypted file).
const AGE_MAGIC: &[u8] = b"age-encryption.org/v1";

/// A test passphrase for `--yes` runs.
const PASSPHRASE: &str = "raccpack a3.2 cli raid test passphrase";

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
    /// pinned to null, so a Commit test that forgets passphrase/env can never
    /// hang on an interactive prompt.
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

    /// A fresh, empty directory usable as the den.
    fn den(&self) -> PathBuf {
        self.work.path().join("den")
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

/// Recursively collect all `.age` files under `root` (missing root → empty).
fn collect_age_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
                if ext.as_deref() == Some("age") {
                    files.push(path);
                }
            }
        }
    }
    files
}

/// Recursively collect all `*.tar.zst` files under `root` (missing root → empty).
fn collect_pack_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() && path.to_string_lossy().ends_with(".tar.zst") {
                files.push(path);
            }
        }
    }
    files
}

/// Build a generic `app` project fixture: sources (packable), a name-denied
/// secret `.env` and a `node_modules/` trash dir (rinse target).
fn raid_project(root: &Path) -> PathBuf {
    let app = root.join("app");
    write(&app, "Cargo.toml", "[package]\nname = \"app\"\n");
    write(&app, "src/main.rs", "fn main() {}\n");
    write(&app, ".env", &format!("PASSWORD={SECRET_VALUE}\n"));
    write(&app, "node_modules/pkg/index.js", "module.exports = 1;\n");
    app
}

// --- Case 1: dry-run by default ------------------------------------------------

#[test]
fn cli_raid_dry_run_writes_nothing_and_shows_progress() {
    let h = harness();
    let app = raid_project(&h.projects_root());
    let den = h.den();

    // No `--yes`, no passphrase: default mode is DryRun.
    let assert = h
        .cmd()
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    for line in ["→ stash:", "→ rinse:", "→ pack:", "→ move:"] {
        assert!(
            stdout.contains(line),
            "human output must show the phase line `{line}`, got:\n{stdout}"
        );
    }
    assert!(stdout.contains("Success"), "final line must say Success");

    assert!(
        !den.join("secrets").exists(),
        "dry-run must not create den/secrets/"
    );
    assert!(
        !den.join("packs").exists(),
        "dry-run must not create den/packs/"
    );
}

// --- Case 2: --json dry-run is silent about progress ---------------------------

#[test]
fn cli_raid_json_dry_run_prints_only_the_result() {
    let h = harness();
    let app = raid_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(true),
        "default run must report dry_run == true"
    );
    assert!(
        json["success"].as_bool().is_some(),
        "RaidResult must carry `success`"
    );
    let stages = json["stages"].as_array().expect("stages is an array");
    assert_eq!(stages.len(), 4, "stash/rinse/pack/move stages");

    let stdout = stdout_str(&assert);
    assert!(
        !stdout.contains("→ "),
        "JSON mode must not render progress lines, got:\n{stdout}"
    );
}

// --- Case 3: commit with env passphrase places artifacts -----------------------

#[test]
fn cli_raid_commit_places_age_and_pack_and_applies_phases() {
    let h = harness();
    let app = raid_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", PASSPHRASE)
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    assert!(stdout.contains("Success"), "commit must end with Success");
    assert!(
        stdout.contains("→ stash:"),
        "commit must still render progress lines"
    );

    let archives = collect_age_files(&den);
    assert_eq!(
        archives.len(),
        1,
        "one .age must be written under den/secrets/: {archives:?}"
    );
    let bytes = fs::read(&archives[0]).expect("read .age bytes");
    assert!(
        bytes.starts_with(AGE_MAGIC),
        "stash artifact must be age-encrypted"
    );

    let packs = collect_pack_files(&den);
    assert_eq!(
        packs.len(),
        1,
        "one .tar.zst must be written under den/packs/: {packs:?}"
    );

    assert!(
        !app.join(".env").exists(),
        "remove_sources defaults to true in a raid commit"
    );
    assert!(
        !app.join("node_modules").exists(),
        "rinse must remove node_modules/"
    );
    assert!(
        app.join("Cargo.toml").is_file(),
        "normal project files must survive"
    );
}

// --- Case 4: --yes --dry-run → dry-run wins ------------------------------------

#[test]
fn cli_raid_dry_run_wins_over_yes() {
    let h = harness();
    let app = raid_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", PASSPHRASE)
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--dry-run", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(true),
        "--dry-run must win over --yes"
    );

    assert!(
        !den.join("secrets").exists(),
        "--dry-run must not create den/secrets/"
    );
    assert!(
        !den.join("packs").exists(),
        "--dry-run must not create den/packs/"
    );
}

// --- Case 5: missing --project is rejected -------------------------------------

#[test]
fn cli_raid_missing_project_is_rejected() {
    let h = harness();

    // `--project` is required, so clap must reject the invocation.
    h.cmd()
        .args(["raid", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// --- Case 6: smoke no-panic + no raw secret in human output --------------------

#[test]
fn cli_raid_human_output_never_leaks_raw_secret() {
    let h = harness();
    let app = raid_project(&h.projects_root());
    let den = h.den();

    // Human dry-run smoke: no panic with CliProgress, no raw value anywhere.
    let assert = h
        .cmd()
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    assert!(
        !stdout.contains(SECRET_VALUE),
        "human output must never contain the raw secret value"
    );
}
