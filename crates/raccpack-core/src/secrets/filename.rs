//! Filename-based secret detection.
//!
//! A static table of [`FilenamePattern`]s ([`DEFAULT_FILENAME_PATTERNS`]) maps
//! file names to a base [`SensitiveRisk`]. [`match_filename`] /
//! [`match_filename_all`] match a path by its `file_name()` only (no content is
//! read, no regex/glob), and [`scan_filenames`] walks a tree with the shared
//! walk helper and collects findings whose risk meets the configured minimum.

use std::path::Path;

use crate::domain::{Error, SensitiveRisk};

use super::finding::{FindingSource, SensitiveFinding};

/// How a [`FilenamePattern`] is matched against a file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameMatchKind {
    /// Exact file name (case-sensitive).
    Exact,
    /// File name ends with the pattern.
    Suffix,
    /// File name starts with the pattern.
    Prefix,
    /// File name contains the pattern as a substring (used sparingly).
    Contains,
}

/// A single filename pattern mapping a name shape to a severity.
#[derive(Debug, Clone)]
pub struct FilenamePattern {
    /// How `pattern` is matched against the file name.
    pub kind: NameMatchKind,
    /// Plain literal to match; not regex/glob.
    pub pattern: &'static str,
    /// Base risk assigned to a matching file.
    pub risk: SensitiveRisk,
    /// Stable id for tests/docs (e.g. `"env_file"`).
    pub id: &'static str,
    /// Human label for reports.
    pub label: &'static str,
}

/// Default filename pattern table, in deterministic order.
///
/// This is the single aggregation point for name-based patterns: adding a
/// pattern is one row here. When several patterns match one file,
/// [`match_filename`] picks the highest risk, breaking ties by table order.
pub static DEFAULT_FILENAME_PATTERNS: &[FilenamePattern] = &[
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".env",
        risk: SensitiveRisk::High,
        id: "env_file",
        label: "Environment file",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".env.local",
        risk: SensitiveRisk::High,
        id: "env_local",
        label: "Environment file (local)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".env.production",
        risk: SensitiveRisk::Critical,
        id: "env_prod",
        label: "Environment file (production)",
    },
    FilenamePattern {
        kind: NameMatchKind::Prefix,
        pattern: ".env.",
        risk: SensitiveRisk::High,
        id: "env_prefix",
        label: "Environment file (prefixed)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "credentials",
        risk: SensitiveRisk::High,
        id: "aws_credentials",
        label: "AWS credentials",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "credentials",
        risk: SensitiveRisk::High,
        id: "aws_credentials_path",
        label: "AWS credentials",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".pem",
        risk: SensitiveRisk::High,
        id: "private_key_pem",
        label: "Private key (PEM)",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".key",
        risk: SensitiveRisk::High,
        id: "private_key_key",
        label: "Private key",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "id_rsa",
        risk: SensitiveRisk::Critical,
        id: "id_rsa",
        label: "SSH private key (RSA)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "id_ed25519",
        risk: SensitiveRisk::Critical,
        id: "id_ed25519",
        label: "SSH private key (Ed25519)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "id_ecdsa",
        risk: SensitiveRisk::Critical,
        id: "id_ecdsa",
        label: "SSH private key (ECDSA)",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".ppk",
        risk: SensitiveRisk::High,
        id: "ppk",
        label: "PuTTY private key",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".p12",
        risk: SensitiveRisk::High,
        id: "p12",
        label: "PKCS#12 keystore",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".pfx",
        risk: SensitiveRisk::High,
        id: "pfx",
        label: "PKCS#12 certificate store",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: ".jks",
        risk: SensitiveRisk::High,
        id: "keystore",
        label: "Java keystore (JKS)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "kubeconfig",
        risk: SensitiveRisk::High,
        id: "kubeconfig",
        label: "Kubernetes kubeconfig",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "config.json",
        risk: SensitiveRisk::Medium,
        id: "docker_config",
        label: "Docker config",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".netrc",
        risk: SensitiveRisk::High,
        id: "netrc",
        label: "netrc credentials",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".npmrc",
        risk: SensitiveRisk::High,
        id: "npmrc",
        label: "npm registry config",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".pypirc",
        risk: SensitiveRisk::High,
        id: "pypirc",
        label: "PyPI registry config",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".git-credentials",
        risk: SensitiveRisk::Critical,
        id: "git_credentials",
        label: "Git credentials",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "secrets.json",
        risk: SensitiveRisk::High,
        id: "secrets_json",
        label: "Secrets file (JSON)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "secrets.yaml",
        risk: SensitiveRisk::High,
        id: "secrets_yaml",
        label: "Secrets file (YAML)",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: "secrets.yml",
        risk: SensitiveRisk::High,
        id: "secrets_yml",
        label: "Secrets file (YAML)",
    },
    FilenamePattern {
        kind: NameMatchKind::Contains,
        pattern: "service-account",
        risk: SensitiveRisk::High,
        id: "service_account",
        label: "Service account",
    },
    FilenamePattern {
        kind: NameMatchKind::Suffix,
        pattern: "-sa.json",
        risk: SensitiveRisk::High,
        id: "google_sa",
        label: "Google service account",
    },
    FilenamePattern {
        kind: NameMatchKind::Exact,
        pattern: ".htpasswd",
        risk: SensitiveRisk::High,
        id: "htpasswd",
        label: "htpasswd credentials",
    },
    FilenamePattern {
        kind: NameMatchKind::Contains,
        pattern: "wallet.dat",
        risk: SensitiveRisk::Critical,
        id: "wallet",
        label: "Wallet data",
    },
];

