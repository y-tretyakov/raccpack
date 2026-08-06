use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{Error, Result, ScanReport};

/// Schema version of the sniff cache file. Bump on incompatible cache entry
/// format changes; stale caches are then treated as a miss.
const SNIFF_CACHE_SCHEMA: u32 = 1;

/// Cache entry persisted as `{hash}.json`.
///
/// Field names and their meaning are part of the on-disk contract; `created_at`
/// is informational only and is not part of validation.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SniffCacheEntry {
    cache_schema: u32,
    core_version: String,
    root: PathBuf,
    max_depth: usize,
    policy_fingerprint: String,
    created_at: String,
    report: ScanReport,
}

/// Load a cached [`ScanReport`] for `(root, max_depth, policy_fp)`.
///
/// Cache location is XDG cache: `$XDG_CACHE_HOME/raccpack/sniff/{hash}.json`,
/// falling back to `~/.cache/raccpack/sniff/{hash}.json` when
/// `XDG_CACHE_HOME` is unset. The cache is never written into `scan_root`.
/// When HOME / XDG_CACHE_HOME cannot be resolved the cache is simply
/// unavailable.
///
/// A missing, unreadable, malformed or version-mismatched entry is a **miss**:
/// returns `Ok(None)` and never fails the caller.
pub fn try_load_sniff_cache(
    root: &Path,
    max_depth: usize,
    policy_fp: &str,
) -> Result<Option<ScanReport>> {
    let Some(path) = cache_file_path(root, max_depth, policy_fp) else {
        return Ok(None);
    };

    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let Ok(entry) = serde_json::from_str::<SniffCacheEntry>(&content) else {
        return Ok(None);
    };
    if !entry.is_valid_for(root, max_depth, policy_fp) {
        return Ok(None);
    }
    Ok(Some(entry.report))
}

/// Persist `report` to the sniff cache for `(root, max_depth, policy_fp)`.
///
/// Uses the same XDG cache location as [`try_load_sniff_cache`]. Failures to
/// create the cache directory or write the file surface as
/// [`Error::Io`]; callers (`sniff`) are expected to swallow them.
/// Returns `Ok(())` when the cache location cannot be resolved.
pub fn store_sniff_cache(
    root: &Path,
    max_depth: usize,
    policy_fp: &str,
    report: &ScanReport,
) -> Result<()> {
    let Some(path) = cache_file_path(root, max_depth, policy_fp) else {
        return Ok(());
    };

    let entry = SniffCacheEntry {
        cache_schema: SNIFF_CACHE_SCHEMA,
        core_version: crate::core_version().to_string(),
        root: root.to_path_buf(),
        max_depth,
        policy_fingerprint: policy_fp.to_string(),
        created_at: utc_iso8601_now().unwrap_or_default(),
        report: report.clone(),
    };
    let content = serde_json::to_string(&entry).map_err(|source| Error::Other {
        message: source.to_string(),
    })?;

    let dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
        path: dir.clone(),
        source,
    })?;
    std::fs::write(&path, content).map_err(|source| Error::Io { path, source })?;
    Ok(())
}

impl SniffCacheEntry {
    /// Whether this entry matches the requested cache key.
    ///
    /// A mismatch on any of `cache_schema`, `core_version`, `max_depth`,
    /// `policy_fingerprint` or the absolute `root` invalidates the entry.
    fn is_valid_for(&self, root: &Path, max_depth: usize, policy_fp: &str) -> bool {
        self.cache_schema == SNIFF_CACHE_SCHEMA
            && self.core_version == crate::core_version()
            && self.max_depth == max_depth
            && self.policy_fingerprint == policy_fp
            && self.root == root
    }
}

/// Absolute path of the cache file, or `None` when HOME / XDG_CACHE_HOME
/// cannot be resolved.
fn cache_file_path(root: &Path, max_depth: usize, policy_fp: &str) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let key = cache_key(root, max_depth, policy_fp);
    Some(dir.join(format!("{key}.json")))
}

/// Base cache directory `…/raccpack/sniff` per the XDG base directory spec.
///
/// `$XDG_CACHE_HOME` is used only when set to a non-empty absolute path;
/// otherwise `$HOME/.cache` is used.
fn cache_dir() -> Option<PathBuf> {
    if let Some(value) = env::var_os("XDG_CACHE_HOME") {
        let value = PathBuf::from(value);
        if value.is_absolute() && !value.as_os_str().is_empty() {
            return Some(value.join("raccpack").join("sniff"));
        }
    }
    let home = env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".cache")
            .join("raccpack")
            .join("sniff"),
    )
}

/// Deterministic cache key for `(root, max_depth, policy_fp)`.
///
/// FNV-1a 64-bit over `root + "\0" + max_depth + "\0" + policy_fp`. This is a
/// cache key, NOT a security hash — collisions only cause a shared cache slot.
/// `DefaultHasher` is deliberately avoided because it is randomized per
/// process and would break cache hits across runs.
fn cache_key(root: &Path, max_depth: usize, policy_fp: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(128);
    bytes.extend_from_slice(root.to_string_lossy().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(max_depth.to_string().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(policy_fp.as_bytes());
    format!("{:016x}", fnv1a_64(&bytes))
}

/// FNV-1a 64-bit hash.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Current UTC time formatted as ISO-8601 `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Returns `None` when the system clock precedes the Unix epoch.
fn utc_iso8601_now() -> Option<String> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let total_secs = i64::try_from(duration.as_secs()).ok()?;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Days-since-epoch → civil `(year, month, day)`.
///
/// Standard `civil_from_days` algorithm; avoids a chrono/time dependency.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = i64::from(yoe) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2), m, d)
}
