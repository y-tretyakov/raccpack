//! Integration tests for A4.3: tracing / `-v` verbosity logging.
//!
//! Spec: `docs/alpha/a4/a4.3-tracing-verbose.md` (§3 levels, §4 redaction,
//! §5 CLI examples, §6 tests).
//!
//! Every test spawns the real `racc` binary (via `assert_cmd`, which resolves
//! it through `CARGO_BIN_EXE_racc`; the crate is bin-only, there is no lib
//! target to link against). The base [`Harness::cmd`] fully isolates the
//! child environment:
//!
//! - `HOME`, `XDG_CACHE_HOME`, `RACCPACK_CONFIG` point inside a fresh
//!   `TempDir`, so no real user config/den/cache is ever touched;
//! - `RUST_LOG` is REMOVED, because `std::process::Command` inherits the
//!   parent environment and a developer/CI `RUST_LOG` would otherwise defeat
//!   the verbosity-flag cases;
//! - `RACCPACK_PASSPHRASE` is removed (tests that need it set it explicitly);
//! - stdin is pinned to null, so no case can hang on an interactive prompt.
//!
//! Logs are asserted on stderr, data/JSON on stdout. Because
//! `tracing-subscriber` may colourise its output with ANSI escapes even when
//! piped, every captured stream is passed through [`strip_ansi`] before
//! matching level/target markers.
//!
//! NOTE: the `-v/--verbose` flag and `logging.rs` are being implemented in
//! parallel by the Dev agent. Until that lands these tests cannot pass (clap
//! rejects the unknown flag); they encode the behaviour from the spec and are
//! stitched to the code at acceptance.
//!
//! Narrow suite command: `cargo test -p raccpack-cli --test tracing_logging`

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::prelude::*;
use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Passphrase planted in the child env for the stash invariant test. It must
/// NEVER appear in any captured stream, even at `-vv`.
const PASSPHRASE: &str = "secret-test-value-42";

/// Fake AWS secret-access-key value written into a fixture file. Its raw text
/// must NEVER appear in any captured stream, even at `-vv`.
const AWS_SECRET_VALUE: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

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
    /// `RUST_LOG` and `RACCPACK_PASSPHRASE` are always removed first: the
    /// child inherits our process env, and both variables would silently
    /// change the behaviour under test (log filtering, stash prompting).
    fn cmd(&self) -> Command {
        let mut std_cmd = std::process::Command::cargo_bin("racc").expect("locate racc binary");
        std_cmd.stdin(std::process::Stdio::null());
        let mut cmd = Command::from_std(std_cmd);
        cmd.env("HOME", self.work.path())
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("RACCPACK_CONFIG", &self.config_file)
            .env_remove("RUST_LOG")
            .env_remove("RACCPACK_PASSPHRASE");
        cmd
    }

    /// A projects root containing one minimal Rust project for sniff/dig.
    fn projects_root(&self) -> PathBuf {
        let app = self.work.path().join("projects/app");
        fs::create_dir_all(app.join("src")).expect("create project dirs");
        fs::write(app.join("Cargo.toml"), "[package]\nname = \"app\"\n").expect("write Cargo.toml");
        fs::write(app.join("src/main.rs"), "fn main() {}\n").expect("write main.rs");
        self.work.path().join("projects")
    }

    /// A fresh directory usable as the den.
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

/// Remove ANSI escape sequences (CSI `ESC [ ... final-byte`) from captured
/// output so level/target markers can be matched regardless of whether the
/// subscriber colourises its output.
fn strip_ansi(input: &str) -> String {
    #[derive(PartialEq)]
    enum State {
        Text,
        Esc,
        Csi,
    }
    let mut state = State::Text;
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match state {
            State::Text => {
                if c == '\x1b' {
                    state = State::Esc;
                } else {
                    out.push(c);
                }
            }
            State::Esc => {
                state = if c == '[' { State::Csi } else { State::Text };
            }
            State::Csi => {
                if ('@'..='~').contains(&c) {
                    state = State::Text;
                }
            }
        }
    }
    out
}

/// Decoded stdout of a finished command, ANSI-stripped.
fn stdout_str(assert: &assert_cmd::assert::Assert) -> String {
    strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout))
}

/// Decoded stderr of a finished command, ANSI-stripped.
fn stderr_str(assert: &assert_cmd::assert::Assert) -> String {
    strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr))
}

/// Combined stdout+stderr, ANSI-stripped (for leak checks over everything the
/// process could have printed).
fn combined_output(assert: &assert_cmd::assert::Assert) -> String {
    let output = assert.get_output();
    let mut all = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    all = strip_ansi(&all);
    all
}

/// Parse the captured stdout as JSON.
fn parse_json(assert: &assert_cmd::assert::Assert) -> Value {
    serde_json::from_str(&stdout_str(assert)).expect("stdout must be valid JSON")
}

/// True when the stripped stderr carries at least one tracing marker: a level
/// token or a `raccpack*` target (spec §3: subscriber prints level + target).
fn has_log_marker(stderr: &str) -> bool {
    stderr.contains("INFO") || stderr.contains("raccpack")
}

