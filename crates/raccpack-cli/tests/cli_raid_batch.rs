//! Integration tests for `racc raid --root` batch mode.
//!
//! Spec: A4 batch raid CLI. Tests cover:
//! 1. `--root` discovers projects and raids them (dry-run, two projects)
//! 2. `--only` filters to matching projects
//! 3. `--limit` caps the number of projects raided
//! 4. `--project` and `--root` together → conflict (exit 1)
//! 5. Neither `--project` nor `--root` → error (exit 1)

use std::fs;
use std::path::PathBuf;

use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// A self-contained test environment with isolated HOME / XDG cache.
struct Harness {
    work: TempDir,
    cache_home: PathBuf,
    config_file: PathBuf,
}

fn harness() -> Harness {
    let work = TempDir::new().expect("create work dir");
    let cache_home = work.path().join("xdg-cache");
    fs::create_dir_all(&cache_home).expect("create cache home");
    let config_file = work.path().join("empty-config.toml");
    fs::write(&config_file, "").expect("write empty config");
    Harness {
        work,
        cache_home,
        config_file,
    }
}

impl Harness {
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

    fn projects_root(&self) -> PathBuf {
        let root = self.work.path().join("projects");
        fs::create_dir_all(&root).expect("create projects root");
        root
    }

    fn den(&self) -> PathBuf {
        self.work.path().join("den")
    }
}

/// Create a file, creating parent dirs first.
fn write(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("has parent")).expect("create dirs");
    fs::write(&path, content).expect("write file");
}

/// Create a minimal project with a marker so `find_candidates` discovers it.
fn create_project(root: &std::path::Path, name: &str) {
    let dir = root.join(name);
    write(
        &dir,
        "Cargo.toml",
        &format!("[package]\nname = \"{name}\"\n"),
    );
    write(&dir, "src/main.rs", "fn main() {}\n");
}

// --- 1: --root dry-run discovers 2 projects and shows batch summary --------

#[test]
fn raid_root_dry_run_two_projects() {
    let h = harness();
    let root = h.projects_root();
    create_project(&root, "alpha");
    create_project(&root, "beta");
    let den = h.den();

    let assert = h
        .cmd()
        .args(["raid", "--root"])
        .arg(&root)
        .args(["--den"])
        .arg(&den)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        stdout.contains("Batch: 2 ok"),
        "must show batch summary with 2 ok, got:\n{stdout}"
    );
    assert!(
        stdout.contains("alpha"),
        "must mention first project, got:\n{stdout}"
    );
    assert!(
        stdout.contains("beta"),
        "must mention second project, got:\n{stdout}"
    );
    assert!(
        !den.join("secrets").exists(),
        "dry-run must not create den artifacts"
    );
}

// --- 2: --only filters to matching projects --------------------------------

#[test]
fn raid_root_only_filters() {
    let h = harness();
    let root = h.projects_root();
    create_project(&root, "alpha");
    create_project(&root, "beta");
    let den = h.den();

    let assert = h
        .cmd()
        .args(["raid", "--root"])
        .arg(&root)
        .args(["--only", "alpha", "--den"])
        .arg(&den)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        stdout.contains("Batch: 1 ok"),
        "must show 1 ok after filtering, got:\n{stdout}"
    );
    assert!(
        stdout.contains("alpha"),
        "must mention the matched project, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("beta"),
        "must not contain the filtered-out project, got:\n{stdout}"
    );
}

// --- 3: --limit caps the number of projects --------------------------------

#[test]
fn raid_root_limit() {
    let h = harness();
    let root = h.projects_root();
    create_project(&root, "alpha");
    create_project(&root, "beta");
    let den = h.den();

    let assert = h
        .cmd()
        .args(["raid", "--root"])
        .arg(&root)
        .args(["--limit", "1", "--den"])
        .arg(&den)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        stdout.contains("Batch: 1 ok"),
        "must show 1 ok with limit 1, got:\n{stdout}"
    );
    // Only one project should appear in the per-project lines
    let project_lines: Vec<&str> = stdout.lines().filter(|l| l.contains("— ok")).collect();
    assert_eq!(
        project_lines.len(),
        1,
        "exactly one project line expected, got {project_lines:?}"
    );
}

// --- 4: --project and --root together → conflict (exit 1) ------------------

#[test]
fn raid_project_and_root_conflicts() {
    let h = harness();
    let root = h.projects_root();
    let app = root.join("app");
    fs::create_dir_all(&app).expect("create app dir");
    h.cmd()
        .args(["raid", "--project"])
        .arg(&app)
        .args(["--root"])
        .arg(&root)
        .assert()
        .failure();
}

// --- 5: neither --project nor --root → error (exit 1) ----------------------

#[test]
fn raid_neither_project_nor_root() {
    let h = harness();
    let den = h.den();

    h.cmd()
        .args(["raid", "--den"])
        .arg(&den)
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("required").or(predicate::str::contains(
                "exactly one of --project or --root",
            )),
        );
}
