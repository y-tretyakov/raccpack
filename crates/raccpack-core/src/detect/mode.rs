//! Selection of the language/framework detection pipeline.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Detection pipeline used to resolve project languages and frameworks.
///
/// The default [`DetectMode::PriorityTable`] keeps the §4.1 marker priority
/// table behaviour; [`DetectMode::CompositeDag`] selects the experimental
/// composite pipeline ([`super::workspace::WorkspaceDetector`]) that
/// additionally fills `Project.stack_tree`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DetectMode {
    /// Marker priority table — the current, default pipeline.
    #[default]
    PriorityTable,
    /// Composite DAG pipeline (experimental, Detect v2).
    CompositeDag,
}

impl DetectMode {
    /// Canonical TOML / CLI string for this mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PriorityTable => "priority_table",
            Self::CompositeDag => "composite_dag",
        }
    }
}

impl fmt::Display for DetectMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string does not name a known [`DetectMode`].
///
/// The payload is the rejected input, ready for a typed config error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModeError(pub String);

impl FromStr for DetectMode {
    type Err = UnknownModeError;

    /// Parses the same canonical strings the serde impls accept:
    /// `"priority_table"` and `"composite_dag"`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "priority_table" => Ok(Self::PriorityTable),
            "composite_dag" => Ok(Self::CompositeDag),
            other => Err(UnknownModeError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_priority_table() {
        assert_eq!(DetectMode::default(), DetectMode::PriorityTable);
    }

    #[test]
    fn as_str_matches_canonical_names() {
        assert_eq!(DetectMode::PriorityTable.as_str(), "priority_table");
        assert_eq!(DetectMode::CompositeDag.as_str(), "composite_dag");
        assert_eq!(DetectMode::PriorityTable.to_string(), "priority_table");
    }

    #[test]
    fn from_str_parses_canonical_names_only() {
        assert_eq!(
            "priority_table".parse::<DetectMode>(),
            Ok(DetectMode::PriorityTable)
        );
        assert_eq!(
            "composite_dag".parse::<DetectMode>(),
            Ok(DetectMode::CompositeDag)
        );
        assert!("dag".parse::<DetectMode>().is_err());
        assert!("bogus".parse::<DetectMode>().is_err());
    }

    #[test]
    fn from_str_error_carries_the_rejected_value() {
        let err = "nope".parse::<DetectMode>().unwrap_err();
        assert_eq!(err.0, "nope");
    }

    #[test]
    fn serde_round_trip_uses_snake_case_strings() {
        assert_eq!(
            serde_json::to_value(DetectMode::PriorityTable).unwrap(),
            serde_json::json!("priority_table")
        );
        assert_eq!(
            serde_json::to_value(DetectMode::CompositeDag).unwrap(),
            serde_json::json!("composite_dag")
        );
        let de = |text: &str| serde_json::from_str::<DetectMode>(text).ok();
        assert_eq!(de("\"priority_table\""), Some(DetectMode::PriorityTable));
        assert_eq!(de("\"composite_dag\""), Some(DetectMode::CompositeDag));
    }

    #[test]
    fn serde_rejects_unknown_and_alias_strings() {
        assert!(serde_json::from_str::<DetectMode>("\"bogus\"").is_err());
        assert!(serde_json::from_str::<DetectMode>("\"dag\"").is_err());
    }
}
