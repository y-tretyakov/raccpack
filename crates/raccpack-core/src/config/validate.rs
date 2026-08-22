//! Pure validation checks applied after config deserialization.

use std::str::FromStr;

use super::ConfigError;

/// `scanner.max_depth` must be at least 1; a value of 0 would make the walker
/// refuse to descend at all.
pub(super) fn validate_max_depth(max_depth: usize) -> Result<(), ConfigError> {
    if max_depth == 0 {
        Err(ConfigError::InvalidMaxDepth { value: max_depth })
    } else {
        Ok(())
    }
}

/// Every `cleanup.enabled_strategies` entry must be a known strategy id
/// (case-insensitive). An unknown id is a strict error (spec §4 recommends
/// Error over silently ignoring).
pub(super) fn validate_enabled_strategies(ids: &[String]) -> Result<(), ConfigError> {
    for id in ids {
        if crate::clean::strategy::StrategyId::from_str_ignore_case(id).is_none() {
            return Err(ConfigError::UnknownCleanupStrategy { id: id.clone() });
        }
    }
    Ok(())
}

/// `detect.mode` in the raw TOML document must name a known pipeline.
///
/// Runs on the raw value **before** typed parsing so an unknown id surfaces as
/// [`ConfigError::UnknownDetectMode`] (with the rejected string) instead of an
/// opaque serde parse error. A missing section/key, or a non-string value, is
/// left to serde: absence means the default applies and type mismatches are
/// reported by the typed parse.
pub(super) fn validate_detect_mode_raw(raw: &toml::Value) -> Result<(), ConfigError> {
    let Some(mode) = raw.get("detect").and_then(|section| section.get("mode")) else {
        return Ok(());
    };
    let Some(text) = mode.as_str() else {
        return Ok(());
    };
    match crate::detect::DetectMode::from_str(text) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConfigError::UnknownDetectMode {
            value: text.to_string(),
        }),
    }
}
