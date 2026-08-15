//! Integration tests for the A1.4 CLI command `racc stash`.
//!
//! Spec: `docs/alpha/a1/a1.4-cli-stash.md` (§1 goal, §3 clap, §4 flow, §5
//! tests, §6 DoD).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`) against
//! fixtures it creates itself and fully isolates the child's environment:
//! `HOME`, `XDG_CACHE_HOME` and `RACCPACK_CONFIG` all point inside a fresh
//! `TempDir`, so stash can never read or write the developer's real
//! `~/.config/raccpack`, `~/.cache/raccpack` or `~/.raccpack/den`. The den is
//! also passed explicitly via `--den` on every run.
//!
//! The base [`Harness::cmd`] additionally:
//!
//! - removes `RACCPACK_PASSPHRASE` from the child env, and
//! - pins the child stdin to [`std::process::Stdio::null`].
//!
//! Together these guarantee that a Commit test which forgets to supply a
//! passphrase can NEVER hang on an interactive tty prompt: the child sees an
//! immediate EOF on stdin and must fall through to the "no tty, no env" error
//! path. (assert_cmd's public API has no `stdin` method, so the harness builds
//! the inner [`std::process::Command`] via `assert_cmd::cargo::CommandCargoExt`
//! and wraps it back with `Command::from_std`. Note that `assert_cmd` always
//! respawns with a piped stdin and — when no `.write_stdin` buffer is set —
//! delivers EOF, so the no-hang property holds either way; the explicit
//! `Stdio::null` is belt-and-suspenders.) Tests that need env set it with
//! `.env("RACCPACK_PASSPHRASE", "...")`; tests that need piped stdin override
//! with `.write_stdin("...\n")`.
//!
//! NOTE: the `stash` subcommand is being implemented in parallel by the Dev
//! agent. Until that lands these tests may not pass (clap rejects the unknown
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

/// Distinctive secret value used in the fixture. It must NEVER appear in any
/// output — human or JSON — of a `racc stash` run (spec: JSON `StashResult`
/// is raw-free; the `.age` file is encrypted).
const SECRET_VALUE: &str = "SUPER-SECRET-ONLY-IN-TEST-VALUE";

/// Age binary header (first bytes of every age-encrypted file).
const AGE_MAGIC: &[u8] = b"age-encryption.org/v1";

// --- Test helpers -----------------------------------------------------------

/// A self-contained test environment (same isolation pattern as cli_pack.rs).
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
    ///
    /// The child's `RACCPACK_PASSPHRASE` is always removed and its stdin is
    /// pinned to null, so a Commit test that forgets passphrase/env can never
    /// hang on an interactive prompt (spec §1.4: no tty + no env → error).
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

