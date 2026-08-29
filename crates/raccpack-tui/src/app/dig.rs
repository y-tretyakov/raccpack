//! Dig screen state (Findings view).
//!
//! Holds only display metadata derived from a dig run. **Raw secret payloads
//! (`content_match` / masked preview) never cross the core boundary into TUI
//! state** — [`FindingRow`] carries no field for them (DoD B1.3). Reveal of the
//! masked value is a later phase (B1.5).

use std::path::PathBuf;
use std::time::SystemTime;

use raccpack_core::app::{DigResult, ProgressEvent, SensitiveFile};
use raccpack_core::domain::SensitiveRisk;
use ratatui::widgets::TableState;

/// A display-only row in the findings table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingRow {
    /// Absolute path of the sensitive file.
    pub path: PathBuf,
    /// Computed severity (maximum over all sources).
    pub risk: SensitiveRisk,
    /// What matched (labels joined, e.g. `.env`, `aws_access_key_id`).
    pub kind: String,
    /// Git status (`tracked`, `untracked`, `ignored`, …); empty when unknown.
    pub git_status: String,
}

/// Minimum severity shown in the findings table; `f` cycles the steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RiskFilter {
    /// Show every finding.
    #[default]
    ShowAll,
    /// Show only `Critical`.
    OnlyCritical,
    /// Show `High` and above.
    HighAndAbove,
    /// Show `Medium` and above.
    MediumAndAbove,
}

impl RiskFilter {
    /// Advance one step (stricter → looser → wraps to all).
    pub fn next(self) -> Self {
        match self {
            Self::ShowAll => Self::OnlyCritical,
            Self::OnlyCritical => Self::HighAndAbove,
            Self::HighAndAbove => Self::MediumAndAbove,
            Self::MediumAndAbove => Self::ShowAll,
        }
    }

    /// Lower bound applied via `risk >= min`; `None` shows everything.
    pub fn min_risk(self) -> Option<SensitiveRisk> {
        match self {
            Self::ShowAll => None,
            Self::OnlyCritical => Some(SensitiveRisk::Critical),
            Self::HighAndAbove => Some(SensitiveRisk::High),
            Self::MediumAndAbove => Some(SensitiveRisk::Medium),
        }
    }

    /// Short label shown in the table title / footer.
    pub fn label(self) -> &'static str {
        match self {
            Self::ShowAll => "all",
            Self::OnlyCritical => "critical",
            Self::HighAndAbove => "high+",
            Self::MediumAndAbove => "medium+",
        }
    }
}

/// State for the dig screen.
#[derive(Debug, Default)]
pub struct DigScreenState {
    /// Every finding from the last successful run, risk desc then path asc.
    /// Never contains secret payloads (see module docs).
    pub all_findings: Vec<FindingRow>,
    /// Visible subset after the [`RiskFilter`]. Identical to `all_findings`
    /// when the filter is `ShowAll`.
    pub findings: Vec<FindingRow>,
    /// Project directory the current run targets. Cleared when the user leaves
    /// the view so re-entry (or `r`) requires a fresh dig.
    pub project: Option<PathBuf>,
    pub is_loading: bool,
    pub error: Option<String>,
    /// Whether file contents are scanned in addition to filenames.
    pub scan_content: bool,
    /// Active severity floor (defaults to show everything).
    pub min_risk: RiskFilter,
    pub table_state: TableState,
    pub progress: Option<ProgressEvent>,
    pub last_run: Option<SystemTime>,
}

impl DigScreenState {
    /// Replace the run result: map core files to display rows and sort
    /// (risk desc, then path asc) before applying the current filter.
    pub fn set_dig_result(&mut self, result: DigResult) {
        let mut rows: Vec<FindingRow> = result.files.iter().map(file_to_row).collect();
        sort_rows(&mut rows);
        self.all_findings = rows;
        self.reapply_filter();
    }

