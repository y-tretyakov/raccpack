//! Forward-effect write-ahead log for the atomic raid commit (A3.3 PR3).
//!
//! Before every commit effect (parent dir creation, artifact rename, source /
//! trash delete) the caller appends the matching [`WalOp`] to a JSONL file and
//! fsyncs it — `append + fsync` **before** the effect. A mid-commit failure
//! then reads the log in reverse ([`Wal::read_reverse`]) and applies the
//! inverse of every recorded op via [`super::rollback::rollback_from_wal`],
//! so no placed artifact or created parent dir survives the failed commit.
//!
//! INVARIANTS:
//!
//! - The WAL stores only its path; every operation re-opens the file with
//!   create+append, so each record is durable before the caller mutates state.
//! - A corrupt line aborts the whole reverse read with [`Error::Other`]:
//!   rollback never continues over garbage (fail-safe).

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};

/// Forward-effect WAL ops; rollback applies inverses in reverse order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum WalOp {
    CreateDir { path: PathBuf },
    CreateFile { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    DeleteFile { path: PathBuf },
    DeleteDir { path: PathBuf },
}

impl WalOp {
    /// Inverse ops to apply when rolling back `self`, plus an optional warning
    /// for irreversible ops (DeleteFile / DeleteDir have no inverse).
    pub(crate) fn inverse(&self) -> (Vec<WalOp>, Option<String>) {
        match self {
            WalOp::CreateDir { path } => (vec![WalOp::DeleteDir { path: path.clone() }], None),
            WalOp::CreateFile { path } => (vec![WalOp::DeleteFile { path: path.clone() }], None),
            // Undo a rename by dropping the new file: `from` lived in the raid
            // staging, which is cleaned separately.
            WalOp::Rename { to, .. } => (vec![WalOp::DeleteFile { path: to.clone() }], None),
            WalOp::DeleteFile { path } => (
                vec![],
                Some(format!("cannot restore deleted file: {}", path.display())),
            ),
            WalOp::DeleteDir { path } => (
                vec![],
                Some(format!(
                    "cannot restore deleted directory: {}",
                    path.display()
                )),
            ),
        }
    }
}

/// Append-only JSONL write-ahead log recording forward commit effects.
///
/// Stores only the log path; operations open the file themselves with
/// create+append. [`Wal::new`] creates the file up-front so a WAL that cannot
/// be written fails the commit before any effect is applied.
pub(crate) struct Wal {
    path: PathBuf,
}

