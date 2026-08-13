//! Secret detection engine: filename patterns, content markers, masking, and
//! the risk model.
//!
//! This module implements secret detection from static pattern tables plus the
//! severity API:
//!
//! - **Filename patterns** ([`DEFAULT_FILENAME_PATTERNS`]) match names like
//!   `.env`, `id_rsa`, `*.pem`, `.netrc`, `secrets.json` and more — see
//!   [`filename`].
//! - **Content markers** ([`DEFAULT_CONTENT_MARKERS`]) match file contents with
//!   prefix / contains / regex rules (AWS keys, PEM headers, GitHub tokens,
//!   Stripe keys, connection strings, JWT-like tokens) guarded by size and
//!   binary limits — see [`content`].
//! - **Masking** ([`mask`]) turns any raw value into a public-safe
//!   [`MaskedValue`]: a short preview, a stable blake3 `value_hash`, and a byte
//!   length. Raw values never appear in results, logs, or Debug output.
//! - **Combined scan** ([`scan_secrets`]) merges filename + content detections
//!   per path, upgrading risk via [`upgrade_risk`].
//!
//! The facade `dig` use-case (M3.3) builds on [`scan`].

pub mod content;
pub mod filename;
pub mod finding;
pub mod mask;
pub mod risk;
pub mod scan;

pub use content::{
    scan_file_content, ContentHit, ContentMarker, ContentMatchKind, ContentScanLimits,
    DEFAULT_CONTENT_MARKERS,
};
pub use filename::{
    match_filename, match_filename_all, scan_filenames, FilenameMatch, FilenamePattern,
    FilenameScanOptions, NameMatchKind, DEFAULT_FILENAME_PATTERNS,
};
pub use finding::{FindingSource, SensitiveFinding};
pub use mask::{fingerprint_secret, mask_secret, MaskedValue};
pub use risk::{upgrade_risk, SensitiveRisk};
pub use scan::{scan_secrets, SecretScanOptions};