    /// Recompute the visible list and clamp the selection into range.
    pub fn reapply_filter(&mut self) {
        self.findings = match self.min_risk.min_risk() {
            None => self.all_findings.clone(),
            Some(min) => self
                .all_findings
                .iter()
                .filter(|row| row.risk >= min)
                .cloned()
                .collect(),
        };
        self.clamp_selection();
    }

    /// Advance the severity floor and rebuild the visible list.
    pub fn cycle_min_risk(&mut self) {
        self.min_risk = self.min_risk.next();
        self.reapply_filter();
    }

    pub fn select_next(&mut self) {
        let i = self.table_state.selected().unwrap_or(0);
        if i + 1 < self.findings.len() {
            self.table_state.select(Some(i + 1));
        }
    }

    pub fn select_previous(&mut self) {
        let i = self.table_state.selected().unwrap_or(0);
        if i > 0 {
            self.table_state.select(Some(i - 1));
        }
    }

    pub fn select_first(&mut self) {
        if !self.findings.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        let n = self.findings.len();
        if n > 0 {
            self.table_state.select(Some(n - 1));
        }
    }

    pub fn selected_finding(&self) -> Option<&FindingRow> {
        self.table_state
            .selected()
            .and_then(|i| self.findings.get(i))
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
        if loading {
            self.error = None;
        }
    }

    /// Drop the project scope and reset transient state (results are kept).
    pub fn leave(&mut self) {
        self.project = None;
        self.set_loading(false);
        self.progress = None;
        self.table_state = TableState::default();
    }

    pub fn clear(&mut self) {
        self.all_findings.clear();
        self.findings.clear();
        self.project = None;
        self.is_loading = false;
        self.error = None;
        self.min_risk = RiskFilter::ShowAll;
        self.table_state = TableState::default();
        self.progress = None;
        self.last_run = None;
    }

