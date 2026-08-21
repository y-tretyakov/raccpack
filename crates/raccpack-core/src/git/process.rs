//! [`GitClient`] implementation shelling out to the `git` binary.
//!
//! No network access: `GIT_TERMINAL_PROMPT=0` is always set and every spawn is
//! bounded by a poll-based timeout that kills the child on expiry. Diagnostics
//! in [`Error::Git`] carry git's own messages only — never file contents.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::{Error, Result};
use crate::git::client::{GitClient, GitFileStatus};

/// Poll interval while waiting for the git child process.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Upper bound for diagnostics copied into an error message.
const MAX_MESSAGE_CHARS: usize = 512;

/// Default binary resolution ("git" from `PATH`) and 30s timeout.
#[derive(Debug, Clone)]
pub struct ProcessGitClient {
    /// Git executable to spawn.
    pub git_binary: PathBuf,
    /// Wall-clock budget per git invocation; the child is killed on expiry.
    pub timeout: Duration,
}

/// Outcome of one git invocation: exit flag plus captured output.
struct GitOutcome {
    success: bool,
    stdout: Vec<u8>,
    stderr: String,
}

impl ProcessGitClient {
    /// Client using `git` from `PATH` with a 30 second timeout.
    pub fn new() -> Self {
        Self {
            git_binary: PathBuf::from("git"),
            timeout: Duration::from_secs(30),
        }
    }

    fn git_error(context: &str, detail: String) -> Error {
        Error::Git {
            message: format!("{} ({context})", truncate_message(&detail)),
        }
    }

    /// Spawn `git -C <repo> <args>` and wait under the timeout budget.
    ///
    /// stdout/stderr are drained on helper threads so large status output can
    /// never deadlock against full OS pipes while we poll. A non-zero exit is
    /// reported as [`GitOutcome::success == false`], not as an error; only
    /// spawn/wait failures and timeouts return `Err`.
    fn run_git(&self, repo: &Path, args: &[OsString]) -> Result<GitOutcome> {
        let mut child = Command::new(&self.git_binary)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Self::git_error("failed to start git", e.to_string()))?;

        let stdout_reader = child
            .stdout
            .take()
            .map(|mut pipe| thread::spawn(move || drain_to_vec(&mut pipe)));
        let stderr_reader = child
            .stderr
            .take()
            .map(|mut pipe| thread::spawn(move || drain_to_vec(&mut pipe)));

        let deadline = Instant::now() + self.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout = join_drain(stdout_reader);
                    let stderr = String::from_utf8_lossy(&join_drain(stderr_reader)).into_owned();
                    return Ok(GitOutcome {
                        success: status.success(),
                        stdout,
                        stderr,
                    });
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(Error::Git {
                            message: format!("git timed out after {}s", self.timeout.as_secs()),
                        });
                    }
                    thread::sleep(POLL_INTERVAL);
                }
                Err(e) => {
                    return Err(Self::git_error("failed to wait for git", e.to_string()));
                }
            }
        }
    }
}

impl Default for ProcessGitClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitClient for ProcessGitClient {
    fn is_repo(&self, path: &Path) -> Result<bool> {
        let args = vec![
            OsString::from("rev-parse"),
            OsString::from("--is-inside-work-tree"),
        ];
        match self.run_git(path, &args)? {
            outcome if outcome.success => {
                Ok(String::from_utf8_lossy(&outcome.stdout).trim() == "true")
            }
            // rev-parse exits non-zero outside a repo: not a repo, not an
            // error. Only spawn failures and timeouts surface as Err.
            _ => Ok(false),
        }
    }

    fn files_status(
        &self,
        repo: &Path,
        files: &[PathBuf],
    ) -> Result<HashMap<PathBuf, GitFileStatus>> {
        if files.is_empty() {
            return Ok(HashMap::new());
        }

        let mut rel_args = Vec::with_capacity(files.len());
        let mut unqueryable = Vec::new();
        for file in files {
            match file.strip_prefix(repo) {
                Ok(rel) => rel_args.push(rel.as_os_str().to_os_string()),
                Err(_) => unqueryable.push(file.clone()),
            }
        }

        let mut statuses: HashMap<PathBuf, GitFileStatus> = unqueryable
            .iter()
            .map(|p| (p.clone(), GitFileStatus::Unknown))
            .collect();

        if !rel_args.is_empty() {
            let outcome = self.run_git(repo, &status_args(&rel_args))?;
            if !outcome.success {
                let detail = if outcome.stderr.trim().is_empty() {
                    "git exited with a non-zero status".to_string()
                } else {
                    outcome.stderr
                };
                return Err(Self::git_error("git status failed", detail));
            }
            let parsed = parse_porcelain_z(&outcome.stdout);
            for file in files {
                if statuses.contains_key(file) {
                    continue;
                }
                if let Ok(rel) = file.strip_prefix(repo) {
                    let status = parsed.get(rel).copied().unwrap_or(GitFileStatus::Tracked);
                    statuses.insert(file.clone(), status);
                }
            }
        }

        Ok(statuses)
    }
}

