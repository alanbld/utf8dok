//! Compliance Dashboard - Generates human-readable compliance reports
//!
//! Supports multiple output formats:
//! - HTML dashboards for management and web viewing
//! - Markdown reports for GitHub/GitLab PR comments
//! - JSON output for CI/CD pipeline integration
//!
//! # Example
//!
//! ```ignore
//! let engine = ComplianceEngine::new();
//! let graph = WorkspaceGraph::new();
//! // ... populate graph ...
//!
//! let dashboard = ComplianceDashboard::new(&engine, &graph);
//! let html = dashboard.generate_html();
//! let markdown = dashboard.generate_markdown();
//! let json = dashboard.generate_json();
//! ```

mod html;
mod json;
mod markdown;

pub use html::HtmlGenerator;
pub use json::JsonGenerator;
pub use markdown::MarkdownGenerator;

use super::{ComplianceEngine, ComplianceResult};
use crate::workspace::graph::WorkspaceGraph;

/// Dashboard generator for compliance reports
#[allow(dead_code)]
pub struct ComplianceDashboard<'a> {
    engine: &'a ComplianceEngine,
    graph: &'a WorkspaceGraph,
}

#[allow(dead_code)]
impl<'a> ComplianceDashboard<'a> {
    /// Create a new dashboard generator
    pub fn new(engine: &'a ComplianceEngine, graph: &'a WorkspaceGraph) -> Self {
        Self { engine, graph }
    }

    /// Run compliance checks and get results
    pub fn run_checks(&self) -> ComplianceResult {
        self.engine.run_with_stats(self.graph)
    }

    /// Generate HTML dashboard
    pub fn generate_html(&self) -> String {
        let result = self.run_checks();
        HtmlGenerator::generate(&result, self.engine.rule_descriptions())
    }

    /// Generate Markdown report
    pub fn generate_markdown(&self) -> String {
        let result = self.run_checks();
        MarkdownGenerator::generate(&result, self.engine.rule_descriptions())
    }

    /// Generate JSON report
    pub fn generate_json(&self) -> String {
        let result = self.run_checks();
        JsonGenerator::generate(&result)
    }
}

/// Get current timestamp as ISO 8601 string (without external deps)
pub(crate) fn timestamp() -> String {
    // Use system time for a basic timestamp
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Convert to basic date-time format (UTC)
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Calculate year/month/day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since epoch to year/month/day
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Simplified calculation - not accounting for all edge cases
    let mut remaining_days = days;
    let mut year = 1970i32;

    // Count years
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Count months
    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for &days_in_month in &days_in_months {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    (year, month, (remaining_days + 1) as u32)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        // Should be ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.len() == 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
    }

    #[test]
    fn test_days_to_ymd() {
        // 1970-01-01 is day 0
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 1970-01-02 is day 1
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
    }

    #[test]
    fn test_dashboard_creation() {
        let engine = ComplianceEngine::new();
        let graph = WorkspaceGraph::new();
        let dashboard = ComplianceDashboard::new(&engine, &graph);

        // Should be able to generate all formats without panic
        let _html = dashboard.generate_html();
        let _md = dashboard.generate_markdown();
        let _json = dashboard.generate_json();
    }
}
