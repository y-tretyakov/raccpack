//! Content markers: regex / prefix / contains rules over file contents.
//!
//! [`DEFAULT_CONTENT_MARKERS`] is the single aggregation point for content
//! rules (one row per marker). [`scan_file_content`] applies the table to a
//! single regular file with size and binary-sniff limits, returning masked
//! [`ContentHit`]s — never raw values.

use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::domain::{Error, SensitiveRisk};

use super::mask::{mask_secret, MaskedValue};

/// How a [`ContentMarker`] is matched against a line of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMatchKind {
    /// Value/token starts with the prefix (e.g. "AKIA").
    Prefix,
    /// Contains literal needle (used sparingly).
    Contains,
    /// Regex, compiled once at startup.
    Regex,
}

/// A single content rule mapping a pattern to a severity.
///
/// Values extracted by a marker are always masked via [`mask_secret`] before
/// they appear in any result; the raw text never leaves this module.
#[derive(Debug, Clone)]
pub struct ContentMarker {
    /// Stable id for tests/reports (e.g. `"aws_access_key"`).
    pub id: &'static str,
    /// How `pattern` is matched.
    pub kind: ContentMatchKind,
    /// Literal prefix / needle, or a regex source for [`ContentMatchKind::Regex`].
    pub pattern: &'static str,
    /// Severity assigned to a hit from this marker.
    pub risk: SensitiveRisk,
    /// Human label for reports.
    pub label: &'static str,
    /// If true, only applies to text-ish (non-binary) files.
    pub text_only: bool,
}

/// Default content marker table, in deterministic order (12 rows).
///
/// This is the single aggregation point for content rules: adding a marker is
/// one row here. The `telegram_bot` rule from the M3.2 spec is deliberately
/// deferred (it is noisy and needs a value-length bound); it will be re-added
/// with its own unit tests when the matcher supports length constraints.
pub static DEFAULT_CONTENT_MARKERS: &[ContentMarker] = &[
    ContentMarker {
        id: "aws_access_key",
        kind: ContentMatchKind::Prefix,
        pattern: "AKIA",
        risk: SensitiveRisk::Critical,
        label: "AWS access key",
        text_only: true,
    },
    ContentMarker {
        id: "aws_secret_assign",
        kind: ContentMatchKind::Regex,
        pattern: r"(?i)aws_secret_access_key\s*=\s*\S+",
        risk: SensitiveRisk::Critical,
        label: "AWS secret access key assignment",
        text_only: true,
    },
    ContentMarker {
        id: "generic_api_key_assign",
        kind: ContentMatchKind::Regex,
        pattern: r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[A-Za-z0-9_\-]{16,}"#,
        risk: SensitiveRisk::High,
        label: "API key assignment",
        text_only: true,
    },
    ContentMarker {
        id: "generic_secret_assign",
        kind: ContentMatchKind::Regex,
        pattern: r#"(?i)(secret|password|passwd|token)\s*[:=]\s*['"]?\S{8,}"#,
        risk: SensitiveRisk::High,
        label: "Secret assignment",
        text_only: true,
    },
    ContentMarker {
        id: "private_key_header",
        kind: ContentMatchKind::Regex,
        pattern: r"-----BEGIN[ \t]*(?:RSA |EC |DSA |OPENSSH |ENCRYPTED )?PRIVATE KEY-----",
        risk: SensitiveRisk::Critical,
        label: "Private key (PEM header)",
        text_only: true,
    },
    ContentMarker {
        id: "github_pat",
        kind: ContentMatchKind::Prefix,
        pattern: "ghp_",
        risk: SensitiveRisk::Critical,
        label: "GitHub personal access token",
        text_only: true,
    },
    ContentMarker {
        id: "github_oauth",
        kind: ContentMatchKind::Prefix,
        pattern: "gho_",
        risk: SensitiveRisk::Critical,
        label: "GitHub OAuth token",
        text_only: true,
    },
    ContentMarker {
        id: "slack_token",
        kind: ContentMatchKind::Prefix,
        pattern: "xoxb-",
        risk: SensitiveRisk::High,
        label: "Slack token",
        text_only: true,
    },
    ContentMarker {
        id: "stripe_live",
        kind: ContentMatchKind::Prefix,
        pattern: "sk_live_",
        risk: SensitiveRisk::Critical,
        label: "Stripe live key",
        text_only: true,
    },
    ContentMarker {
        id: "stripe_test",
        kind: ContentMatchKind::Prefix,
        pattern: "sk_test_",
        risk: SensitiveRisk::Medium,
        label: "Stripe test key",
        text_only: true,
    },
    ContentMarker {
        id: "connection_string",
        kind: ContentMatchKind::Regex,
        pattern: r"(?i)(postgres|mysql|mongodb)://\S+:\S+@",
        risk: SensitiveRisk::Critical,
        label: "Database connection string",
        text_only: true,
    },
    ContentMarker {
        id: "jwt_like",
        kind: ContentMatchKind::Regex,
        pattern: r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
        risk: SensitiveRisk::Medium,
        label: "JWT-like token",
        text_only: true,
    },
];

