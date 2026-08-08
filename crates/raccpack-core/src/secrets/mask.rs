//! Masking and fingerprinting for raw secret values.
//!
//! Raw values never leave `raccpack-core`; every public-facing preview goes
//! through [`mask_secret`], which produces a [`MaskedValue`] containing only a
//! short preview, a stable hash, and a byte length. This is the single place a
//! raw value is transformed for reports — nothing else must format it.

/// Public-safe preview of a secret value.
///
/// Never contains the full raw value. Serde-serializable so it can travel to
/// CLI / TUI / Desktop DTOs without leaking the secret.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaskedValue {
    /// e.g. `"AKIA…23"` or `"****"`.
    pub masked: String,
    /// Stable hash for repeated-secret detection (blake3 hex of raw). Never raw.
    pub value_hash: String,
    /// Length of original value in BYTES (not chars).
    pub original_len: usize,
}

/// Mask a secret for reports + fingerprint.
///
/// Masking rules (fixed):
///
/// - `raw.len() <= 8` bytes → `masked = "****"` (includes the empty string).
/// - `raw.len() > 8` bytes → `masked = first 4 chars + "…" (U+2026) + last 2
///   chars`. Slicing is char-based so multi-byte UTF-8 never panics.
///
/// The masked preview can never contain the full raw value.
pub fn mask_secret(raw: &str) -> MaskedValue {
    let original_len = raw.len();
    let masked = if raw.len() <= 8 {
        "****".to_string()
    } else {
        let head: String = raw.chars().take(4).collect();
        let tail: String = raw
            .chars()
            .rev()
            .take(2)
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}…{tail}")
    };
    MaskedValue {
        masked,
        value_hash: fingerprint_secret(raw),
        original_len,
    }
}

/// Stable hash (blake3 hex) of a raw secret value.
///
/// Same input always produces the same lowercase hex digest; different inputs
/// produce (with overwhelming probability) different digests. Used to detect
/// repeated secrets across files without ever storing the raw value.
pub fn fingerprint_secret(raw: &str) -> String {
    blake3::hash(raw.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_values_mask_to_stars() {
        for raw in ["", "a", "abcd", "abcdefgh"] {
            let m = mask_secret(raw);
            assert_eq!(m.masked, "****", "len {} must mask to ****", raw.len());
            assert_eq!(m.original_len, raw.len());
        }
    }

    #[test]
    fn longer_values_keep_head_and_tail() {
        assert_eq!(mask_secret("abcdefghi").masked, "abcd…hi");
        assert_eq!(mask_secret("123456789012345").masked, "1234…45");
        assert_eq!(mask_secret("12345678901234567890").masked, "1234…90");
    }

    #[test]
    fn multibyte_values_never_panic() {
        let raw = "ключсекрет";
        let m = mask_secret(raw);
        assert!(!m.masked.contains("секрет"));
        assert_eq!(m.masked.chars().count(), 4 + 1 + 2);
    }

    #[test]
    fn masked_never_contains_full_raw() {
        for raw in [
            "x",
            "short",
            "AKIAABCDEFGHIJKLMNOPQRST",
            "12345678901234567890",
        ] {
            let m = mask_secret(raw);
            assert!(
                !m.masked.contains(raw),
                "masked {m:?} must not contain full raw {raw:?}"
            );
        }
        assert_eq!(mask_secret("").masked, "****");
    }

    #[test]
    fn original_len_is_bytes() {
        assert_eq!(mask_secret("éééé").original_len, 8);
        assert_eq!(mask_secret("abcdef").original_len, 6);
    }

    #[test]
    fn fingerprint_is_deterministic() {
        assert_eq!(fingerprint_secret("abc"), fingerprint_secret("abc"));
        assert_eq!(mask_secret("same").value_hash, fingerprint_secret("same"));
    }

    #[test]
    fn fingerprint_differs_for_different_inputs() {
        assert_ne!(fingerprint_secret("abc"), fingerprint_secret("abd"));
        assert_ne!(mask_secret("a").value_hash, mask_secret("aa").value_hash);
    }
}
