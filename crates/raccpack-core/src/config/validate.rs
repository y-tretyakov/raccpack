//! Pure validation checks applied after config deserialization.

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