/// A [`ContentMarker`] paired with its compiled regex (Regex kind only).
///
/// Fields are `pub(crate)` so reveal (`super::reveal`) can re-extract candidates
/// for a specific marker without duplicating the matching logic.
pub(crate) struct CompiledMarker {
    pub(crate) marker: &'static ContentMarker,
    pub(crate) regex: Option<Regex>,
}

/// Lazily compiled marker table.
///
/// # Why the `.expect` here is sanctioned
///
/// This is the **single** allowed `expect` in the module (spec §8 test 9):
/// `DEFAULT_CONTENT_MARKERS` is a static table written by us, so any invalid
/// regex is a programmer error that must fail at startup (compile of the
/// OnceLock) rather than silently disabling a marker at scan time. If a
/// marker's regex ever fails to compile, that is a bug in this file, not a
/// runtime input error.
fn compiled_markers() -> &'static [CompiledMarker] {
    static COMPILED: OnceLock<Vec<CompiledMarker>> = OnceLock::new();
    COMPILED
        .get_or_init(|| {
            DEFAULT_CONTENT_MARKERS
                .iter()
                .map(|marker| {
                    let regex = match marker.kind {
                        ContentMatchKind::Regex => Some(
                            Regex::new(marker.pattern).expect("static marker regex must be valid"),
                        ),
                        ContentMatchKind::Prefix | ContentMatchKind::Contains => None,
                    };
                    CompiledMarker { marker, regex }
                })
                .collect()
        })
        .as_slice()
}

/// Limits guarding a content scan of a single file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentScanLimits {
    /// Skip files larger than this (default 1 MiB).
    pub max_file_bytes: u64,
    /// Max bytes read per file (default = max_file_bytes).
    pub max_read_bytes: u64,
    /// Skip likely-binary files (null byte in first 8 KiB).
    pub skip_binary: bool,
}

impl Default for ContentScanLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 1_048_576,
            max_read_bytes: 1_048_576,
            skip_binary: true,
        }
    }
}

/// One content match found in a file, carrying only masked data.
#[derive(Debug, Clone)]
pub struct ContentHit {
    /// Marker id from [`ContentMarker::id`].
    pub marker_id: String,
    /// Human label from [`ContentMarker::label`].
    pub label: String,
    /// Severity from [`ContentMarker::risk`].
    pub risk: SensitiveRisk,
    /// Masked preview of the matched value; never the raw value.
    pub masked: MaskedValue,
    /// Optional line number, 1-based.
    pub line: Option<u32>,
}

