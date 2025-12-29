//! Bridge Framework Domain Plugin
//!
//! Provides domain intelligence for Bridge Framework documents,
//! including ADRs (Architecture Decision Records).

use crate::domain::traits::DocumentDomain;
use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, Documentation,
    MarkupContent, MarkupKind, Position, Range, SemanticTokenType,
};

/// Valid ADR status values
const VALID_STATUSES: &[&str] = &[
    "Draft",
    "Proposed",
    "Accepted",
    "Rejected",
    "Deprecated",
    "Superseded",
];

/// Required ADR sections
const REQUIRED_SECTIONS: &[&str] = &["Context", "Decision", "Consequences"];

/// Bridge Framework domain plugin
pub struct BridgePlugin;

impl BridgePlugin {
    pub fn new() -> Self {
        Self
    }

    /// Check if document looks like an ADR
    fn is_adr_document(&self, text: &str) -> bool {
        static ADR_TITLE_RE: OnceLock<Regex> = OnceLock::new();
        let adr_title_re =
            ADR_TITLE_RE.get_or_init(|| Regex::new(r"(?i)^=\s*ADR[\s\-]?\d+").unwrap());

        // Check for ADR title pattern
        if adr_title_re.is_match(text) {
            return true;
        }

        // Check for status attribute with valid ADR value
        static STATUS_RE: OnceLock<Regex> = OnceLock::new();
        let status_re = STATUS_RE.get_or_init(|| {
            Regex::new(r"(?i):status:\s*(Draft|Proposed|Accepted|Rejected|Deprecated|Superseded)")
                .unwrap()
        });

        if status_re.is_match(text) {
            return true;
        }

        // Check for ADR-like structure
        let has_context = text.contains("== Context") || text.contains("=== Context");
        let has_decision = text.contains("== Decision") || text.contains("=== Decision");
        let has_consequences =
            text.contains("== Consequences") || text.contains("=== Consequences");

        has_decision && (has_context || has_consequences)
    }