/// Spec §6.2 helper: panic unless `secret` is absent from `log`.
///
/// The panic message deliberately reports only lengths/offsets — echoing the
/// secret (or its surrounding text) would leak it into CI logs.
fn assert_no_secret_in_string(log: &str, secret: &str) {
    assert!(
        !log.contains(secret),
        "secret material ({} chars) leaked into output at byte offset {}; \
         surrounding text intentionally not printed",
        secret.len(),
        log.find(secret).unwrap_or(0),
    );
}

// --- Cases ------------------------------------------------------------------

/// Case 1: without `-v` a successful command stays quiet — no INFO/WARN/DEBUG
/// lines on stderr (spec §3: default level warn/error; §5 «обычный режим —
/// почти тихо»).
#[test]
fn default_verbosity_is_quiet_on_success() {
    let h = harness();
    let root = h.projects_root();

    let assert = h
        .cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .assert()
        .success();

    let stderr = stderr_str(&assert);
    for level in ["INFO", "WARN", "DEBUG", "TRACE"] {
        assert!(
            !stderr.contains(level),
            "default verbosity must not emit {level} log lines, got: {stderr:?}"
        );
    }
}

/// Case 2: `-v` enables info-level logging — stderr carries log output with a
/// level/target marker (spec §3: `-v` → info; §5 «фазы, пути, счётчики»).
#[test]
fn verbose_flag_enables_info_logs() {
    let h = harness();
    let root = h.projects_root();

    let assert = h
        .cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .arg("-v")
        .assert()
        .success();

    let stderr = stderr_str(&assert);
    assert!(
        !stderr.trim().is_empty(),
        "-v must produce log output on stderr"
    );
    assert!(
        has_log_marker(&stderr),
        "-v stderr must contain a level/target marker (INFO or raccpack), got: {stderr:?}"
    );
}

/// Case 3: `-vv` enables debug-level logging — stderr contains a DEBUG marker
/// (spec §3: `-vv` → debug).
#[test]
fn double_verbose_enables_debug_logs() {
    let h = harness();
    let root = h.projects_root();

    let assert = h
        .cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .args(["-vv"])
        .assert()
        .success();

    let stderr = stderr_str(&assert);
    assert!(
        stderr.contains("DEBUG"),
        "-vv stderr must contain DEBUG log lines, got: {stderr:?}"
    );
}

/// Case 4: `RUST_LOG` wins over the `-v` flag — `RUST_LOG=error` plus `-v`
/// suppresses info lines (spec §3 recommendation: RUST_LOG wins if set).
#[test]
fn rust_log_overrides_verbose_flag() {
    let h = harness();
    let root = h.projects_root();

    let assert = h
        .cmd()
        .env("RUST_LOG", "error")
        .args(["sniff", "--root"])
        .arg(&root)
        .arg("-v")
        .assert()
        .success();

    let stderr = stderr_str(&assert);
    for level in ["INFO", "DEBUG", "TRACE"] {
        assert!(
            !stderr.contains(level),
            "RUST_LOG=error must suppress {level} lines despite -v, got: {stderr:?}"
        );
    }
}

/// Case 5: with `--json -v` the machine-readable result goes to stdout (valid
/// JSON) while logs stay on stderr (spec §5: «JSON в stdout, логи в stderr»).
#[test]
fn json_output_and_verbose_logs_use_separate_streams() {
    let h = harness();
    let root = h.projects_root();

    let assert = h
        .cmd()
        .args(["--json", "-v", "sniff", "--root"])
        .arg(&root)
        .assert()
        .success();

    let value = parse_json(&assert);
    assert!(value.is_object(), "sniff JSON result must be an object");

    let stderr = stderr_str(&assert);
    assert!(
        has_log_marker(&stderr),
        "-v logs must be present on stderr next to JSON stdout, got: {stderr:?}"
    );
}

/// Case 6 (main invariant): neither the passphrase nor the raw secret text of
/// a detected file may appear anywhere in the output of a committing
/// `stash --yes -vv` run (spec §4 redaction rules).
#[test]
fn passphrase_and_secret_never_appear_in_verbose_logs() {
    let h = harness();
    let app = h.work.path().join("projects/app");
    write(
        &app,
        ".env",
        &format!(
            "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\naws_secret_access_key = \"{AWS_SECRET_VALUE}\"\n"
        ),
    );
    let den = h.den();

    let assert = h
        .cmd()
        .env("RACCPACK_PASSPHRASE", PASSPHRASE)
        .args(["stash", "--project"])
        .arg(&app)
        .args(["--den"])
        .arg(&den)
        .args(["--yes", "-vv"])
        .assert()
        .success();

    let all = combined_output(&assert);
    assert_no_secret_in_string(&all, PASSPHRASE);
    assert_no_secret_in_string(&all, AWS_SECRET_VALUE);
}

/// Case 7: smoke — maximum verbosity `-vvv` must not panic or crash a plain
/// successful run (exit code 0).
#[test]
fn trace_verbosity_does_not_panic_on_sniff() {
    let h = harness();
    let root = h.projects_root();

    h.cmd()
        .args(["sniff", "--root"])
        .arg(&root)
        .args(["-vvv"])
        .assert()
        .code(0);
}
