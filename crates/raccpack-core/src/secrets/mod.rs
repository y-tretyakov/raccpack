//! Secret detection engine: filename patterns and the risk model.
//!
//! This stage implements name-based secret detection from a static pattern
//! table plus the severity API. Pattern categories covered by
//! [`DEFAULT_FILENAME_PATTERNS`]:
//!
//! - **Environment files**: `.env`, `.env.local`, `.env.production`, `.env.*`
//! - **Private keys / SSH**: `id_rsa`, `id_ed25519`, `id_ecdsa`, `*.pem`,
//!   `*.key`, `*.ppk`
//! - **Keystores / certificates**: `*.p12`, `*.pfx`, `*.jks`
//! - **Registry / config credentials**: `.netrc`, `.npmrc`, `.pypirc`,
//!   `kubeconfig`, `.git-credentials`, `.htpasswd`, Docker `config.json`
//! - **Cloud / service accounts**: `credentials`, `*-service-account*`,
//!   `*-sa.json`, `secrets.{json,yaml,yml}`
//! - **Wallets**: `wallet.dat`
//!
//! Content markers (M3.2) and the facade `dig` use-case (M3.3) build on this
//! module.

pub mod filename;
pub mod finding;
pub mod risk;

pub use filename::{
    match_filename, match_filename_all, scan_filenames, FilenameMatch, FilenamePattern,
    FilenameScanOptions, NameMatchKind, DEFAULT_FILENAME_PATTERNS,
};
pub use finding::{FindingSource, SensitiveFinding};
pub use risk::{upgrade_risk, SensitiveRisk};
