//! Severity helpers around [`SensitiveRisk`].
//!
//! [`SensitiveRisk`] lives in `domain`; this module re-exports it and adds the
//! severity API. Risk levels only ever upgrade via [`upgrade_risk`] (max), and
//! [`SensitiveRisk::at_least`] is the standard guard for minimum-risk filters.

pub use crate::domain::SensitiveRisk;

impl SensitiveRisk {
    /// Whether `self` meets or exceeds the given minimum severity.
    pub fn at_least(self, min: Self) -> bool {
        self >= min
    }
}

/// Merge two risks into the higher of the two.
///
/// Severity only ever upgrades (max), never downgrades. This is the single
/// sanctioned place risk levels are combined; content markers (M3.2) upgrade a
/// filename finding's base risk through this function.
pub fn upgrade_risk(a: SensitiveRisk, b: SensitiveRisk) -> SensitiveRisk {
    a.max(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_risk_returns_max() {
        assert_eq!(
            upgrade_risk(SensitiveRisk::High, SensitiveRisk::Critical),
            SensitiveRisk::Critical
        );
        assert_eq!(
            upgrade_risk(SensitiveRisk::Critical, SensitiveRisk::Low),
            SensitiveRisk::Critical
        );
        assert_eq!(
            upgrade_risk(SensitiveRisk::Medium, SensitiveRisk::Medium),
            SensitiveRisk::Medium
        );
    }

    #[test]
    fn at_least_is_inclusive_threshold() {
        assert!(SensitiveRisk::High.at_least(SensitiveRisk::Low));
        assert!(SensitiveRisk::High.at_least(SensitiveRisk::High));
        assert!(!SensitiveRisk::High.at_least(SensitiveRisk::Critical));
    }
}
