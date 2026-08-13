//! Integration tests for the M4.4 CLI command `racc pack`.
//!
//! Spec: `docs/mvp/m4/m4.4-cli-pack-e2e.md` (§3 clap, §4 flow, §5 text/JSON
//! output, §6 tests, §7 E2E checklist).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir`, so pack can never read or write the developer's real
//! `~/.config/raccpack`, `~/.cache/raccpack` or `~/.raccpack/den`. The den is
//! also passed explicitly via `--den` on every run.
//!
//! NOTE: the `pack` subcommand is being implemented in parallel by the Dev
//! agent. Until that lands these tests may not compile or pass; they encode the
//! behaviour from the spec (§6 tests + §7 E2E check) and are stitched to the
//! code at acceptance. Assertions that depend on the exact human-output wording
//! are marked inline.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// A self-contained test environment (same isolation pattern as cli_dig.rs).
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

/// Build a generic `app` project fixture under `projects_root` with a Rust
/// source file and a name-deny secret `.env`.
fn app_project(root: &Path) -> PathBuf {
    let app = root.join("app");
    write(&app, "Cargo.toml", "[package]\nname = \"app\"\n");
    write(&app, "src/main.rs", "fn main() {}\n");
    write(&app, ".env", "FOO=bar\n");
    app
}

/// Decode a `.tar.zst` archive and return the relative entry names.
///
/// Streamed through zstd + tar. Skipping pax/global extensions is handled by
/// iterating regular entries only (the project never produces them, but we
/// ignore any header-extension entries defensively).
fn unpack_names(path: &Path) -> Vec<String> {
    let bytes = fs::read(path).expect("read archive bytes");
    let decoder =
        zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes)).expect("zstd decode header");
    let mut archive = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for entry in archive.entries().expect("read tar entries") {
        let mut entry = entry.expect("valid tar entry");
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut _content = Vec::new();
        Read::read_to_end(&mut entry, &mut _content).expect("read entry contents");
        if !name.is_empty() {
            out.push(name);
        }
    }
    out
}

/// Recursively collect all `.tar.zst` files under `root` (missing root → empty).
fn collect_archives(root: &Path) -> Vec<PathBuf> {
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
                if ext.as_deref() == Some("zst") {
                    files.push(path);
                }
            }
        }
    }
    files
}

// --- Case 1: clap parse pack flags (behavioural check via --help) ------------

#[test]
fn pack_help_lists_all_pack_flags() {
    let h = harness();

    h.cmd()
        .args(["pack", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--den"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--no-content-deny"))
        .stdout(predicate::str::contains("--zstd-level"))
        .stdout(predicate::str::contains("--output-name"))
        .stdout(predicate::str::contains("--root"));
}

// --- Case 2: missing --project is rejected -----------------------------------

#[test]
fn pack_missing_project_is_rejected_by_clap() {
    let h = harness();
    let den = h.den();

    // `--project` is required (spec §3), so clap must reject the invocation.
    // The exact code is clap's parse-error exit code (2); assert `.failure()`
    // plus a non-empty stderr so the test stays stable across clap versions.
    h.cmd()
        .args(["pack", "--den"])
        .arg(&den)
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// --- Case 3: dry-run leaves the den untouched --------------------------------

#[test]
fn pack_dry_run_leaves_den_untouched() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--dry-run"])
        .assert()
        .success();

    assert!(
        !den.join("packs").exists(),
        "dry-run must not create packs/ under the den"
    );
    let archives = collect_archives(&den.join("packs"));
    assert!(
        archives.is_empty(),
        "dry-run must create no archive: {archives:?}"
    );

    let stdout = stdout_str(&assert);
    // Wording from spec §5; Dev may adjust the exact phrasing.
    assert!(stdout.contains("dry-run"), "human output signals dry-run");
}

// --- Case 4: --dry-run wins over --yes ----------------------------------------

#[test]
fn pack_dry_run_wins_over_yes() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    h.cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--dry-run", "--yes"])
        .assert()
        .success();

    // Spec §4/§3 conflict resolution: dry-run has priority, so no archive.
    let archives = collect_archives(&den.join("packs"));
    assert!(
        archives.is_empty(),
        "--dry-run must win over --yes: {archives:?}"
    );
}

// --- Case 5: commit creates an archive under packs/{yyyy}/{mm}/ ---------------

#[test]
fn pack_commit_creates_archive_and_excludes_env() {
    let h = harness();
    let den = h.den();
    let app = app_project(&h.projects_root());

    let assert = h
        .cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .success();

    // Human output names the artifact (spec §5; Dev may adjust the wording).
    let stdout = stdout_str(&assert);
    assert!(
        stdout.contains("Output:"),
        "commit report names the artifact"
    );

    // Exactly one archive under packs/<yyyy>/<mm>/.
    let archives = collect_archives(&den.join("packs"));
    assert_eq!(
        archives.len(),
        1,
        "exactly one archive must be written: {archives:?}"
    );
    let archive = &archives[0];
    let rel = archive
        .strip_prefix(den.join("packs"))
        .expect("archive under packs/");
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(parts.len(), 3, "packs/{{yyyy}}/{{mm}}/name, got {rel:?}");
    assert!(
        parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit()),
        "year must be yyyy: {:?}",
        parts[0]
    );
    assert!(
        parts[1].len() == 2 && parts[1].chars().all(|c| c.is_ascii_digit()),
        "month must be mm: {:?}",
        parts[1]
    );

    // `.den-version` is written with the current DEN_VERSION value.
    let version = fs::read_to_string(den.join(".den-version")).expect(".den-version exists");
    assert_eq!(
        version.trim(),
        raccpack_core::DEN_VERSION,
        "den version marker"
    );
    assert!(den.join("README.txt").is_file(), "README.txt must exist");

    // Archive contents: the three packed files, but never `.env`.
    let names = unpack_names(archive);
    assert!(names.iter().any(|n| n == "Cargo.toml"), "{names:?}");
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");
    assert!(
        !names.iter().any(|n| n == ".env"),
        "`.env` must not be archived: {names:?}"
    );
}