/// Build a generic `app` project fixture: `Cargo.toml`, `src/main.rs` and a
/// name-denied secret `.env` (risk High from the filename match) carrying a
/// distinctive secret value.
fn stash_project(root: &Path) -> PathBuf {
    let app = root.join("app");
    write(&app, "Cargo.toml", "[package]\nname = \"app\"\n");
    write(&app, "src/main.rs", "fn main() {}\n");
    write(&app, ".env", &format!("PASSWORD={SECRET_VALUE}\n"));
    app
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

/// Assert the path structure of a stash artifact: the file must live at
/// `den/secrets/{yyyy}/{mm}/{name}` with a 4-digit year and 2-digit month.
fn assert_artifact_layout(den: &Path, archive: &Path) {
    let rel = archive
        .strip_prefix(den.join("secrets"))
        .expect("archive under den/secrets/");
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(parts.len(), 3, "secrets/{{yyyy}}/{{mm}}/name, got {rel:?}");
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
    assert!(
        parts[2].ends_with("__secrets.age"),
        "artifact name must end with __secrets.age: {:?}",
        parts[2]
    );
}

// --- Case 1: clap parse stash flags (behavioural check via --help) ----------

#[test]
fn stash_help_lists_all_stash_flags() {
    let h = harness();

    h.cmd()
        .args(["stash", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--den"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--remove-sources"))
        .stdout(predicate::str::contains("--min-risk"))
        .stdout(predicate::str::contains("--only"))
        .stdout(predicate::str::contains("--batch-id"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--root"));
}

// --- Case 2: missing --project is rejected ----------------------------------

#[test]
fn stash_missing_project_is_rejected() {
    let h = harness();
    let den = h.den();

    // `--project` is required (spec §3), so clap must reject the invocation.
    // Assert `.failure()` plus a non-empty stderr so the test stays stable
    // across clap versions (mirrors cli_pack.rs).
    h.cmd()
        .args(["stash", "--den"])
        .arg(&den)
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// --- Case 3: dry-run writes nothing ------------------------------------------

#[test]
fn stash_dry_run_writes_nothing() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    // No `--yes`, no passphrase: DryRun never requires a passphrase.
    let assert = h
        .cmd()
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .assert()
        .success();

    assert!(
        !den.join("secrets").exists(),
        "dry-run must not create den/secrets/ (den: {})",
        den.display()
    );
    assert!(
        collect_age_files(&den).is_empty(),
        "dry-run must create no .age file: {:?}",
        collect_age_files(&den)
    );

    let stdout = stdout_str(&assert);
    // Wording from spec §4; Dev may adjust the exact phrasing.
    assert!(stdout.contains("dry-run"), "human output signals dry-run");
}

// --- Case 4: --json dry-run reports dry_run == true --------------------------

#[test]
fn stash_json_dry_run_reports_dry_run_true() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(
        json["dry_run"].as_bool() == Some(true),
        "dry-run must report dry_run == true"
    );
    let files = json["files_archived"]
        .as_u64()
        .expect("files_archived is a number");
    assert!(
        files >= 1,
        "dry-run must report the selected files: {files}"
    );
    assert_eq!(
        json["removed_sources"].as_u64(),
        Some(0),
        "dry-run never removes sources"
    );

    let archive_path = json["archive_path"]
        .as_str()
        .expect("archive_path is a string");
    assert!(
        archive_path.ends_with("__secrets.age"),
        "archive_path must end with __secrets.age: {archive_path}"
    );
    assert!(
        Path::new(archive_path).starts_with(&den),
        "archive_path must live under the den: {archive_path}"
    );

    let stdout = stdout_str(&assert);
    assert!(
        !stdout.contains(SECRET_VALUE),
        "JSON must never contain the raw secret value"
    );
}

// --- Case 5: commit with env passphrase creates a .age ------------------------

#[test]
fn stash_commit_env_passphrase_creates_age() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .success();

    // Human output names the artifact (spec §4; Dev may adjust the wording).
    let stdout = stdout_str(&assert);
    assert!(
        stdout.contains("secrets.age") || stdout.contains("secrets"),
        "commit report names the stash artifact"
    );

    // Exactly one .age under den/secrets/{yyyy}/{mm}/.
    let archives = collect_age_files(&den);
    assert_eq!(
        archives.len(),
        1,
        "exactly one .age must be written: {archives:?}"
    );
    let archive = &archives[0];
    assert_artifact_layout(&den, archive);

    // File begins with the age magic bytes.
    let bytes = fs::read(archive).expect("read .age bytes");
    assert!(
        bytes.starts_with(AGE_MAGIC),
        "file must be age-encrypted (magic {AGE_MAGIC:?}), got {:?}",
        &bytes[..bytes.len().min(AGE_MAGIC.len())]
    );

    // Source stays untouched without --remove-sources.
    assert!(
        app.join(".env").is_file(),
        "source `.env` must still exist (no --remove-sources)"
    );
}

// --- Case 6: commit with --remove-sources deletes the originals --------------

#[test]
fn stash_commit_remove_sources_deletes_originals() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--remove-sources", "--json"])
        .assert()
        .success();

    let archives = collect_age_files(&den);
    assert_eq!(archives.len(), 1, "one .age written: {archives:?}");
    assert!(
        !app.join(".env").exists(),
        "--remove-sources must delete the archived `.env`"
    );

    let json = parse_json(&assert);
    let removed = json["removed_sources"]
        .as_u64()
        .expect("removed_sources is a number");
    assert!(removed >= 1, "removed_sources must be >= 1: {removed}");
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(false),
        "commit must report dry_run == false"
    );
}

// --- Case 7: missing passphrase, non-tty, no env → exit 1 --------------------