    fn clamp_selection(&mut self) {
        if self.findings.is_empty() {
            self.table_state.select(None);
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state
            .select(Some(i.min(self.findings.len() - 1)));
    }
}

/// Map a core finding to a display row, deliberately dropping `content_match`.
fn file_to_row(file: &SensitiveFile) -> FindingRow {
    FindingRow {
        path: file.path.clone(),
        risk: file.risk,
        kind: file.labels.join(", "),
        git_status: file.git_status.clone().unwrap_or_default(),
    }
}

/// Sort findings for display: risk descending, then path ascending.
fn sort_rows(rows: &mut [FindingRow]) {
    rows.sort_by(|a, b| b.risk.cmp(&a.risk).then_with(|| a.path.cmp(&b.path)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use raccpack_core::app::SensitiveFile;

    fn row(path: &str, risk: SensitiveRisk, git: Option<&str>) -> FindingRow {
        FindingRow {
            path: PathBuf::from(path),
            risk,
            kind: "fixture".to_string(),
            git_status: git.unwrap_or("tracked").to_string(),
        }
    }

    fn sensitive(path: &str, risk: SensitiveRisk, content: bool) -> SensitiveFile {
        SensitiveFile {
            path: PathBuf::from(path),
            risk,
            labels: vec!["fixture".to_string()],
            content_match: content.then(|| raccpack_core::secrets::MaskedValue {
                masked: "SHOULD-NEVER-LEAK<<<".to_string(),
                value_hash: "deadbeef".to_string(),
                original_len: 42,
            }),
            git_status: Some("tracked".to_string()),
        }
    }

    #[test]
    fn file_to_row_never_copies_content_match() {
        let file = sensitive("/repo/.env", SensitiveRisk::Critical, true);
        let row = file_to_row(&file);
        assert_eq!(row.path, PathBuf::from("/repo/.env"));
        assert_eq!(row.risk, SensitiveRisk::Critical);
        assert_eq!(row.kind, "fixture");
        assert_eq!(row.git_status, "tracked");
        // The masked payload must not appear anywhere on the row.
        let debug = format!("{row:?}");
        assert!(
            !debug.contains("SHOULD-NEVER-LEAK"),
            "secret leaked via {debug}"
        );
        assert!(!debug.contains("deadbeef"));
    }

    #[test]
    fn file_to_row_treats_missing_git_as_empty() {
        let mut file = sensitive("/repo/x", SensitiveRisk::Low, false);
        file.git_status = None;
        assert_eq!(file_to_row(&file).git_status, "");
    }

    #[test]
    fn sort_rows_orders_risk_desc_then_path_asc() {
        let mut rows = vec![
            row("/a", SensitiveRisk::Medium, None),
            row("/b", SensitiveRisk::High, None),
            row("/z", SensitiveRisk::Critical, None),
            row("/a", SensitiveRisk::High, None),
        ];
        sort_rows(&mut rows);
        let risks: Vec<SensitiveRisk> = rows.iter().map(|r| r.risk).collect();
        assert_eq!(
            risks,
            vec![
                SensitiveRisk::Critical,
                SensitiveRisk::High,
                SensitiveRisk::High,
                SensitiveRisk::Medium,
            ]
        );
        // Same-risk rows tie-break by path ascending.
        assert_eq!(rows[1].path, PathBuf::from("/a"));
        assert_eq!(rows[2].path, PathBuf::from("/b"));
    }

    #[test]
    fn risk_filter_cycle_wraps() {
        let filter = RiskFilter::ShowAll;
        assert_eq!(filter.next(), RiskFilter::OnlyCritical);
        assert_eq!(filter.next().next(), RiskFilter::HighAndAbove);
        assert_eq!(filter.next().next().next(), RiskFilter::MediumAndAbove);
        assert_eq!(filter.next().next().next().next(), RiskFilter::ShowAll);
    }

    #[test]
    fn risk_filter_labels() {
        assert_eq!(RiskFilter::ShowAll.label(), "all");
        assert_eq!(RiskFilter::OnlyCritical.label(), "critical");
        assert_eq!(RiskFilter::HighAndAbove.label(), "high+");
        assert_eq!(RiskFilter::MediumAndAbove.label(), "medium+");
    }

    #[test]
    fn reapply_filter_filters_and_clamps_selection() {
        let mut state = DigScreenState {
            all_findings: vec![
                row("/high", SensitiveRisk::High, None),
                row("/crit", SensitiveRisk::Critical, None),
                row("/low", SensitiveRisk::Low, None),
            ],
            ..Default::default()
        };
        state.min_risk = RiskFilter::OnlyCritical;
        state.reapply_filter();
        assert_eq!(state.findings.len(), 1);
        assert_eq!(state.findings[0].path, PathBuf::from("/crit"));

        // Selection beyond the visible range is clamped.
        state.table_state.select(Some(5));
        state.min_risk = RiskFilter::ShowAll;
        state.reapply_filter();
        assert_eq!(state.table_state.selected(), Some(2));
    }

    #[test]
    fn reapply_filter_clears_selection_when_empty() {
        let mut state = DigScreenState {
            all_findings: vec![row("/x", SensitiveRisk::Low, None)],
            ..Default::default()
        };
        state.min_risk = RiskFilter::OnlyCritical;
        state.reapply_filter();
        assert!(state.findings.is_empty());
        assert_eq!(state.table_state.selected(), None);
    }

    #[test]
    fn set_dig_result_sorts_and_filters() {
        let mut state = DigScreenState::default();
        let result = DigResult {
            root: PathBuf::from("/repo"),
            files: vec![
                sensitive("/d", SensitiveRisk::High, false),
                sensitive("/a", SensitiveRisk::Critical, true),
            ],
            repeated: vec![],
            duration_ms: 1,
            files_scanned: 10,
        };
        state.set_dig_result(result);
        assert_eq!(state.all_findings.len(), 2);
        assert_eq!(state.findings.len(), 2);
        assert_eq!(state.all_findings[0].path, PathBuf::from("/a"));
        for f in &state.findings {
            assert!(!format!("{f:?}").contains("SHOULD-NEVER-LEAK"));
        }
        state.set_loading(true);
        state.cycle_min_risk();
        state.set_loading(false);
        assert_eq!(state.min_risk, RiskFilter::OnlyCritical);
    }
}
