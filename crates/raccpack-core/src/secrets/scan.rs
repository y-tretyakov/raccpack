//! Combined secret scan: filename + content detection over one tree.
//!
//! [`scan_secrets`] performs a single walk (honoring the shared skip policy and
//! max depth) and merges, per path, filename matches and content hits into one
//! [`SensitiveFinding`]. Risk only ever upgrades via [`upgrade_risk`]. No raw
//! secret values appear in results — content hits are masked.

use std::path::Path;

use crate::domain::{Error, SensitiveRisk};

use super::content::{scan_file_content, ContentHit, ContentScanLimits};
use super::filename::match_filename_all;
use super::finding::{FindingSource, SensitiveFinding};
use super::risk::upgrade_risk;

/// Options controlling [`scan_secrets`].
#[derive(Debug, Clone)]
pub struct SecretScanOptions {
    /// Maximum directory depth to descend (0 walks only the root).
    pub max_depth: usize,
    /// Policy deciding which directories are skipped.
    pub policy: crate::scan::SkipPolicy,
    /// Minimum risk to include; sources below this are dropped per path.
    pub min_risk: SensitiveRisk,
    /// Default true. When false, file contents are never read (filename only).
    pub scan_content: bool,
    /// Size / binary limits for content scanning.
    pub limits: ContentScanLimits,
    /// Accepted for M3.3 API-shape; NOT used for aggregation on this stage.
    pub find_repeated: bool,
}

impl Default for SecretScanOptions {
    fn default() -> Self {
        Self {
            max_depth: crate::config::default_max_depth(),
            policy: crate::scan::SkipPolicy::default_scan(),
            min_risk: SensitiveRisk::Low,
            scan_content: true,
            limits: ContentScanLimits::default(),
            find_repeated: false,
        }
    }
}

/// One walk combining filename + content detection, merged per path.
///
/// # Algorithm
///
/// 1. The root is validated with
///    [`crate::scan::walk::ensure_scan_root`] and walked with
///    [`crate::scan::walk::walk_tree`] (`max_depth` and `policy` from `opts`,
///    `include_root: false`). Walk errors are never dropped: IO errors map to
///    [`Error::Io`], other walkdir errors to [`Error::Other`].
/// 2. For each regular file entry, filename matches come from
///    [`match_filename_all`]; content hits come from
///    [`scan_file_content`] when `opts.scan_content` is true. A content read
///    error is best-effort skipped (no content hits for that file; the walk
///    continues).
/// 3. Sources are filtered individually: only those meeting
///    [`SensitiveRisk::at_least`] on `opts.min_risk` are kept. If none remain,
///    the file is skipped.
/// 4. Merging per path: `risk` is the fold of [`upgrade_risk`] over all kept
///    risks; `sources` are filename sources (table order) then content sources
///    (line/marker order); `labels` are aligned; `content_match` is the masked
///    value of the highest-risk content hit (ties break to the first in scan
///    order); `source` / `label` are the first of `sources` / `labels`.
/// 5. The result is sorted by `path` ascending, then `risk` descending, so
///    output is deterministic.
pub fn scan_secrets(root: &Path, opts: &SecretScanOptions) -> Result<Vec<SensitiveFinding>, Error> {
    scan_secrets_with_count(root, opts).map(|(findings, _)| findings)
}

/// Like [`scan_secrets`] but also reports the number of regular files walked.
///
/// The count increments for every regular file entry yielded by the walk,
/// before filename / content processing, so files that produce no finding are
/// still counted. Used by the `dig` facade to report scan coverage.
pub(crate) fn scan_secrets_with_count(
    root: &Path,
    opts: &SecretScanOptions,
) -> Result<(Vec<SensitiveFinding>, u64), Error> {
    crate::scan::walk::ensure_scan_root(root)?;

    let walk_opts = crate::scan::walk::WalkOptions {
        max_depth: opts.max_depth,
        policy: opts.policy.clone(),
        include_root: false,
    };

    let mut findings: Vec<SensitiveFinding> = Vec::new();
    let mut files_scanned: u64 = 0;
    for item in crate::scan::walk::walk_tree(root, &walk_opts) {
        let entry = item.map_err(|err| map_walk_error(err, root))?;
        if !entry.file_type().is_file() {
            continue;
        }
        files_scanned += 1;
        let path = entry.path();

        let filename_matches = match_filename_all(path)
            .into_iter()
            .filter(|m| m.risk.at_least(opts.min_risk));

        let content_hits: Vec<ContentHit> = if opts.scan_content {
            scan_file_content(path, &opts.limits).unwrap_or_default()
        } else {
            Vec::new()
        };
        let content_hits: Vec<ContentHit> = content_hits
            .into_iter()
            .filter(|hit| hit.risk.at_least(opts.min_risk))
            .collect();

        let mut sources: Vec<FindingSource> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        let mut first: Option<(FindingSource, String)> = None;
        let mut risk = SensitiveRisk::Low;
        let mut best_content: Option<&ContentHit> = None;

        for matched in filename_matches {
            risk = upgrade_risk(risk, matched.risk);
            let source = FindingSource::Filename {
                pattern_id: matched.pattern_id.clone(),
            };
            if first.is_none() {
                first = Some((source.clone(), matched.label.clone()));
            }
            sources.push(source);
            labels.push(matched.label);
        }

        for hit in &content_hits {
            risk = upgrade_risk(risk, hit.risk);
            let source = FindingSource::Content {
                marker_id: hit.marker_id.clone(),
                masked: hit.masked.clone(),
                line: hit.line,
            };
            if first.is_none() {
                first = Some((source.clone(), hit.label.clone()));
            }
            sources.push(source);
            labels.push(hit.label.clone());
            let better = match best_content {
                Some(best) => hit.risk > best.risk,
                None => true,
            };
            if better {
                best_content = Some(hit);
            }
        }

        let Some((source, label)) = first else {
            continue;
        };

        findings.push(SensitiveFinding {
            path: path.to_path_buf(),
            risk,
            source,
            label,
            sources,
            labels,
            content_match: best_content.map(|hit| hit.masked.clone()),
        });
    }

    findings.sort_by(|a, b| a.path.cmp(&b.path).then(b.risk.cmp(&a.risk)));
    Ok((findings, files_scanned))
}