/// Best-effort content scan of one regular file.
///
/// Returns `Ok(vec![])` (no error) when the file is **skipped**: it is empty,
/// larger than `limits.max_file_bytes`, or — with `limits.skip_binary` — has a
/// null byte in its first 8 KiB (binary). A `text_only` marker never fires on a
/// binary file because the whole file is skipped before any line is scanned.
/// Open / read failures return [`Error::Io`].
///
/// Scanning is line-oriented: up to `limits.max_read_bytes` bytes are read
/// (`std::fs::File::open` + `Read::take`), lossily converted to UTF-8 and split
/// on `'\n'`; line numbers are 1-based. Per line, every marker fires for every
/// occurrence:
///
/// - [`ContentMatchKind::Prefix`]: the token starting at each occurrence of the
///   prefix, extended with ASCII alphanumeric / `-` / `_` characters.
/// - [`ContentMatchKind::Contains`]: the needle itself, per occurrence.
/// - [`ContentMatchKind::Regex`]: every `Regex::find_iter` match.
///
/// Results are deterministic: lines in ascending order, markers in table order.
pub fn scan_file_content(
    path: &Path,
    limits: &ContentScanLimits,
) -> Result<Vec<ContentHit>, Error> {
    let mut file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let metadata = file.metadata().map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let len = metadata.len();
    if len == 0 || len > limits.max_file_bytes {
        return Ok(Vec::new());
    }

    if limits.skip_binary {
        let sniff_len = std::cmp::min(8192, len) as usize;
        let mut buf = vec![0u8; sniff_len];
        let mut read = 0usize;
        while read < buf.len() {
            match file.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(n) => read += n,
                Err(source) => {
                    return Err(Error::Io {
                        path: path.to_path_buf(),
                        source,
                    })
                }
            }
        }
        if buf[..read].contains(&0) {
            return Ok(Vec::new());
        }
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(0))
            .map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
    }

    let mut content = Vec::new();
    file.take(limits.max_read_bytes)
        .read_to_end(&mut content)
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let text = String::from_utf8_lossy(&content);

    let mut hits: Vec<ContentHit> = Vec::new();
    for (idx, line) in text.split('\n').enumerate() {
        let line_no = (idx + 1) as u32;
        for compiled in compiled_markers() {
            for value in extract_raw_candidates(compiled, line) {
                hits.push(build_hit(compiled.marker, &value, line_no));
            }
        }
    }
    Ok(hits)
}

/// Look up a compiled marker by its stable id.
///
/// Returns `None` when the id is not in [`DEFAULT_CONTENT_MARKERS`]. Used by
/// [`super::reveal`] to re-scan a single marker for a later reveal.
pub(crate) fn compiled_marker_by_id(id: &str) -> Option<&'static CompiledMarker> {
    compiled_markers().iter().find(|c| c.marker.id == id)
}

