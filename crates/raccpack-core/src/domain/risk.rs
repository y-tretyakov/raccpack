/// Severity of a sensitive finding.
///
/// Used by `dig`, `stash`, pack deny and exit policy. Ordering is
/// `Critical > High > Medium > Low`; serde names are PascalCase in JSON
/// (`Low`, `Medium`, `High`, `Critical`) — do not change without a
/// breaking-change note.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "PascalCase")]
pub enum SensitiveRisk {
    /// Informational / low confidence.
    Low,
    /// Worth reviewing.
    Medium,
    /// Likely secret; default min for stash.
    High,
    /// Almost certainly credential / key material.
    Critical,
}

impl SensitiveRisk {
    /// Stable string name of the risk level (PascalCase).
    pub fn as_str(self) -> &'static str {
        match self {
            SensitiveRisk::Low => "Low",
            SensitiveRisk::Medium => "Medium",
            SensitiveRisk::High => "High",
            SensitiveRisk::Critical => "Critical",
        }
    }

    /// Parse a risk level from its name, ignoring case.
    pub fn from_str_ignore_case(s: &str) -> Option<Self> {
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "low" => Some(SensitiveRisk::Low),
            "medium" => Some(SensitiveRisk::Medium),
            "high" => Some(SensitiveRisk::High),
            "critical" => Some(SensitiveRisk::Critical),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_puts_critical_on_top() {
        let mut levels = vec![
            SensitiveRisk::Critical,
            SensitiveRisk::Low,
            SensitiveRisk::High,
            SensitiveRisk::Medium,
        ];
        levels.sort();
        assert_eq!(
            levels,
            vec![
                SensitiveRisk::Low,
                SensitiveRisk::Medium,
                SensitiveRisk::High,
                SensitiveRisk::Critical,
            ]
        );
        assert!(SensitiveRisk::Low < SensitiveRisk::Medium);
        assert!(SensitiveRisk::Medium < SensitiveRisk::High);
        assert!(SensitiveRisk::High < SensitiveRisk::Critical);
    }

    #[test]
    fn as_str_matches_variants() {
        assert_eq!(SensitiveRisk::Low.as_str(), "Low");
        assert_eq!(SensitiveRisk::Medium.as_str(), "Medium");
        assert_eq!(SensitiveRisk::High.as_str(), "High");
        assert_eq!(SensitiveRisk::Critical.as_str(), "Critical");
    }

    #[test]
    fn from_str_ignore_case_is_case_insensitive() {
        assert_eq!(
            SensitiveRisk::from_str_ignore_case("low"),
            Some(SensitiveRisk::Low)
        );
        assert_eq!(
            SensitiveRisk::from_str_ignore_case("MEDIUM"),
            Some(SensitiveRisk::Medium)
        );
        assert_eq!(
            SensitiveRisk::from_str_ignore_case("High"),
            Some(SensitiveRisk::High)
        );
        assert_eq!(
            SensitiveRisk::from_str_ignore_case("critical"),
            Some(SensitiveRisk::Critical)
        );
        assert_eq!(SensitiveRisk::from_str_ignore_case("unknown"), None);
    }

    #[test]
    fn serde_roundtrip_uses_pascal_case() {
        for risk in [
            SensitiveRisk::Low,
            SensitiveRisk::Medium,
            SensitiveRisk::High,
            SensitiveRisk::Critical,
        ] {
            let json = serde_json::to_string(&risk).unwrap();
            let back: SensitiveRisk = serde_json::from_str(&json).unwrap();
            assert_eq!(risk, back);
        }
        assert_eq!(
            serde_json::to_string(&SensitiveRisk::High).unwrap(),
            "\"High\""
        );
    }
}
