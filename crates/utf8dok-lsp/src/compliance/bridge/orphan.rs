//! Orphan Rule for Bridge Framework
//!
//! Detects documents that are not reachable from any entry point (index, README).

use tower_lsp::lsp_types::{CodeActionKind, Position, Range, TextEdit, Url};

use crate::compliance::actions::{find_index_insert_position, generate_orphan_link, ComplianceFix};
use crate::compliance::{ComplianceRule, Violation, ViolationSeverity};
use crate::config::Settings;
use crate::workspace::graph::WorkspaceGraph;

/// Rule: All documents should be reachable from an entry point (index, README).
/// Orphaned documents are those with no incoming references.
#[allow(dead_code)]
pub struct OrphanRule {
    /// Known entry point patterns (case-insensitive matching on filename)
    entry_point_patterns: Vec<&'static str>,
    /// Severity level for violations (None = disabled)
    severity: Option<ViolationSeverity>,
}

#[allow(dead_code)]
impl OrphanRule {
    pub fn new() -> Self {
        Self {
            entry_point_patterns: vec![
                "index.adoc",
                "readme.adoc",
                "index.asciidoc",
                "readme.asciidoc",
                "readme.md",
                "index.md",
            ],
            severity: Some(ViolationSeverity::Warning),
        }
    }

    /// Create an OrphanRule configured from settings
    pub fn with_settings(settings: &Settings) -> Self {
        Self {
            entry_point_patterns: vec![
                "index.adoc",
                "readme.adoc",
                "index.asciidoc",
                "readme.asciidoc",
                "readme.md",
                "index.md",
            ],
            severity: settings.compliance.bridge.orphans.to_violation_severity(),
        }
    }

    /// Check if a URI is an entry point
    fn is_entry_point(&self, uri: &str) -> bool {
        let uri_lower = uri.to_lowercase();
        self.entry_point_patterns
            .iter()
            .any(|pattern| uri_lower.ends_with(pattern))
    }

    /// Find all entry point URIs in the graph
    fn find_entry_points<'a>(&self, graph: &'a WorkspaceGraph) -> Vec<&'a str> {
        graph
            .document_uris()
            .into_iter()
            .filter(|uri| self.is_entry_point(uri))
            .map(|s| s.as_str())
            .collect()
    }
}

impl Default for OrphanRule {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceRule for OrphanRule {
    fn check(&self, graph: &WorkspaceGraph) -> Vec<Violation> {
        // If rule is disabled, return no violations
        let severity = match self.severity {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut violations = Vec::new();

        // Find entry points
        let entry_points = self.find_entry_points(graph);

        // If no entry points, we can't determine orphans
        if entry_points.is_empty() {
            return violations;
        }

        // Find all reachable documents
        let reachable = graph.find_reachable_documents(&entry_points);

        // Check each document
        for uri in graph.document_uris() {
            // Skip entry points themselves
            if self.is_entry_point(uri) {
                continue;
            }

            // Check if this document is reachable
            if !reachable.contains(uri) {
                let parsed_uri =
                    Url::parse(uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap());

                // Extract filename for better error message
                let filename = uri.rsplit('/').next().unwrap_or(uri);

                violations.push(Violation {
                    uri: parsed_uri,
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                    message: format!(
                        "Orphaned document: '{}' is not reachable from any entry point (index.adoc, README.adoc)",
                        filename
                    ),
                    severity,
                    code: "BRIDGE003".to_string(),
                });
            }
        }

        violations
    }

    fn code(&self) -> &'static str {
        "BRIDGE003"
    }

    fn description(&self) -> &'static str {
        "All documents should be reachable from an entry point"
    }

    fn fix(&self, violation: &Violation, graph: &WorkspaceGraph) -> Option<ComplianceFix> {
        // Only fix BRIDGE003 violations
        if violation.code != "BRIDGE003" {
            return None;
        }

        // Get the orphan document URI from the violation
        let orphan_uri = violation.uri.as_str();

        // Find entry points
        let entry_points = self.find_entry_points(graph);
        let index_uri = entry_points.first()?;

        // Get the index document text
        let index_text = graph.get_document_text(index_uri)?;

        // Find where to insert the link
        let insert_pos = find_index_insert_position(index_text);

        // Generate the link text
        let link_text = generate_orphan_link(orphan_uri);

        // Extract filename for the title
        let filename = orphan_uri.rsplit('/').next().unwrap_or("document.adoc");

        // Create the fix
        Some(ComplianceFix {
            title: format!("Add link to '{}' in index", filename),
            uri: Url::parse(index_uri).ok()?,
            edits: vec![TextEdit {
                range: insert_pos,
                new_text: link_text,
            }],
            kind: CodeActionKind::QUICKFIX,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_entry_point() {
        let rule = OrphanRule::new();

        assert!(rule.is_entry_point("file:///project/index.adoc"));
        assert!(rule.is_entry_point("file:///project/README.adoc"));
        assert!(rule.is_entry_point("file:///project/INDEX.ADOC")); // case insensitive
        assert!(!rule.is_entry_point("file:///project/adr-001.adoc"));
    }

    #[test]
    fn test_find_entry_points() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "= Index");
        graph.add_document("file:///adr-001.adoc", "= ADR 001");

        let rule = OrphanRule::new();
        let entry_points = rule.find_entry_points(&graph);

        assert_eq!(entry_points.len(), 1);
        assert!(entry_points[0].contains("index.adoc"));
    }
}
