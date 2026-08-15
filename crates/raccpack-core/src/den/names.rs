//! Den naming conventions (facade-and-den §9.2).
//!
//! - [`project_slug`]: sanitizes a project name or path into `[a-zA-Z0-9._-]`
//!   with whitespace collapsed to `-`, capped at 80 chars; empty input falls
//!   back to `"project"`.
//! - [`utc_timestamp_now`]: compact UTC `YYYYMMDDThhmmssZ` computed from the
//!   system clock without an external date crate.
//! - [`short_id`]: 8 lowercase hex chars from a blake3 hash of clock nanos and
//!   a process address — high uniqueness, no determinism required.
//! - [`pack_relative_path`] yields `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`.
//! - [`secrets_relative_path`] yields
//!   `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age`.
//!
//! INVARIANT: every function is deterministic where required, never panics
//! (short/malformed `ts` falls back to `"0000"` / `"00"`), and produces ASCII
//! names safe to embed in filesystem paths.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum slug length in characters (output is pure ASCII, so chars == bytes).
const MAX_SLUG_LEN: usize = 80;

/// Fallback slug when sanitization yields an empty result.
const FALLBACK_SLUG: &str = "project";

/// Process-unique seed so two processes calling [`short_id`] in the same
/// nanosecond still differ.
static SHORT_ID_SEED: u8 = 0;

/// Sanitize a project name or path into a den-safe slug.
///
/// If the input contains a path separator, only the last path component is
/// used (lossy for non-UTF8). Allowed characters are ASCII alphanumerics plus
/// `.`, `_`, `-`; whitespace becomes `-`; every other character is dropped.
/// The result is truncated to 80 chars and never empty — an empty sanitized
/// result falls back to `"project"`.
pub fn project_slug(name_or_path: &str) -> String {
    let last = name_or_path
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(name_or_path);

    let mut slug = String::with_capacity(last.len());
    for ch in last.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            slug.push(ch);
        } else if ch.is_whitespace() {
            slug.push('-');
        }
        if slug.len() >= MAX_SLUG_LEN {
            break;
        }
    }

    if slug.is_empty() {
        FALLBACK_SLUG.to_string()
    } else {
        slug
    }
}

/// Current UTC time as compact `YYYYMMDDThhmmssZ`.
///
/// Computed from [`SystemTime`] + [`UNIX_EPOCH`] with an internal civil-date
/// conversion (no chrono dependency). When the system clock precedes the Unix
/// epoch, falls back to the epoch instant (`19700101T000000Z`) — never panics.
pub fn utc_timestamp_now() -> String {
    let total_secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(_) => 0,
    };
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    let (year, month, day) = days_to_civil(days);
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

/// 8 lowercase hex characters with high uniqueness.
///
/// First 8 hex chars of a blake3 hash of (clock nanoseconds, process seed
/// address). Uniqueness does not rely on determinism.
pub fn short_id() -> String {
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    let seed_addr = std::ptr::addr_of!(SHORT_ID_SEED) as usize;

    let mut hasher = blake3::Hasher::new();
    hasher.update(&nanos.to_le_bytes());
    hasher.update(&u64::try_from(seed_addr).unwrap_or(0).to_le_bytes());
    let hex = hasher.finalize().to_hex();
    hex[..8].to_string()
}

/// Relative den path for a pack: `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`.
///
/// `yyyy` = `ts[0..4]`, `mm` = `ts[4..6]`. A `ts` shorter than 6 chars falls
/// back to `"0000"` / `"00"` so this never panics.
pub fn pack_relative_path(slug: &str, ts: &str) -> PathBuf {
    let yyyy = ts.get(..4).unwrap_or("0000");
    let mm = ts.get(4..6).unwrap_or("00");
    PathBuf::from("packs")
        .join(yyyy)
        .join(mm)
        .join(format!("{slug}__{ts}.tar.zst"))
}

/// Relative den path for a stash: `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age`.
///
/// `yyyy` = `ts[0..4]`, `mm` = `ts[4..6]`. A `ts` shorter than 6 chars falls
/// back to `"0000"` / `"00"` so this never panics.
pub fn secrets_relative_path(slug: &str, ts: &str) -> PathBuf {
    secrets_relative_path_token(slug, ts, ts)
}