/// A matched filename pattern for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilenameMatch {
    /// Stable pattern id from [`FilenamePattern::id`].
    pub pattern_id: String,
    /// Human label from [`FilenamePattern::label`].
    pub label: String,
    /// Risk assigned by the matched pattern.
    pub risk: SensitiveRisk,
}

/// Match `path` by `file_name()` only and return the highest-risk match.
///
/// Matching is case-sensitive on the lossy `file_name()` string. When several
/// patterns match, the highest risk wins; ties resolve to the first pattern in
/// [`DEFAULT_FILENAME_PATTERNS`] (deterministic). Directories and paths without
/// a file name never match.
pub fn match_filename(path: &Path) -> Option<FilenameMatch> {
    let name = path.file_name()?.to_string_lossy();
    let mut best: Option<FilenameMatch> = None;
    for pattern in DEFAULT_FILENAME_PATTERNS {
        if !pattern_matches(pattern, &name) {
            continue;
        }
        let matched = FilenameMatch {
            pattern_id: pattern.id.to_string(),
            label: pattern.label.to_string(),
            risk: pattern.risk,
        };
        best = match best {
            Some(current) if matched.risk > current.risk => Some(matched),
            Some(current) => Some(current),
            None => Some(matched),
        };
    }
    best
}

/// Return every pattern that matches `path`, in table order.
///
/// Unlike [`match_filename`], no risk reduction is applied: all matching
/// patterns are reported (used for audit-style listings).
pub fn match_filename_all(path: &Path) -> Vec<FilenameMatch> {
    let Some(name) = path.file_name() else {
        return Vec::new();
    };
    let name = name.to_string_lossy();
    DEFAULT_FILENAME_PATTERNS
        .iter()
        .filter(|pattern| pattern_matches(pattern, &name))
        .map(|pattern| FilenameMatch {
            pattern_id: pattern.id.to_string(),
            label: pattern.label.to_string(),
            risk: pattern.risk,
        })
        .collect()
}

fn pattern_matches(pattern: &FilenamePattern, name: &str) -> bool {
    match pattern.kind {
        NameMatchKind::Exact => name == pattern.pattern,
        NameMatchKind::Suffix => name.ends_with(pattern.pattern),
        NameMatchKind::Prefix => name.starts_with(pattern.pattern),
        NameMatchKind::Contains => name.contains(pattern.pattern),
    }
}