impl Wal {
    /// Open (create if missing) the WAL for appending.
    ///
    /// Creating the file here fails fast before any commit effect: a WAL that
    /// cannot be written must abort the commit rather than record nothing.
    ///
    /// # Errors
    ///
    /// Any open failure → [`Error::Io`] on the log path.
    pub(crate) fn new(path: &Path) -> Result<Wal> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(Wal {
            path: path.to_path_buf(),
        })
    }

    /// Append `op` as one JSON line and fsync. Called **before** the effect.
    ///
    /// # Errors
    ///
    /// - Serialization failure → [`Error::Other`].
    /// - File/fsync failure → [`Error::Io`] on the log path.
    pub(crate) fn record(&mut self, op: &WalOp) -> Result<()> {
        let json = serde_json::to_string(op).map_err(|err| Error::Other {
            message: format!("failed to serialize wal entry: {err}"),
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| Error::Io {
                path: self.path.clone(),
                source,
            })?;
        writeln!(file, "{json}").map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;
        file.sync_all().map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })
    }

    /// Read the WAL at `path` and return its ops in **reverse** order.
    ///
    /// Blank lines are skipped. A corrupt/broken line fails with
    /// [`Error::Other`] (fail-safe: rollback must not continue over garbage).
    ///
    /// # Errors
    ///
    /// - Unreadable file → [`Error::Io`].
    /// - Unparsable line → [`Error::Other`].
    pub(crate) fn read_reverse(path: &Path) -> Result<Vec<WalOp>> {
        let file = std::fs::File::open(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut ops = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let op = serde_json::from_str(line).map_err(|err| Error::Other {
                message: format!("corrupt wal entry in {}: {err}", path.display()),
            })?;
            ops.push(op);
        }
        ops.reverse();
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn op_variants() -> Vec<WalOp> {
        vec![
            WalOp::CreateDir {
                path: PathBuf::from("/den/secrets/2026/08"),
            },
            WalOp::CreateFile {
                path: PathBuf::from("/den/staging/x/tmp"),
            },
            WalOp::Rename {
                from: PathBuf::from("/den/staging/x/secrets.age"),
                to: PathBuf::from("/den/secrets/2026/08/p__ts.age"),
            },
            WalOp::DeleteFile {
                path: PathBuf::from("/proj/.env"),
            },
            WalOp::DeleteDir {
                path: PathBuf::from("/proj/node_modules"),
            },
        ]
    }

    #[test]
    fn inverse_maps_reversible_ops_to_delete_ops() {
        assert_eq!(
            WalOp::CreateDir {
                path: PathBuf::from("/den/secrets/2026/08"),
            }
            .inverse(),
            (
                vec![WalOp::DeleteDir {
                    path: PathBuf::from("/den/secrets/2026/08")
                }],
                None
            )
        );
        assert_eq!(
            WalOp::CreateFile {
                path: PathBuf::from("/tmp/f"),
            }
            .inverse(),
            (
                vec![WalOp::DeleteFile {
                    path: PathBuf::from("/tmp/f")
                }],
                None
            )
        );
        let (ops, warning) = WalOp::Rename {
            from: PathBuf::from("/from"),
            to: PathBuf::from("/to"),
        }
        .inverse();
        assert_eq!(
            ops,
            vec![WalOp::DeleteFile {
                path: PathBuf::from("/to")
            }]
        );
        assert!(warning.is_none());
    }

    #[test]
    fn inverse_of_delete_ops_is_irreversible_with_warning() {
        let (ops, warning) = WalOp::DeleteFile {
            path: PathBuf::from("/proj/.env"),
        }
        .inverse();
        assert!(ops.is_empty());
        assert_eq!(warning.unwrap(), "cannot restore deleted file: /proj/.env");

        let (ops, warning) = WalOp::DeleteDir {
            path: PathBuf::from("/proj/node_modules"),
        }
        .inverse();
        assert!(ops.is_empty());
        assert_eq!(
            warning.unwrap(),
            "cannot restore deleted directory: /proj/node_modules"
        );
    }

    #[test]
    fn wal_record_append_then_read_reverse() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");

        let mut wal = Wal::new(&path).unwrap();
        for op in &op_variants() {
            wal.record(op).unwrap();
        }

        let reverse = Wal::read_reverse(&path).unwrap();
        let mut expected = op_variants();
        expected.reverse();
        assert_eq!(reverse, expected);
    }

    #[test]
    fn wal_read_reverse_skips_blank_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");
        fs::write(
            &path,
            "{\"CreateFile\":{\"path\":\"/a\"}}\n\n{\"DeleteFile\":{\"path\":\"/b\"}}\n",
        )
        .unwrap();

        let ops = Wal::read_reverse(&path).unwrap();
        assert_eq!(
            ops,
            vec![
                WalOp::DeleteFile {
                    path: PathBuf::from("/b")
                },
                WalOp::CreateFile {
                    path: PathBuf::from("/a")
                },
            ]
        );
    }

    #[test]
    fn wal_read_reverse_fails_fast_on_corrupt_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");
        fs::write(&path, "{\"CreateDir\":{\"path\":\"/a\"}}\nnot-json\n").unwrap();

        let err = Wal::read_reverse(&path).unwrap_err();
        assert!(matches!(err, Error::Other { .. }));
    }
}
