//! RFC Domain Plugin
//!
//! Provides domain intelligence for RFC-style documents.

use crate::domain::traits::DocumentDomain;
use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, Documentation,
    MarkupContent, MarkupKind, Position, Range, SemanticTokenType,
};

/// Valid RFC categories
const RFC_CATEGORIES: &[&str] = &[
    "standards-track",
    "informational",
    "experimental",
    "best-current-practice",
    "historic",
];

/// Required RFC sections
const REQUIRED_SECTIONS: &[&str] = &["Abstract"];

/// Optional but common RFC sections
#[allow(dead_code)]
const COMMON_SECTIONS: &[&str] = &[
    "Introduction",
    "Requirements",
    "Security Considerations",
    "IANA Considerations",
];

/// RFC domain plugin
pub struct RfcPlugin;

impl RfcPlugin {
    pub fn new() -> Self {
        Self
    }

    /// Check if document looks like an RFC
    fn is_rfc_document(&self, text: &str) -> bool {
        static RFC_TITLE_RE: OnceLock<Regex> = OnceLock::new();
        let rfc_title_re = RFC_TITLE_RE.get_or_init(|| Regex::new(r"(?i)^=\s*RFC\s*\d+").unwrap());

        // Check for RFC title pattern
        if rfc_title_re.is_match(text) {
            return true;
        }

        // Check for category attribute
        if text.contains(":category:") {
            for cat in RFC_CATEGORIES {
                if text.to_lowercase().contains(cat) {
                    return true;
                }
            }
        }

        // Check for RFC-specific sections
        text.contains("== Abstract")
            && (text.contains("== Security Considerations")
                || text.contains("== IANA Considerations"))
    }

    /// Validate RFC sections
    fn validate_sections(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for section in REQUIRED_SECTIONS {
            let section_pattern = format!("== {}", section);

            if !text.contains(&section_pattern) {
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
                        "RFC001".to_string(),
                    )),
                    source: Some("utf8dok-rfc".to_string()),
                    message: format!("Missing required RFC section: '{}'", section),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    /// Validate category attribute
    fn validate_category(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        static CATEGORY_RE: OnceLock<Regex> = OnceLock::new();
        let category_re = CATEGORY_RE.get_or_init(|| Regex::new(r"^:category:\s*(\S+)").unwrap());

        for (line_num, line) in text.lines().enumerate() {
            if let Some(cap) = category_re.captures(line.trim()) {
                let category = cap.get(1).unwrap().as_str();
                if !RFC_CATEGORIES
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(category))
                {
                    let start_char = line.find(category).unwrap_or(0);
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: start_char as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (start_char + category.len()) as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(tower_lsp::lsp_types::NumberOrString::String(
                            "RFC002".to_string(),
                        )),
                        source: Some("utf8dok-rfc".to_string()),
                        message: format!(
                            "Invalid category '{}'. Expected one of: {}",
                            category,
                            RFC_CATEGORIES.join(", ")
                        ),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    /// Get category completions
    fn complete_categories(&self) -> Vec<CompletionItem> {
        RFC_CATEGORIES
            .iter()
            .map(|category| CompletionItem {
                label: category.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("RFC category: {}", category)),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: match *category {
                        "standards-track" => "Standard protocol specification",
                        "informational" => "General information document",
                        "experimental" => "Experimental protocol/procedure",
                        "best-current-practice" => "Recommended practices",
                        "historic" => "Historical or obsolete specification",
                        _ => "RFC category",
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
            (
                "category",
                "RFC category (standards-track, informational, etc.)",
            ),
            ("author", "Document author(s)"),
            ("date", "Publication date"),
            ("area", "IETF area"),
            ("workgroup", "Working group"),
            ("obsoletes", "RFCs obsoleted by this document"),
            ("updates", "RFCs updated by this document"),
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

impl Default for RfcPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentDomain for RfcPlugin {
    fn name(&self) -> &str {
        "rfc"
    }

    fn score_document(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        let mut score: f32 = 0.0;

        // Check for RFC title (strongest signal)
        static RFC_TITLE_RE: OnceLock<Regex> = OnceLock::new();
        let rfc_title_re = RFC_TITLE_RE.get_or_init(|| Regex::new(r"(?i)^=\s*RFC\s*\d+").unwrap());

        if rfc_title_re.is_match(text) {
            score += 0.5;
        }

        // Check for category attribute
        if text.contains(":category:") {
            score += 0.2;

            // Bonus for valid RFC category
            for cat in RFC_CATEGORIES {
                if text.to_lowercase().contains(cat) {
                    score += 0.2;
                    break;
                }
            }
        }

        // Check for RFC-specific sections
        if text.contains("== Abstract") {
            score += 0.1;
        }
        if text.contains("== Security Considerations") {
            score += 0.1;
        }
        if text.contains("== IANA Considerations") {
            score += 0.1;
        }

        score.min(1.0)
    }

    fn validate(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.is_rfc_document(text) {
            diagnostics.extend(self.validate_sections(text));
        }

        // Always validate category if present
        diagnostics.extend(self.validate_category(text));

        diagnostics
    }

    fn complete(&self, _position: Position, line_prefix: &str) -> Vec<CompletionItem> {
        let trimmed = line_prefix.trim();

        // Category value completion
        if trimmed.starts_with(":category:") {
            return self.complete_categories();
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
                // RFC-specific: category is a KEYWORD because it's a controlled vocabulary
                "category" => Some(SemanticTokenType::KEYWORD),
                "author" | "date" | "area" | "workgroup" => Some(SemanticTokenType::PROPERTY),
                "obsoletes" | "updates" => Some(SemanticTokenType::VARIABLE),
                _ => Some(SemanticTokenType::PROPERTY),
            },

            "attribute_value" => {
                // Check if it's a category value
                if RFC_CATEGORIES.iter().any(|c| c.eq_ignore_ascii_case(value)) {
                    Some(SemanticTokenType::KEYWORD)
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
    fn test_rfc_detection() {
        let plugin = RfcPlugin::new();

        assert!(plugin.is_rfc_document("= RFC 1234: Test"));
        assert!(plugin.is_rfc_document("= RFC1234: Another"));
        assert!(plugin.is_rfc_document(
            ":category: standards-track\n\n== Abstract\n\n== Security Considerations"
        ));
        assert!(!plugin.is_rfc_document("= Regular Document"));
    }

    #[test]
    fn test_scoring() {
        let plugin = RfcPlugin::new();

        let rfc_doc = "= RFC 1234: Test\n:category: standards-track\n\n== Abstract\nTest.";
        let plain_doc = "= Regular Document\n\nContent.";

        assert!(plugin.score_document(rfc_doc) > 0.7);
        assert!(plugin.score_document(plain_doc) < 0.3);
    }
}
