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
