//! Process-level launch-arguments contract for `racc-tui` (B1.2.2).
//!
//! These tests run the real `racc-tui` binary with piped stdio, which is never
//! a TTY under `cargo test`. That is exactly the contract under test:
//!
//! * `--version` / `--help` must be handled by clap *before* any terminal init,
//!   so they work (and exit 0) without an interactive terminal.
//! * A plain non-TTY invocation must be refused cleanly with
//!   `racc-tui requires an interactive terminal` on stderr and a non-zero exit
//!   code — not a hang, not a raw-mode error.

use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Path to the built `racc-tui` binary, provided by cargo for integration tests.
const BIN: &str = env!("CARGO_BIN_EXE_racc-tui");

/// Guard timeout for a child that never exits (e.g. a TUI that bypassed the
/// non-TTY gate). `std::process` has no built-in timeout; without this a hung
/// child would stall the whole suite.
const CHILD_TIMEOUT: Duration = Duration::from_secs(20);

/// Run `racc-tui` with piped/null stdio (deterministically non-TTY) and return
/// its output, killing the child if it does not exit in time.
fn run_racc_tui(args: &[&str]) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn().expect("spawn racc-tui");
    let child = Arc::new(Mutex::new(Some(child)));
    let waiter = Arc::clone(&child);
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut guard = waiter.lock().expect("lock child");
        let child = guard.take().expect("child still present");
        let output = child.wait_with_output();
        let _ = tx.send(output);
    });

    match rx.recv_timeout(CHILD_TIMEOUT) {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => panic!("failed to read racc-tui output: {e}"),
        Err(_) => {
            if let Some(mut child) = child.lock().expect("lock child").take() {
                let _ = child.kill();
            }
            panic!("racc-tui did not exit within {CHILD_TIMEOUT:?} — launched the TUI despite non-TTY stdio");
        }
    }
}

#[test]
fn version_prints_without_a_terminal() {
    let out = run_racc_tui(&["--version"]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "--version must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("racc-tui"),
        "--version must name the binary; stdout={stdout}"
    );
    assert!(
        stdout.contains("0.4"),
        "--version must print the current '0.4.x' version; stdout={stdout}"
    );
}

#[test]
fn help_mentions_root_and_view() {
    let out = run_racc_tui(&["--help"]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "--help must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        all.contains("--root"),
        "--help must list --root; help={all}"
    );
    assert!(
        all.contains("--view"),
        "--help must list --view; help={all}"
    );
}

#[test]
fn unknown_flag_is_rejected() {
    let out = run_racc_tui(&["--badflag"]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "an unknown flag must not succeed"
    );
}

#[test]
fn nontty_requires_interactive_terminal() {
    let out = run_racc_tui(&[]);

    assert_ne!(
        out.status.code(),
        Some(0),
        "a plain non-TTY invocation must be refused"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("requires an interactive terminal"),
        "stderr must carry the non-interactive refusal; stderr={stderr}"
    );
    assert!(
        out.stdout.is_empty(),
        "non-interactive refusal must print nothing on stdout"
    );
}

#[test]
fn launch_args_do_not_bypass_nontty_gate() {
    // Parsed args succeed, but the TTY check still wins: the gate runs only
    // after `Cli::parse()` and before the event loop.
    let out = run_racc_tui(&["--root", "/tmp/foo", "--view", "projects"]);

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("requires an interactive terminal"),
        "stderr={stderr}"
    );
}