/// Map a [`walkdir::Error`] to the domain [`Error`] type.
///
/// Identical mapping to `scan_filenames`: IO errors map to [`Error::Io`] with
/// the offending path (falling back to the scan root); walkdir errors without
/// an IO source (e.g. loop detection) map to [`Error::Other`].
fn map_walk_error(err: walkdir::Error, root: &Path) -> Error {
    let path = err
        .path()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf());
    let message = err.to_string();
    match err.into_io_error() {
        Some(source) => Error::Io { path, source },
        None => Error::Other { message },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
        fs::write(&path, content).expect("write fixture file");
    }

    fn rel_paths(findings: &[SensitiveFinding], root: &Path) -> Vec<PathBuf> {
        findings
            .iter()
            .map(|f| {
                f.path
                    .strip_prefix(root)
                    .expect("finding must live under the scan root")
                    .to_path_buf()
            })
            .collect()
    }

    #[test]
    fn combined_env_and_content_upgrade_risk() {
        let root = TempDir::new().unwrap();
        write(root.path(), ".env", b"AKIAABCDEFGHIJKLMNOPQRST\n");

        let findings = scan_secrets(root.path(), &SecretScanOptions::default()).unwrap();
        assert_eq!(
            rel_paths(&findings, root.path()),
            vec![PathBuf::from(".env")]
        );
        let finding = &findings[0];
        assert_eq!(finding.risk, SensitiveRisk::Critical);
        assert_eq!(finding.sources.len(), 2);
        assert!(matches!(
            &finding.sources[0],
            FindingSource::Filename { pattern_id } if pattern_id == "env_file"
        ));
        assert!(matches!(
            &finding.sources[1],
            FindingSource::Content { marker_id, .. } if marker_id == "aws_access_key"
        ));
        assert!(finding.content_match.is_some());
        let content_match = finding.content_match.as_ref().unwrap();
        assert!(content_match.masked.starts_with("AKIA"));
        assert!(content_match.masked.contains('…'));
    }

    #[test]
    fn content_only_finding() {
        let root = TempDir::new().unwrap();
        write(
            root.path(),
            "notes.txt",
            b"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ\n",
        );

        let findings = scan_secrets(root.path(), &SecretScanOptions::default()).unwrap();
        assert_eq!(
            rel_paths(&findings, root.path()),
            vec![PathBuf::from("notes.txt")]
        );
        let finding = &findings[0];
        assert_eq!(finding.risk, SensitiveRisk::Critical);
        assert_eq!(finding.sources.len(), 1);
        assert!(matches!(
            &finding.sources[0],
            FindingSource::Content { marker_id, .. } if marker_id == "github_pat"
        ));
        assert_eq!(finding.label, "GitHub personal access token");
    }

    #[test]
    fn min_risk_filters_sources_per_path() {
        let root = TempDir::new().unwrap();
        write(root.path(), "stripe_test.txt", b"sk_test_abc123\n");
        write(root.path(), ".env", b"FOO=bar\n");

        let opts = SecretScanOptions {
            min_risk: SensitiveRisk::Critical,
            ..SecretScanOptions::default()
        };
        let findings = scan_secrets(root.path(), &opts).unwrap();
        assert!(
            findings.is_empty(),
            "Medium and High sources must be filtered"
        );

        write(root.path(), "stripe_live.txt", b"sk_live_abc123\n");
        let findings = scan_secrets(root.path(), &opts).unwrap();
        assert_eq!(
            rel_paths(&findings, root.path()),
            vec![PathBuf::from("stripe_live.txt")]
        );
    }

    #[test]
    fn scan_content_false_is_filename_only() {
        let root = TempDir::new().unwrap();
        write(root.path(), ".env", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        write(root.path(), "notes.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");

        let opts = SecretScanOptions {
            scan_content: false,
            ..SecretScanOptions::default()
        };
        let findings = scan_secrets(root.path(), &opts).unwrap();
        assert_eq!(
            rel_paths(&findings, root.path()),
            vec![PathBuf::from(".env")]
        );
    }

    #[test]
    fn deterministic_sort_by_path_then_risk() {
        let root = TempDir::new().unwrap();
        write(root.path(), ".env", b"FOO=bar\n");
        write(root.path(), "a.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        write(root.path(), "b.sh", b"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");

        let findings = scan_secrets(root.path(), &SecretScanOptions::default()).unwrap();
        assert_eq!(
            rel_paths(&findings, root.path()),
            vec![
                PathBuf::from(".env"),
                PathBuf::from("a.txt"),
                PathBuf::from("b.sh"),
            ]
        );
    }

    #[test]
    fn no_scan_inside_node_modules() {
        let root = TempDir::new().unwrap();
        write(root.path(), ".env", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        write(
            root.path(),
            "node_modules/pkg/.env",
            b"AKIAABCDEFGHIJKLMNOPQRST\n",
        );

        let findings = scan_secrets(root.path(), &SecretScanOptions::default()).unwrap();
        let rels = rel_paths(&findings, root.path());
        assert_eq!(rels, vec![PathBuf::from(".env")]);
    }
}