/// Build the `status --porcelain=v1 -z …` argument list for relative paths.
fn status_args(rel_paths: &[OsString]) -> Vec<OsString> {
    let mut args = Vec::with_capacity(rel_paths.len() + 6);
    args.push(OsString::from("status"));
    args.push(OsString::from("--porcelain=v1"));
    args.push(OsString::from("-z"));
    args.push(OsString::from("--ignored=matching"));
    args.push(OsString::from("--untracked-files=all"));
    args.push(OsString::from("--"));
    args.extend(rel_paths.iter().cloned());
    args
}

/// Map a two-letter porcelain XY code to a [`GitFileStatus`].
///
/// Priority follows the A4.1 contract: any `M` wins over staged adds, so
/// `AM` reports `Modified`; absence from the output is handled by the caller
/// as `Tracked`. Rename/copy are index operations and count as staged.
fn map_xy(xy: &[u8]) -> GitFileStatus {
    match xy {
        b"??" => GitFileStatus::Untracked,
        b"!!" => GitFileStatus::Ignored,
        _ if xy.contains(&b'M') => GitFileStatus::Modified,
        _ if matches!(xy.first(), Some(b'A' | b'R' | b'C')) => GitFileStatus::Staged,
        _ if xy.contains(&b'D') => GitFileStatus::Deleted,
        _ if xy.iter().all(u8::is_ascii_whitespace) => GitFileStatus::Tracked,
        _ => GitFileStatus::Unknown,
    }
}

/// Parse NUL-separated `XY <path>\0` porcelain v1 records.
///
/// Rename/copy records carry a second NUL-terminated field (the original
/// path), which is consumed and discarded.
fn parse_porcelain_z(output: &[u8]) -> HashMap<PathBuf, GitFileStatus> {
    let mut records = output.split(|byte| *byte == 0).filter(|r| !r.is_empty());
    let mut parsed = HashMap::new();
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let xy = &record[..2];
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        if xy.contains(&b'R') || xy.contains(&b'C') {
            records.next();
        }
        parsed.insert(PathBuf::from(path), map_xy(xy));
    }
    parsed
}

/// Read a stream to EOF into a vector (worker-thread helper).
fn drain_to_vec<R: Read>(reader: &mut R) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf);
    buf
}

/// Join a drain thread, falling back to empty output on panic.
fn join_drain(handle: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

/// Trim and cap a diagnostic so error lines stay readable.
fn truncate_message(message: &str) -> String {
    let trimmed = message.trim();
    match trimmed.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((idx, _)) => format!("{}…", &trimmed[..idx]),
        None => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_xy_covers_the_contract_table() {
        let cases = [
            (&b"??"[..], GitFileStatus::Untracked),
            (b"!!", GitFileStatus::Ignored),
            (b" M", GitFileStatus::Modified),
            (b"M ", GitFileStatus::Modified),
            (b"MM", GitFileStatus::Modified),
            (b"A ", GitFileStatus::Staged),
            (b"AM", GitFileStatus::Modified),
            (b" D", GitFileStatus::Deleted),
            (b"D ", GitFileStatus::Deleted),
            (b"XX", GitFileStatus::Unknown),
        ];
        for (xy, expected) in cases {
            assert_eq!(
                map_xy(xy),
                expected,
                "mapping of {:?}",
                String::from_utf8_lossy(xy)
            );
        }
    }

    #[test]
    fn clean_record_maps_to_tracked() {
        assert_eq!(map_xy(b"  "), GitFileStatus::Tracked);
    }

    #[test]
    fn parse_porcelain_reads_records_and_skips_rename_source() {
        let mut output = b"?? .env\0 M notes.txt\0R  new.txt\0old.txt\0!! ignored.log\0".to_vec();
        output.push(0);
        let parsed = parse_porcelain_z(&output);

        assert_eq!(
            parsed.get(Path::new(".env")),
            Some(&GitFileStatus::Untracked)
        );
        assert_eq!(
            parsed.get(Path::new("notes.txt")),
            Some(&GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.get(Path::new("new.txt")),
            Some(&GitFileStatus::Staged)
        );
        assert_eq!(
            parsed.get(Path::new("ignored.log")),
            Some(&GitFileStatus::Ignored)
        );
        assert!(
            !parsed.contains_key(Path::new("old.txt")),
            "rename source must be consumed"
        );
    }

    #[test]
    fn parse_porcelain_empty_output_is_empty_map() {
        assert!(parse_porcelain_z(b"\0\0").is_empty());
    }

    #[test]
    fn status_args_use_porcelain_v1_with_ignored_and_all() {
        let args = status_args(&[OsString::from(".env")]);
        let joined: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(joined.contains(&"--porcelain=v1".to_string()));
        assert!(joined.contains(&"-z".to_string()));
        assert!(joined.contains(&"--ignored=matching".to_string()));
        assert!(joined.contains(&"--untracked-files=all".to_string()));
        assert_eq!(joined.last().map(String::as_str), Some(".env"));
    }

    #[test]
    fn truncate_message_caps_long_diagnostics() {
        let long = "x".repeat(MAX_MESSAGE_CHARS + 50);
        let cut = truncate_message(&long);
        assert!(cut.chars().count() <= MAX_MESSAGE_CHARS + 1);
        assert!(cut.ends_with('…'));

        assert_eq!(truncate_message("  short\n"), "short");
    }
}