    /// Validate ADR sections
    fn validate_sections(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for section in REQUIRED_SECTIONS {
            let section_pattern = format!("== {}", section);
            let alt_pattern = format!("=== {}", section);

            if !text.contains(&section_pattern) && !text.contains(&alt_pattern) {
                let last_line = text.lines().count().saturating_sub(1) as u32;
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: last_line,
                            character: 0,
                        },
                        end: Position {
                            line: last_line,
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(tower_lsp::lsp_types::NumberOrString::String(
                        "ADR001".to_string(),
                    )),
                    source: Some("utf8dok-bridge".to_string()),
                    message: format!("Missing required ADR section: '{}'", section),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    /// Validate status attribute value
    fn validate_status(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        static STATUS_RE: OnceLock<Regex> = OnceLock::new();
        let status_re = STATUS_RE.get_or_init(|| Regex::new(r"^:status:\s*(\S+)").unwrap());

        for (line_num, line) in text.lines().enumerate() {
            if let Some(cap) = status_re.captures(line.trim()) {
                let status = cap.get(1).unwrap().as_str();
                if !VALID_STATUSES
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(status))
                {
                    let start_char = line.find(status).unwrap_or(0);
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: start_char as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (start_char + status.len()) as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(tower_lsp::lsp_types::NumberOrString::String(
                            "ADR002".to_string(),
                        )),
                        source: Some("utf8dok-bridge".to_string()),
                        message: format!(
                            "Invalid status value '{}'. Expected one of: {}",
                            status,
                            VALID_STATUSES.join(", ")
                        ),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    /// Get status value completions
    fn complete_status_values(&self) -> Vec<CompletionItem> {
        VALID_STATUSES
            .iter()
            .map(|status| CompletionItem {
                label: status.to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(format!("ADR status: {}", status)),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: match *status {
                        "Draft" => "Initial state, under discussion",
                        "Proposed" => "Ready for review and decision",
                        "Accepted" => "Decision has been approved",
                        "Rejected" => "Decision was not approved",
                        "Deprecated" => "No longer applicable",
                        "Superseded" => "Replaced by another ADR",
                        _ => "ADR status value",
                    }
                    .to_string(),
                })),
                ..Default::default()
            })
            .collect()
    }

    /// Get attribute name completions
    fn complete_attribute_names(&self, prefix: &str) -> Vec<CompletionItem> {
        let attributes = [
            ("status", "ADR lifecycle status"),
            ("author", "Document author"),
            ("date", "Creation or decision date"),
            ("context", "Background context"),
            ("decision", "The decision made"),
            ("consequences", "Impact of the decision"),
            ("decision-drivers", "Factors driving the decision"),
            ("considered-options", "Alternatives evaluated"),
            ("outcome", "Result of the decision"),
        ];

        attributes
            .iter()
            .filter(|(name, _)| prefix.is_empty() || name.starts_with(prefix))
            .map(|(name, desc)| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(desc.to_string()),
                insert_text: Some(format!("{}: ", name)),
                ..Default::default()
            })
            .collect()
    }
}

impl Default for BridgePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentDomain for BridgePlugin {
    fn name(&self) -> &str {
        "bridge"
    }

    fn score_document(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        let mut score: f32 = 0.0;

        // Check for ADR title (strongest signal)
        static ADR_TITLE_RE: OnceLock<Regex> = OnceLock::new();
        let adr_title_re =
            ADR_TITLE_RE.get_or_init(|| Regex::new(r"(?i)^=\s*ADR[\s\-]?\d+").unwrap());

        if adr_title_re.is_match(text) {
            score += 0.5;
        }

        // Check for status attribute
        if text.contains(":status:") {
            score += 0.2;

            // Bonus if it's a valid ADR status
            for status in VALID_STATUSES {
                if text
                    .to_lowercase()
                    .contains(&format!(":status: {}", status.to_lowercase()))
                {
                    score += 0.1;
                    break;
                }
            }
        }

        // Check for required sections
        for section in REQUIRED_SECTIONS {
            if text.contains(&format!("== {}", section))
                || text.contains(&format!("=== {}", section))
            {
                score += 0.1;
            }
        }

        score.min(1.0)
    }

    fn validate(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.is_adr_document(text) {
            diagnostics.extend(self.validate_sections(text));
        }

        // Always validate status if present
        diagnostics.extend(self.validate_status(text));

        diagnostics
    }

    fn complete(&self, _position: Position, line_prefix: &str) -> Vec<CompletionItem> {
        let trimmed = line_prefix.trim();

        // Status value completion
        if trimmed.starts_with(":status:") {
            return self.complete_status_values();
        }

        // Attribute name completion
        if trimmed.starts_with(':') && !trimmed.contains(": ") {
            let prefix = trimmed.trim_start_matches(':');
            return self.complete_attribute_names(prefix);
        }

        Vec::new()
    }

    fn classify_element(&self, element_type: &str, value: &str) -> Option<SemanticTokenType> {
        match element_type {
            "header" => Some(SemanticTokenType::CLASS),

            "attribute_name" => match value {
                "status" => Some(SemanticTokenType::ENUM),
                "author" | "date" | "version" => Some(SemanticTokenType::PROPERTY),
                "decision-drivers" | "considered-options" | "outcome" => {
                    Some(SemanticTokenType::PROPERTY)
                }
                _ => Some(SemanticTokenType::PROPERTY),
            },

            "attribute_value" => {
                // Check if it's a status value
                if VALID_STATUSES
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(value))
                {
                    Some(SemanticTokenType::ENUM_MEMBER)
                } else {
                    Some(SemanticTokenType::STRING)
                }
            }

            "xref" | "anchor" => Some(SemanticTokenType::VARIABLE),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adr_detection() {
        let plugin = BridgePlugin::new();

        assert!(plugin.is_adr_document("= ADR 001: Test"));
        assert!(plugin.is_adr_document("= ADR-002: Another"));
        assert!(plugin.is_adr_document(":status: Draft\n\n== Context\n"));
        assert!(!plugin.is_adr_document("= Regular Document"));
    }

    #[test]
    fn test_scoring() {
        let plugin = BridgePlugin::new();

        let adr_doc = "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.";
        let plain_doc = "= Regular Document\n\nContent.";

        assert!(plugin.score_document(adr_doc) > 0.7);
        assert!(plugin.score_document(plain_doc) < 0.3);
    }
}