#[test]
fn stash_missing_passphrase_non_tty_without_env_fails() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    // No RACCPACK_PASSPHRASE and stdin is null (base harness): the child must
    // not hang on an interactive prompt; it must fail with a hint naming the
    // env var. This assertion is what proves the "no tty, no env" path.
    h.cmd()
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("RACCPACK_PASSPHRASE"));

    assert!(
        collect_age_files(&den).is_empty(),
        "a failed commit must create no .age file"
    );
}

// --- Case 8: --min-risk critical filters a High-only fixture -----------------

#[test]
fn stash_min_risk_critical_filters_high_env() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    // The fixture only has a High `.env`; --min-risk critical selects nothing,
    // so the core returns Error::StashEmpty → exit 1.
    h.cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--min-risk", "critical"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());

    assert!(
        collect_age_files(&den).is_empty(),
        "nothing to stash → no .age may be written"
    );
}

// --- Case 9: --dry-run wins over --yes ----------------------------------------

#[test]
fn stash_dry_run_wins_over_yes() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    // --dry-run overrides --yes (spec §1 default DryRun, §3 conflict
    // resolution); no passphrase is required and nothing is written.
    h.cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--dry-run", "--yes"])
        .assert()
        .success();

    assert!(
        collect_age_files(&den).is_empty(),
        "--dry-run must win over --yes: {:?}",
        collect_age_files(&den)
    );
}

// --- Case 10: piped stdin passphrase works ------------------------------------

#[test]
fn stash_stdin_piped_passphrase_works() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    // No env passphrase; the single-line piped-stdin branch provides it.
    h.cmd()
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes"])
        .write_stdin("ci-passphrase-value-for-tests\n")
        .assert()
        .success();

    let archives = collect_age_files(&den);
    assert_eq!(
        archives.len(),
        1,
        "piped-stdin passphrase must produce one .age: {archives:?}"
    );
    let bytes = fs::read(&archives[0]).expect("read .age bytes");
    assert!(
        bytes.starts_with(AGE_MAGIC),
        "piped passphrase must encrypt"
    );
}

// --- Case 11: --batch-id names the artifact -----------------------------------

#[test]
fn stash_batch_id_names_artifact() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    h.cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--batch-id", "nightly"])
        .assert()
        .success();

    let archives = collect_age_files(&den);
    assert_eq!(archives.len(), 1, "one .age written: {archives:?}");
    let file_name = archives[0]
        .file_name()
        .expect("has a file name")
        .to_string_lossy()
        .into_owned();
    assert!(
        file_name.contains("nightly"),
        "--batch-id must name the artifact: {file_name}"
    );
    assert_artifact_layout(&den, &archives[0]);
}

// --- Case 12: --json commit never leaks raw content ---------------------------

#[test]
fn stash_json_commit_never_leaks_raw() {
    let h = harness();
    let app = stash_project(&h.projects_root());
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "--json"])
        .assert()
        .success();

    let stdout = stdout_str(&assert);
    assert!(
        !stdout.contains(SECRET_VALUE),
        "JSON commit output must never contain the raw secret value"
    );

    let json = parse_json(&assert);
    assert_eq!(
        json["dry_run"].as_bool(),
        Some(false),
        "commit must report dry_run == false"
    );
    let files = json["files_archived"]
        .as_u64()
        .expect("files_archived is a number");
    assert!(files >= 1, "commit must archive at least the `.env`");

    let manifest = json["manifest"].as_array().expect("manifest is an array");
    assert!(!manifest.is_empty(), "manifest must list archived files");
    for entry in manifest {
        assert!(
            entry["original_path"].is_string(),
            "manifest entry must carry original_path: {entry}"
        );
        assert!(
            entry["risk"].is_string(),
            "manifest entry must carry risk: {entry}"
        );
        assert!(
            entry["size_bytes"].is_u64(),
            "manifest entry must carry size_bytes: {entry}"
        );
    }
}

// --- Negative: missing project dir → exit 1 -----------------------------------

#[test]
fn stash_missing_project_dir_fails() {
    let h = harness();
    let den = h.den();

    h.cmd()
        .env("RACCPACK_PASSPHRASE", "env-passphrase-not-for-prod")
        .args(["stash", "--project", "/no/such/path", "--den"])
        .arg(&den)
        .args(["--yes"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not());

    assert!(
        collect_age_files(&den).is_empty(),
        "a failed run must create no .age file"
    );
}
