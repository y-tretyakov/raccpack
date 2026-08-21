//! Integration tests for A4.1 — `GitClient` mock path + `find_repo_root`.
//!
//! Covers the hermetic half of docs/alpha/a4/a4.1-git-client.md §4: a
//! programmed `MockGitClient` fills `SensitiveFile.git_status` through
//! `dig_with_git`, non-repo and erroring clients soft-fail to `None` while
//! dig stays `Ok`, and `find_repo_root` walks up to the nearest `.git`.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! Real-git coverage lives in `git_process.rs` under `#[ignore]`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use raccpack_core::{
    dig_with_git, find_repo_root, AppContext, DigOptions, DigResult, Error, GitClient,
    GitFileStatus, MockGitClient, NullProgress, RaccConfig, RunMode, SensitiveFile,
};
use tempfile::TempDir;

/// A deterministic AWS-style access key id (matches the `aws_access_key` prefix).
const AWS_ACCESS_KEY: &str = "AKIAABCDEFGHIJKLMNOPQRST";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    std::fs::write(&path, contents).expect("write fixture file");
    path
}

/// Create a workspace: a `TempDir` with an existing `projects/` scan root.
fn workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create work dir");
    let projects = temp.path().join("projects");
    std::fs::create_dir_all(&projects).expect("create projects dir");
    (temp, projects)
}

/// Build an `AppContext` from a config pointing at `root` (den is derived as a
/// sibling of the scan root so no real `~/.raccpack/den` is ever touched).
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

/// Run `dig_with_git` with a `NullProgress` sink and return the result.
fn dig_once_with(ctx: &AppContext, opts: &DigOptions, git: &dyn GitClient) -> DigResult {
    let mut progress = NullProgress;
    dig_with_git(ctx, opts, &mut progress, git).expect("dig must not fail on git problems")
}

/// Find the `SensitiveFile` for `path`, panicking with context if absent.
fn file_by_path<'a>(files: &'a [SensitiveFile], path: &Path) -> &'a SensitiveFile {
    files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no finding for {}", path.display()))
}

// --- Case 1: mock statuses fill dig git_status -------------------------------

#[test]
fn mock_statuses_fill_dig_git_status_strings() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", "PASSWORD=supersecretvalue123\n");
    let creds = write(&root, "creds.txt", &format!("{AWS_ACCESS_KEY}\n"));

    let mut statuses = HashMap::new();
    statuses.insert(env.clone(), GitFileStatus::Untracked);
    statuses.insert(creds.clone(), GitFileStatus::Modified);
    let git = MockGitClient::new()
        .with_is_repo(true)
        .with_statuses(statuses);

    let ctx = ctx_for(&root);
    let result = dig_once_with(&ctx, &dig_options(None, false, true), &git);

    assert_eq!(result.files.len(), 2, "both fixture files are findings");
    assert_eq!(
        file_by_path(&result.files, &env).git_status,
        Some("untracked".to_string()),
        "git_status must carry the as_str string, not Debug"
    );
    assert_eq!(
        file_by_path(&result.files, &creds).git_status,
        Some("modified".to_string())
    );
}

#[test]
fn git_file_status_as_str_is_snake_case_and_serde_stable() {
    let pairs = [
        (GitFileStatus::Tracked, "tracked"),
        (GitFileStatus::Untracked, "untracked"),
        (GitFileStatus::Ignored, "ignored"),
        (GitFileStatus::Modified, "modified"),
        (GitFileStatus::Staged, "staged"),
        (GitFileStatus::Deleted, "deleted"),
        (GitFileStatus::Unknown, "unknown"),
    ];
    for (status, expected) in pairs {
        assert_eq!(status.as_str(), expected, "as_str must be snake_case");
        let json = serde_json::to_string(&status).expect("serialize GitFileStatus");
        assert_eq!(
            json,
            format!("\"{expected}\""),
            "JSON serialization must be the stable snake_case string"
        );
    }
}

// --- Case 2: non-repo => all None, dig Ok ------------------------------------

#[test]
fn mock_non_repo_yields_no_git_status_and_dig_ok() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    // Statuses are programmed but must be ignored because is_repo == false.
    let mut statuses = HashMap::new();
    statuses.insert(env.clone(), GitFileStatus::Tracked);
    let git = MockGitClient::new()
        .with_is_repo(false)
        .with_statuses(statuses);

    let ctx = ctx_for(&root);
    let result = dig_once_with(&ctx, &dig_options(None, false, true), &git);

    assert!(!result.files.is_empty());
    assert!(
        result.files.iter().all(|f| f.git_status.is_none()),
        "outside a repo every git_status must stay None"
    );
}

// --- Case 5 (mock half): client error => soft-fail, dig Ok -------------------

#[test]
fn mock_client_error_soft_fails_dig_ok_all_none() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    let git = MockGitClient::new().with_error(Error::Other {
        message: "git subprocess exploded".to_string(),
    });

    let ctx = ctx_for(&root);
    let result = dig_once_with(&ctx, &dig_options(None, false, true), &git);

    assert!(!result.files.is_empty());
    assert!(
        result.files.iter().all(|f| f.git_status.is_none()),
        "a failing GitClient must degrade to None, never fail dig"
    );
}

// --- Case 6: find_repo_root ---------------------------------------------------

#[test]
fn find_repo_root_walks_up_to_nearest_git_dir() {
    let temp = TempDir::new().expect("create repo dir");
    let repo = temp.path();
    std::fs::create_dir_all(repo.join(".git")).expect("create .git dir");
    let deep = repo.join("a").join("b");
    std::fs::create_dir_all(&deep).expect("create nested dirs");

    assert_eq!(
        find_repo_root(&deep),
        Some(repo.to_path_buf()),
        "must walk up from a/b to the repo root"
    );
    assert_eq!(
        find_repo_root(repo),
        Some(repo.to_path_buf()),
        "the repo root itself is its own repo root"
    );
}

#[test]
fn find_repo_root_none_without_any_git_dir() {
    let temp = TempDir::new().expect("create plain dir");
    let deep = temp.path().join("x").join("y");
    std::fs::create_dir_all(&deep).expect("create nested dirs");

    assert_eq!(
        find_repo_root(&deep),
        None,
        "no .git anywhere on the way up => None"
    );
    assert_eq!(find_repo_root(temp.path()), None);
}
