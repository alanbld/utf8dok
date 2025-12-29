//! Status Rule for Bridge Framework
//!
//! Validates that superseded ADRs have the correct status (Deprecated or Superseded).

use tower_lsp::lsp_types::{Position, Range, Url};

use crate::compliance::{ComplianceRule, Violation, ViolationSeverity};
use crate::config::Settings;
use crate::workspace::graph::WorkspaceGraph;

/// Rule: When an ADR claims to supersede another, the superseded ADR
/// must have status "Deprecated" or "Superseded".
#[allow(dead_code)]
pub struct StatusRule {
    /// Severity level for violations (None = disabled)
    severity: Option<ViolationSeverity>,
}

#[allow(dead_code)]
impl StatusRule {
    pub fn new() -> Self {
        Self {
            severity: Some(ViolationSeverity::Error),
        }
    }

    /// Create a StatusRule configured from settings
    pub fn with_settings(settings: &Settings) -> Self {
        Self {
            severity: settings.compliance.bridge.superseded_status.to_violation_severity(),
        }
    }

    /// Parse the supersedes attribute value into individual IDs
    fn parse_supersedes(value: &str) -> Vec<String> {
        value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Check if a status value is valid for a superseded document
    fn is_valid_superseded_status(status: &str) -> bool {
        let status_lower = status.to_lowercase();
        status_lower == "deprecated" || status_lower == "superseded"
    }
}

impl Default for StatusRule {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceRule for StatusRule {
    fn check(&self, graph: &WorkspaceGraph) -> Vec<Violation> {
        // If rule is disabled, return no violations
        let severity = match self.severity {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut violations = Vec::new();

        // Iterate all documents looking for :supersedes: attribute
        for uri in graph.document_uris() {
            if let Some(supersedes_value) = graph.get_document_attribute(uri, "supersedes") {
                let superseded_ids = Self::parse_supersedes(supersedes_value);

                for superseded_id in superseded_ids {
                    // Find the document that defines this ID
                    if let Some(def_uri) = graph.get_definition_uri(&superseded_id) {
                        let def_uri_str = def_uri.as_str();

                        // Check the status of the superseded document
                        if let Some(status) = graph.get_document_attribute(def_uri_str, "status") {
                            if !Self::is_valid_superseded_status(status) {
                                // Create violation at the superseding document
                                let parsed_uri = Url::parse(uri).unwrap_or_else(|_| {
                                    Url::parse("file:///unknown").unwrap()
                                });

                                violations.push(Violation {
                                    uri: parsed_uri,
                                    range: Range {
                                        start: Position { line: 0, character: 0 },
                                        end: Position { line: 0, character: 0 },
                                    },
                                    message: format!(
                                        "Superseded document '{}' has status '{}' but must be Deprecated or Superseded",
                                        superseded_id, status
                                    ),
                                    severity,
                                    code: "BRIDGE001".to_string(),
                                });
                            }
                        } else {
                            // No status attribute - also a violation (should have status)
                            let parsed_uri = Url::parse(uri).unwrap_or_else(|_| {
                                Url::parse("file:///unknown").unwrap()
                            });

                            violations.push(Violation {
                                uri: parsed_uri,
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                                message: format!(
                                    "Superseded document '{}' has no :status: attribute; it must be Deprecated or Superseded",
                                    superseded_id
                                ),
                                severity: ViolationSeverity::Warning,
                                code: "BRIDGE001".to_string(),
                            });
                        }
                    } else {
                        // Supersedes a non-existent ID
                        let parsed_uri = Url::parse(uri).unwrap_or_else(|_| {
                            Url::parse("file:///unknown").unwrap()
                        });

                        violations.push(Violation {
                            uri: parsed_uri,
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 0 },
                            },
                            message: format!(
                                "Document claims to supersede '{}' but that ID is not defined in the workspace",
                                superseded_id
                            ),
                            severity: ViolationSeverity::Warning,
                            code: "BRIDGE002".to_string(),
                        });
                    }
                }
            }
        }

        violations
    }

    fn code(&self) -> &'static str {
        "BRIDGE001"
    }

    fn description(&self) -> &'static str {
        "Superseded ADRs must have status Deprecated or Superseded"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_supersedes_single() {
        let ids = StatusRule::parse_supersedes("adr-001");
        assert_eq!(ids, vec!["adr-001"]);
    }

    #[test]
    fn test_parse_supersedes_multiple() {
        let ids = StatusRule::parse_supersedes("adr-001, adr-002, adr-003");
        assert_eq!(ids, vec!["adr-001", "adr-002", "adr-003"]);
    }

    #[test]
    fn test_is_valid_superseded_status() {
        assert!(StatusRule::is_valid_superseded_status("Deprecated"));
        assert!(StatusRule::is_valid_superseded_status("deprecated"));
        assert!(StatusRule::is_valid_superseded_status("Superseded"));
        assert!(StatusRule::is_valid_superseded_status("superseded"));
        assert!(!StatusRule::is_valid_superseded_status("Accepted"));
        assert!(!StatusRule::is_valid_superseded_status("Proposed"));
    }
}