/// [`secrets_relative_path`] with a custom name token in place of `ts`.
///
/// The `yyyy`/`mm` directory segments are still derived from the real
/// timestamp `ts` (which keeps its `YYYYMMDD…` shape), while the artifact
/// filename becomes `{slug}__{token}__secrets.age` — used for `--batch-id`.
/// `token` must be a safe fragment (no path separators); validation is the
/// caller's responsibility.
pub fn secrets_relative_path_token(slug: &str, ts: &str, token: &str) -> PathBuf {
    let yyyy = ts.get(..4).unwrap_or("0000");
    let mm = ts.get(4..6).unwrap_or("00");
    PathBuf::from("secrets")
        .join(yyyy)
        .join(mm)
        .join(format!("{slug}__{token}__secrets.age"))
}

/// Days since 1970-01-01 → civil `(year, month, day)` (1-indexed month/day).
///
/// Uses `days_before_year` (proleptic Gregorian) with a quick convergence
/// loop; differs from the similar helper in `cache/sniff_cache.rs`.
fn days_to_civil(days: i64) -> (i64, u32, u32) {
    let absolute = days + days_before_year(1970);
    let mut year = 1970 + days.div_euclid(366);
    loop {
        let start = days_before_year(year);
        if absolute < start {
            year -= 1;
        } else if absolute >= days_before_year(year + 1) {
            year += 1;
        } else {
            break;
        }
    }

    let doy = absolute - days_before_year(year);
    let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    const MONTH_DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut remaining = doy;
    let mut month = 0usize;
    while remaining >= MONTH_DAYS[month] + i64::from(month == 1 && is_leap) {
        remaining -= MONTH_DAYS[month] + i64::from(month == 1 && is_leap);
        month += 1;
    }
    (year, month as u32 + 1, remaining as u32 + 1)
}

/// Days from 0001-01-01 (proleptic Gregorian) to the start of `year`.
fn days_before_year(year: i64) -> i64 {
    let y = year - 1;
    365 * y + y / 4 - y / 100 + y / 400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_sanitizes_whitespace_and_invalid_chars() {
        assert_eq!(project_slug("My App!"), "My-App");
    }

    #[test]
    fn slug_takes_basename_of_path() {
        assert_eq!(project_slug("/home/user/proj/my-api"), "my-api");
        assert_eq!(project_slug("C:\\work\\My Project"), "My-Project");
    }

    #[test]
    fn slug_keeps_dots_underscores_dashes() {
        assert_eq!(project_slug("my.api_v1-2"), "my.api_v1-2");
    }

    #[test]
    fn slug_falls_back_to_project() {
        assert_eq!(project_slug(""), "project");
        assert_eq!(project_slug("!!!"), "project");
        assert_eq!(project_slug("///"), "project");
    }

    #[test]
    fn slug_is_capped_at_80() {
        let long = "a".repeat(200);
        assert_eq!(project_slug(&long).len(), 80);
    }

    #[test]
    fn timestamp_has_compact_utc_shape() {
        let ts = utc_timestamp_now();
        assert_eq!(ts.len(), 16);
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.as_bytes()[8], b'T');
        let digits: String = ts.chars().filter(|c| c.is_ascii_digit()).collect();
        assert_eq!(digits.len(), 14);
    }

    #[test]
    fn timestamp_year_is_sane() {
        let ts = utc_timestamp_now();
        let year: i64 = ts[..4].parse().unwrap();
        assert!((1970..=2100).contains(&year));
    }

    #[test]
    fn short_id_is_eight_hex_chars() {
        let id = short_id();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn pack_path_derives_year_month() {
        assert_eq!(
            pack_relative_path("my-api", "20260804T155230Z"),
            PathBuf::from("packs/2026/08/my-api__20260804T155230Z.tar.zst")
        );
    }

    #[test]
    fn pack_path_never_panics_on_short_ts() {
        let path = pack_relative_path("s", "12");
        assert!(path.to_string_lossy().starts_with("packs/"));
    }

    #[test]
    fn secrets_path_derives_year_month_and_suffix() {
        assert_eq!(
            secrets_relative_path("my-api", "20260804T155230Z"),
            PathBuf::from("secrets/2026/08/my-api__20260804T155230Z__secrets.age")
        );
    }

    #[test]
    fn secrets_path_token_keeps_dirs_from_ts() {
        assert_eq!(
            secrets_relative_path_token("my-api", "20260804T155230Z", "nightly"),
            PathBuf::from("secrets/2026/08/my-api__nightly__secrets.age")
        );
    }

    #[test]
    fn secrets_path_never_panics_on_short_ts() {
        let path = secrets_relative_path("s", "12");
        assert!(path.to_string_lossy().starts_with("secrets/"));
    }
}