/// Extract every raw candidate value that `compiled` would match on `line`.
///
/// This is the **single** extraction path shared by the scan (which masks each
/// value via [`mask_secret`]) and reveal (which re-hashes each value). It is
/// deliberately raw — callers must not route the returned strings anywhere that
/// leaks without an explicit opt-in.
pub(crate) fn extract_raw_candidates(compiled: &CompiledMarker, line: &str) -> Vec<String> {
    let marker = compiled.marker;
    match marker.kind {
        ContentMatchKind::Prefix => prefix_tokens(line, marker.pattern),
        ContentMatchKind::Contains => line
            .match_indices(marker.pattern)
            .map(|(_, value)| value.to_string())
            .collect(),
        ContentMatchKind::Regex => compiled
            .regex
            .as_ref()
            .map(|regex| {
                regex
                    .find_iter(line)
                    .map(|matched| matched.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Extract the token starting at every occurrence of `prefix` in `line`.
///
/// The token starts at the prefix occurrence (the prefix itself is included)
/// and extends over consecutive ASCII alphanumeric, `-`, or `_` characters,
/// stopping at any other character. Byte indices from `match_indices` are
/// always char boundaries because every prefix in the static table is ASCII.
fn prefix_tokens(line: &str, prefix: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for (idx, _) in line.match_indices(prefix) {
        let mut token = String::new();
        for ch in line[idx..].chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                token.push(ch);
            } else {
                break;
            }
        }
        tokens.push(token);
    }
    tokens
}

fn build_hit(marker: &ContentMarker, value: &str, line_no: u32) -> ContentHit {
    ContentHit {
        marker_id: marker.id.to_string(),
        label: marker.label.to_string(),
        risk: marker.risk,
        masked: mask_secret(value),
        line: Some(line_no),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ContentScanLimits {
        ContentScanLimits::default()
    }

    #[test]
    fn table_has_expected_rows_in_order() {
        let ids: Vec<&str> = DEFAULT_CONTENT_MARKERS.iter().map(|m| m.id).collect();
        assert_eq!(
            ids,
            vec![
                "aws_access_key",
                "aws_secret_assign",
                "generic_api_key_assign",
                "generic_secret_assign",
                "private_key_header",
                "github_pat",
                "github_oauth",
                "slack_token",
                "stripe_live",
                "stripe_test",
                "connection_string",
                "jwt_like",
            ]
        );
    }

    #[test]
    fn table_ids_are_unique() {
        let mut ids: Vec<&str> = DEFAULT_CONTENT_MARKERS.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), DEFAULT_CONTENT_MARKERS.len());
    }

    #[test]
    fn every_regex_marker_compiles() {
        for marker in DEFAULT_CONTENT_MARKERS {
            if marker.kind == ContentMatchKind::Regex {
                assert!(
                    Regex::new(marker.pattern).is_ok(),
                    "marker {} regex must compile",
                    marker.id
                );
            }
        }
    }

    fn write_fixture(root: &std::path::Path, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = root.join(name);
        std::fs::write(&path, content).expect("write fixture");
        path
    }

    #[test]
    fn aws_access_key_hits_critical() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_fixture(dir.path(), "creds.txt", b"AKIAABCDEFGHIJKLMNOPQRST\n");
        let hits = scan_file_content(&path, &limits()).unwrap();
        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(hit.marker_id, "aws_access_key");
        assert_eq!(hit.risk, SensitiveRisk::Critical);
        assert_eq!(hit.line, Some(1));
        assert!(hit.masked.masked.starts_with("AKIA"));
        assert!(!hit.masked.masked.contains("AKIAABCDEFGHIJKLMNOPQRST"));
    }

    #[test]
    fn pem_header_hits_critical() {
        let dir = tempfile::TempDir::new().unwrap();
        let content = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n";
        let path = write_fixture(dir.path(), "key.pem", content);
        let hits = scan_file_content(&path, &limits()).unwrap();
        let pem: Vec<&ContentHit> = hits
            .iter()
            .filter(|h| h.marker_id == "private_key_header")
            .collect();
        assert_eq!(pem.len(), 1);
        assert_eq!(pem[0].risk, SensitiveRisk::Critical);
        assert_eq!(pem[0].line, Some(1));
    }

    #[test]
    fn too_large_file_is_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_fixture(dir.path(), "big.txt", b"AKIA1234567890123456");
        let limits = ContentScanLimits {
            max_file_bytes: 8,
            max_read_bytes: 8,
            skip_binary: true,
        };
        assert!(scan_file_content(&path, &limits).unwrap().is_empty());
    }

    #[test]
    fn binary_file_is_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_fixture(
            dir.path(),
            "bin.dat",
            &[0u8, 1, 2, 3, b'A', b'K', b'I', b'A'],
        );
        assert!(scan_file_content(&path, &limits()).unwrap().is_empty());
    }

    #[test]
    fn empty_file_is_no_panic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_fixture(dir.path(), "empty.txt", b"");
        assert!(scan_file_content(&path, &limits()).unwrap().is_empty());
    }

    #[test]
    fn prefix_multiple_occurrences_and_word_boundary() {
        assert_eq!(
            prefix_tokens("ghp_abc ghp_xyz", "ghp_"),
            vec!["ghp_abc", "ghp_xyz"]
        );
        assert_eq!(
            prefix_tokens("xoxb-a.xoxb-b", "xoxb-"),
            vec!["xoxb-a", "xoxb-b"]
        );
        assert_eq!(prefix_tokens("AKIA=foo", "AKIA"), vec!["AKIA"]);
    }

    #[test]
    fn non_text_only_markers_still_skip_binary() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_fixture(dir.path(), "bin.bin", &[0u8, b'A', b'K', b'I', b'A']);
        assert!(scan_file_content(&path, &limits()).unwrap().is_empty());
    }
}
