//! Integration tests for A4.1 — `ProcessGitClient` against a real `git`.
//!
//! Covers the real-git half of docs/alpha/a4/a4.1-git-client.md §4: porcelain
//! mapping (Tracked / Modified / Untracked / Ignored), and soft-fail when the
//! git binary is missing — the client returns `Err`, while `dig_with_git`
//! degrades to `Ok` with every `git_status == None`.
//!
//! House rule: every test that shells out to real git is `#[ignore]`d;
//! run them explicitly with `cargo test -p raccpack-core --test git_process -- --ignored`.
//! The defaults test never spawns git and stays hermetic/unignored.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use raccpack_core::{
    dig_with_git, AppContext, DigOptions, GitClient, NullProgress, ProcessGitClient, RaccConfig,
    RunMode,
};
use tempfile::TempDir;

/// A deterministic AWS-style access key id (matches the `aws_access_key` prefix).
const AWS_ACCESS_KEY: &str = "AKIAABCDEFGHIJKLMNOPQRST";

// --- Test helpers -----------------------------------------------------------

/// Run `git <args>` inside `repo`, panicking with stderr on failure.
fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("spawn git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create an empty hermetic git repo with a local committer identity.
fn init_repo() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create repo dir");
    let repo = temp.path().to_path_buf();
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "Test"]);
    run_git(&repo, &["config", "commit.gpgsign", "false"]);
    (temp, repo)
}

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    std::fs::write(&path, contents).expect("write fixture file");
    path
}

/// Build an `AppContext` from a config pointing at `root`.
fn ctx_for(root: &Path) -> AppContext {
    let den = root.parent().expect("scan root has a parent").join("den");
    let config = RaccConfig::default()
        .with_scan_root(root)
        .with_den_dir(&den);
    AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config")
}

/// Default dig options scanning the whole scan root with content enabled.
fn dig_options(project: Option<PathBuf>, find_repeated: bool, scan_content: bool) -> DigOptions {
    DigOptions {
        project,
        find_repeated,
        scan_content,
        use_heuristics: None,
    }
}

// --- Case 3: porcelain mapping on a real repo --------------------------------

#[test]
#[ignore = "requires a real git binary; run with --ignored"]
fn process_files_status_maps_tracked_modified_untracked() {
    let (_temp, repo) = init_repo();

    let committed = write(&repo, "committed.txt", "stable\n");
    run_git(&repo, &["add", "committed.txt"]);
    run_git(&repo, &["commit", "-m", "init"]);

    let edited = write(&repo, "edited.txt", "first\n");
    run_git(&repo, &["add", "edited.txt"]);
    run_git(&repo, &["commit", "-m", "second"]);
    write(&repo, "edited.txt", "first\nsecond\n");

    let env = write(&repo, ".env", &format!("PASSWORD={AWS_ACCESS_KEY}\n"));

    let client = ProcessGitClient::new();
    let statuses = client
        .files_status(&repo, &[committed.clone(), edited.clone(), env.clone()])
        .expect("files_status on a real repo");

    assert_eq!(
        statuses.get(&committed),
        Some(&raccpack_core::GitFileStatus::Tracked),
        "a committed, untouched file is tracked-clean"
    );
    assert_eq!(
        statuses.get(&edited),
        Some(&raccpack_core::GitFileStatus::Modified),
        "a committed file changed afterwards is modified"
    );
    assert_eq!(
        statuses.get(&env),
        Some(&raccpack_core::GitFileStatus::Untracked),
        "a never-added .env is untracked"
    );

    assert!(
        client.is_repo(&repo).expect("is_repo on a real repo"),
        "a repo with .git must be recognized"
    );
}

// --- Case 4: ignored via .gitignore -------------------------------------------

#[test]
#[ignore = "requires a real git binary; run with --ignored"]
fn process_files_status_reports_ignored_env() {
    let (_temp, repo) = init_repo();
    let gitignore = write(&repo, ".gitignore", ".env\n");
    run_git(&repo, &["add", ".gitignore"]);
    run_git(&repo, &["commit", "-m", "init"]);
    let _ = gitignore;

    let env = write(&repo, ".env", "PASSWORD=supersecretvalue123\n");

    let client = ProcessGitClient::new();
    let statuses = client
        .files_status(&repo, std::slice::from_ref(&env))
        .expect("files_status on a real repo");

    assert_eq!(
        statuses.get(&env),
        Some(&raccpack_core::GitFileStatus::Ignored),
        "an .env matched by .gitignore must be reported as ignored"
    );
}

// --- Case 5 (process half): missing binary => Err, dig soft-fails -------------

#[test]
#[ignore = "requires a real git binary; run with --ignored"]
fn process_missing_binary_errors_but_dig_soft_fails() {
    let (_temp, repo) = init_repo();
    let tracked = write(&repo, "notes.txt", "hello\n");
    run_git(&repo, &["add", "notes.txt"]);
    run_git(&repo, &["commit", "-m", "init"]);
    write(&repo, ".env", &format!("{AWS_ACCESS_KEY}\n"));

    let broken = ProcessGitClient {
        git_binary: PathBuf::from("/nonexistent/git"),
        timeout: Duration::from_secs(5),
    };

    // The client itself reports errors instead of panicking.
    assert!(
        broken.is_repo(&repo).is_err(),
        "a missing git binary must yield Err from is_repo"
    );
    assert!(
        broken.files_status(&repo, &[tracked]).is_err(),
        "a missing git binary must yield Err from files_status"
    );

    // dig over the actual repo still succeeds: Ok with every git_status None.
    let ctx = ctx_for(&repo);
    let mut progress = NullProgress;
    let result = dig_with_git(
        &ctx,
        &dig_options(None, false, true),
        &mut progress,
        &broken,
    )
    .expect("dig must not fail when the git binary is missing");

    assert!(!result.files.is_empty());
    assert!(
        result.files.iter().all(|f| f.git_status.is_none()),
        "with a broken git client every git_status must stay None"
    );
}

// --- Defaults (hermetic, no git invocation) -----------------------------------

#[test]
fn process_client_defaults_are_git_on_path_and_30s() {
    let client = ProcessGitClient::new();
    assert_eq!(
        client.git_binary,
        PathBuf::from("git"),
        "default binary must resolve git from PATH"
    );
    assert_eq!(client.timeout, Duration::from_secs(30));
}
