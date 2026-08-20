//! Best-effort rollback from the commit WAL (A3.3 PR3).
//!
//! [`rollback_from_wal`] reads the forward JSONL WAL in reverse and applies
//! the inverse of every recorded effect: placed `.age` / `.tar.zst` artifacts
//! are removed and created parent directories are dropped when empty. Ops with
//! no inverse (source / trash deletes) surface a warning instead.
//!
//! INVARIANTS:
//!
//! - The function **never** returns `Err`; every problem becomes a warning in
//!   the [`RollbackReport`]. A missing WAL means nothing to roll back.
//! - Missing targets are fine (the effect was never applied, or the file was
//!   already removed): `NotFound` is not a warning.

use std::fs;
use std::path::Path;

use super::wal::{Wal, WalOp};

/// Outcome of a best-effort rollback attempt.
pub(crate) struct RollbackReport {
    /// `true` iff at least one inverse op was requested (applied or no-op)
    /// or an irreversible op produced a warning.
    pub applied: bool,
    /// Non-fatal issues encountered while rolling back.
    pub warnings: Vec<String>,
}

/// Apply the inverse of every recorded WAL op in reverse order.
///
/// Best-effort: never returns `Err`; all failures become warnings. A missing
/// WAL (or one that cannot be read) yields `applied: false` with the failure
/// noted as a warning.
pub(crate) fn rollback_from_wal(wal_path: &Path) -> RollbackReport {
    if !wal_path.exists() {
        return RollbackReport {
            applied: false,
            warnings: Vec::new(),
        };
    }

    let mut report = RollbackReport {
        applied: false,
        warnings: Vec::new(),
    };

    let ops = match Wal::read_reverse(wal_path) {
        Ok(ops) => ops,
        Err(err) => {
            report.warnings.push(format!(
                "cannot read rollback log at {}: {err}",
                wal_path.display()
            ));
            return report;
        }
    };

    for op in ops {
        let (inverses, irreversible) = op.inverse();
        if let Some(warning) = irreversible {
            report.applied = true;
            report.warnings.push(warning);
        }
        for inverse in inverses {
            report.applied = true;
            match inverse {
                WalOp::DeleteFile { path } => match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => report
                        .warnings
                        .push(format!("could not remove file {}: {err}", path.display())),
                },
                WalOp::DeleteDir { path } => match fs::remove_dir(&path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => report.warnings.push(format!(
                        "could not remove directory {}: {err}",
                        path.display()
                    )),
                },
                // `inverse()` only ever yields DeleteFile / DeleteDir.
                _ => {}
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn missing_wal_means_nothing_to_roll_back() {
        let dir = TempDir::new().unwrap();
        let report = rollback_from_wal(&dir.path().join("nope.jsonl"));
        assert!(!report.applied);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn rollback_removes_placed_file_and_empty_parent_dir() {
        let dir = TempDir::new().unwrap();
        let parent = dir.path().join("secrets/2026/08");
        fs::create_dir_all(&parent).unwrap();
        let placed = parent.join("p__ts.age");
        fs::write(&placed, b"age-blob").unwrap();

        let wal_path = dir.path().join("wal.jsonl");
        let mut wal = Wal::new(&wal_path).unwrap();
        wal.record(&WalOp::CreateDir {
            path: parent.clone(),
        })
        .unwrap();
        wal.record(&WalOp::Rename {
            from: dir.path().join("staging/secrets.age"),
            to: placed.clone(),
        })
        .unwrap();

        let report = rollback_from_wal(&wal_path);
        assert!(report.applied);
        assert!(report.warnings.is_empty());
        assert!(!placed.exists());
        assert!(!parent.exists(), "empty created dir must be removed");
    }

    #[test]
    fn rollback_tolerates_missing_target_and_reports_irreversible_warnings() {
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("wal.jsonl");
        let mut wal = Wal::new(&wal_path).unwrap();
        wal.record(&WalOp::Rename {
            from: dir.path().join("staging/secrets.age"),
            to: dir.path().join("never-placed.age"),
        })
        .unwrap();
        wal.record(&WalOp::DeleteFile {
            path: dir.path().join("proj/.env"),
        })
        .unwrap();

        let report = rollback_from_wal(&wal_path);
        assert!(report.applied);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("cannot restore deleted file"));
    }

    #[test]
    fn corrupt_wal_reports_warning_without_panic() {
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("wal.jsonl");
        fs::write(&wal_path, "garbage\n").unwrap();

        let report = rollback_from_wal(&wal_path);
        assert!(!report.applied);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("cannot read rollback log"));
    }

    #[test]
    fn non_empty_parent_dir_produces_warning() {
        let dir = TempDir::new().unwrap();
        let parent = dir.path().join("secrets/2026/08");
        fs::create_dir_all(&parent).unwrap();
        fs::write(parent.join("other.age"), b"keep").unwrap();

        let wal_path = dir.path().join("wal.jsonl");
        let mut wal = Wal::new(&wal_path).unwrap();
        wal.record(&WalOp::CreateDir { path: parent }).unwrap();

        let report = rollback_from_wal(&wal_path);
        assert!(report.applied);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("could not remove directory"));
    }
}