/// Options controlling [`scan_filenames`].
#[derive(Debug, Clone)]
pub struct FilenameScanOptions {
    /// Maximum directory depth to descend (0 walks only the root).
    pub max_depth: usize,
    /// Policy deciding which directories are skipped.
    pub policy: crate::scan::SkipPolicy,
    /// Minimum risk to include; findings below this are dropped.
    pub min_risk: SensitiveRisk,
}

impl Default for FilenameScanOptions {
    fn default() -> Self {
        Self {
            max_depth: crate::config::default_max_depth(),
            policy: crate::scan::SkipPolicy::default_scan(),
            min_risk: SensitiveRisk::Low,
        }
    }
}

/// Scan `root` for files matching the filename pattern table.
///
/// The root is validated with [`crate::scan::walk::ensure_scan_root`] and walked
/// with
/// [`crate::scan::walk::walk_tree`] (symlinks never followed, `max_depth` and
/// `policy` taken from `opts`). Only regular files are matched — directories
/// are never considered. Matching uses `file_name()` only; file contents are
/// never read. A finding is kept when its risk satisfies
/// [`SensitiveRisk::at_least`] on `opts.min_risk`.
///
/// Walk errors are never dropped: IO errors map to [`Error::Io`], other walkdir
/// errors to [`Error::Other`]. The result is sorted by `path` ascending, then
/// `risk` descending, so output is deterministic.
pub fn scan_filenames(
    root: &Path,
    opts: &FilenameScanOptions,
) -> Result<Vec<SensitiveFinding>, Error> {
    crate::scan::walk::ensure_scan_root(root)?;

    let walk_opts = crate::scan::walk::WalkOptions {
        max_depth: opts.max_depth,
        policy: opts.policy.clone(),
        include_root: false,
    };

    let mut findings: Vec<SensitiveFinding> = Vec::new();
    for item in crate::scan::walk::walk_tree(root, &walk_opts) {
        let entry = item.map_err(|err| crate::scan::walk::map_walk_error(err, root))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(matched) = match_filename(entry.path()) else {
            continue;
        };
        if !matched.risk.at_least(opts.min_risk) {
            continue;
        }
        findings.push(SensitiveFinding {
            path: entry.path().to_path_buf(),
            risk: matched.risk,
            source: FindingSource::Filename {
                pattern_id: matched.pattern_id.clone(),
            },
            label: matched.label.clone(),
            sources: vec![FindingSource::Filename {
                pattern_id: matched.pattern_id,
            }],
            labels: vec![matched.label],
            content_match: None,
        });
    }

    findings.sort_by(|a, b| a.path.cmp(&b.path).then(b.risk.cmp(&a.risk)));
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_expected_row_count() {
        assert_eq!(DEFAULT_FILENAME_PATTERNS.len(), 28);
    }

    #[test]
    fn table_ids_are_unique() {
        let mut ids: Vec<&str> = DEFAULT_FILENAME_PATTERNS
            .iter()
            .map(|pattern| pattern.id)
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), DEFAULT_FILENAME_PATTERNS.len());
    }

    #[test]
    fn pattern_kinds_implement_expectations() {
        let exact = |name| pattern_matches(&pattern(NameMatchKind::Exact, "id_rsa"), name);
        assert!(exact("id_rsa"));
        assert!(!exact("my_id_rsa"));
        assert!(!exact("id_rsa.pub"));

        let suffix = |name| pattern_matches(&pattern(NameMatchKind::Suffix, ".pem"), name);
        assert!(suffix("key.pem"));
        assert!(!suffix("pem"));

        let prefix = |name| pattern_matches(&pattern(NameMatchKind::Prefix, ".env."), name);
        assert!(prefix(".env.staging"));
        assert!(!prefix("env.staging"));

        let contains =
            |name| pattern_matches(&pattern(NameMatchKind::Contains, "wallet.dat"), name);
        assert!(contains("backup-wallet.dat-2024"));
        assert!(!contains("wallet.txt"));
    }

    fn pattern(kind: NameMatchKind, pattern: &'static str) -> FilenamePattern {
        FilenamePattern {
            kind,
            pattern,
            risk: SensitiveRisk::High,
            id: "test",
            label: "test",
        }
    }
}