// --- Case 6: --json commit output ----------------------------------------------

#[test]
fn pack_json_commit_reports_skipped_env() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("source").is_some(), "`source` key present");
    assert!(json.get("output").is_some(), "`output` key present");
    let size = json["size_bytes"].as_u64().expect("size_bytes is a number");
    assert!(size > 0, "committed archive has positive size");
    let files = json["file_count"].as_u64().expect("file_count is a number");
    assert!(files > 0, "committed archive packs files");
    let skipped = json["skipped_secret_files"]
        .as_u64()
        .expect("skipped_secret_files is a number");
    assert!(skipped >= 1, "the name-denied `.env` must be skipped");
    assert!(
        json["dry_run"].as_bool() == Some(false),
        "commit must report dry_run == false"
    );

    let output = json["output"].as_str().expect("output is a string");
    assert!(
        output.ends_with(".tar.zst"),
        "output must end with .tar.zst: {output}"
    );
    assert!(
        Path::new(output).starts_with(&den),
        "output must live under the den: {output}"
    );
    assert!(Path::new(output).is_file(), "output must exist on disk");
}

// --- Case 7: --json dry-run output ----------------------------------------------

#[test]
fn pack_json_dry_run_reports_nothing_written() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--dry-run", "--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(
        json["dry_run"].as_bool() == Some(true),
        "dry-run must report dry_run == true"
    );
    assert_eq!(
        json["size_bytes"].as_u64(),
        Some(0),
        "dry-run must report zero size"
    );
    assert_eq!(
        json["file_count"].as_u64(),
        Some(0),
        "dry-run must report zero files"
    );

    let output = json["output"].as_str().expect("output is a string");
    assert!(
        output.ends_with(".tar.zst"),
        "output must still be a plausible .tar.zst path: {output}"
    );
    assert!(
        Path::new(output).starts_with(&den),
        "output must still live under the den: {output}"
    );
    assert!(
        !Path::new(output).exists(),
        "dry-run output must not exist on disk"
    );
}

// --- Case 8: missing project dir → exit 1 --------------------------------------

#[test]
fn pack_missing_project_dir_fails_with_stderr() {
    let h = harness();
    let den = h.den();

    h.cmd()
        .args(["pack", "--project", "/no/such/path", "--den"])
        .arg(&den)
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

// --- Case 9: --output-name and --zstd-level are honored ------------------------

#[test]
fn pack_output_name_places_named_archive() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    h.cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--output-name", "snapshot"])
        .assert()
        .success();

    let archives = collect_archives(&den.join("packs"));
    assert_eq!(archives.len(), 1, "exactly one named archive: {archives:?}");
    let file_name = archives[0]
        .file_name()
        .expect("has a file name")
        .to_string_lossy()
        .into_owned();
    assert_eq!(file_name, "snapshot.tar.zst", "named artifact created");
}

#[test]
fn pack_zstd_level_keeps_archive_readable() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    h.cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--zstd-level", "19"])
        .assert()
        .success();

    let archives = collect_archives(&den.join("packs"));
    assert_eq!(archives.len(), 1, "one archive written: {archives:?}");
    // The archive must still be a valid tar.zst stream (Dev may adjust wording).
    let names = unpack_names(&archives[0]);
    assert!(
        names.iter().any(|n| n == "src/main.rs"),
        "compressed archive must remain readable: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == ".env"),
        "`.env` must still be excluded: {names:?}"
    );
}

#[test]
fn pack_invalid_output_name_fails() {
    let h = harness();
    let app = app_project(&h.projects_root());
    let den = h.den();

    // A name containing `/` is rejected by the core validator (Error::Other).
    h.cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--output-name", "bad/name"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());
}

// --- Case 10: spec §7 E2E checklist as one test ---------------------------------

#[test]
fn pack_e2e_mvp_checklist() {
    let h = harness();
    let root = h.projects_root();
    let app = root.join("app");
    write(&app, "Cargo.toml", "[package]\nname = \"app\"\n");
    write(&app, "src/main.rs", "fn main() {}\n");
    write(&app, ".env", "FOO=bar\n");
    write(&app, "notes.txt", "hello\n");
    let den = h.den();

    h.cmd()
        .args(["pack", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .success();

    // .tar.zst exists under packs/yyyy/mm/.
    let archives = collect_archives(&den.join("packs"));
    assert!(
        !archives.is_empty(),
        "pack --yes must create a .tar.zst: {archives:?}"
    );

    // .den-version == 1.
    let version = fs::read_to_string(den.join(".den-version")).expect(".den-version exists");
    assert_eq!(
        version.trim(),
        raccpack_core::DEN_VERSION,
        "den version must be 1"
    );

    // Archive has notes.txt + src/main.rs, never .env.
    let archive = &archives[0];
    let names = unpack_names(archive);
    assert!(names.iter().any(|n| n == "notes.txt"), "{names:?}");
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");
    assert!(
        !names.iter().any(|n| n == ".env"),
        "`.env` must not be archived: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "src/main.rs/../.env"),
        "no path-escaped secret variants: {names:?}"
    );
}
