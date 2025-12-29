//! Domain validation for AsciiDoc documents
//!
//! Provides validation rules for:
//! - ADR templates (required sections, valid status)
//! - General document structure

mod adr;

pub use adr::AdrValidator;

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Diagnostic, DiagnosticSeverity, Position, Range,
    TextEdit, WorkspaceEdit,
};

/// Main domain validator
pub struct DomainValidator {
    adr_validator: AdrValidator,
}

impl DomainValidator {
    pub fn new() -> Self {
        Self {
            adr_validator: AdrValidator::new(),
        }
    }

    /// Validate a document for domain-specific rules
    #[allow(dead_code)]
    pub fn validate_document(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Detect document type
        let doc_type = self.detect_document_type(text);

        match doc_type {
            DocumentType::Adr => {
                diagnostics.extend(self.adr_validator.validate(text));
            }
            DocumentType::Unknown => {
                // No domain-specific validation for unknown documents
            }
        }

        // Validate attribute values regardless of document type
        diagnostics.extend(self.validate_attribute_values(text));

        diagnostics
    }

    /// Get code actions for the given context
    pub fn get_code_actions(&self, text: &str, params: &CodeActionParams) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        let doc_type = self.detect_document_type(text);

        match doc_type {
            DocumentType::Adr => {
                actions.extend(self.adr_validator.get_code_actions(text, params));
            }
            DocumentType::Unknown => {}
        }

        // Add actions for invalid attribute values
        actions.extend(self.get_attribute_value_actions(text, params));

        actions
    }

    /// Detect the document type based on content
    fn detect_document_type(&self, text: &str) -> DocumentType {
        static ADR_RE: OnceLock<Regex> = OnceLock::new();
        let adr_re = ADR_RE.get_or_init(|| {
            Regex::new(r"(?i)(^=\s*ADR[\s\-]?\d+|:status:\s*(Draft|Accepted|Rejected|Deprecated|Superseded))").unwrap()
        });

        if adr_re.is_match(text) {
            return DocumentType::Adr;
        }

        // Check for ADR-like structure (Context, Decision, Consequences sections)
        let has_context = text.contains("== Context") || text.contains("=== Context");
        let has_decision = text.contains("== Decision") || text.contains("=== Decision");
        let has_consequences =
            text.contains("== Consequences") || text.contains("=== Consequences");

        if has_decision && (has_context || has_consequences) {
            return DocumentType::Adr;
        }

        DocumentType::Unknown
    }

    /// Validate attribute values
    #[allow(dead_code)]
    fn validate_attribute_values(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        static STATUS_RE: OnceLock<Regex> = OnceLock::new();
        let status_re = STATUS_RE.get_or_init(|| Regex::new(r"^:status:\s*(\S+)").unwrap());

        let valid_statuses = [
            "Draft",
            "Proposed",
            "Accepted",
            "Rejected",
            "Deprecated",
            "Superseded",
        ];

        for (line_num, line) in text.lines().enumerate() {
            if let Some(cap) = status_re.captures(line.trim()) {
                let status = cap.get(1).unwrap().as_str();
                if !valid_statuses
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
                            "DOM001".to_string(),
                        )),
                        source: Some("utf8dok-domain".to_string()),
                        message: format!(
                            "Invalid status value '{}'. Expected one of: {}",
                            status,
                            valid_statuses.join(", ")
                        ),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    /// Get code actions for invalid attribute values
    fn get_attribute_value_actions(
        &self,
        text: &str,
        params: &CodeActionParams,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let line_num = params.range.start.line as usize;
        let lines: Vec<&str> = text.lines().collect();

        if line_num >= lines.len() {
            return actions;
        }

        let line = lines[line_num];

        // Check if we're on a status line with invalid value
        static STATUS_RE: OnceLock<Regex> = OnceLock::new();
        let status_re = STATUS_RE.get_or_init(|| Regex::new(r"^:status:\s*(\S+)").unwrap());

        if let Some(cap) = status_re.captures(line.trim()) {
            let current_status = cap.get(1).unwrap();
            let valid_statuses = [
                "Draft",
                "Proposed",
                "Accepted",
                "Rejected",
                "Deprecated",
                "Superseded",
            ];

            if !valid_statuses
                .iter()
                .any(|s| s.eq_ignore_ascii_case(current_status.as_str()))
            {
                // Offer to replace with valid values
                for status in valid_statuses {
                    let line_start = line.len() - line.trim_start().len();
                    let status_start = line_start + cap.get(1).unwrap().start();
                    let status_end = line_start + cap.get(1).unwrap().end();

                    let mut changes = HashMap::new();
                    changes.insert(
                        params.text_document.uri.clone(),
                        vec![TextEdit {
                            range: Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: status_start as u32,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: status_end as u32,
                                },
                            },
                            new_text: status.to_string(),
                        }],
                    );

                    actions.push(CodeAction {
                        title: format!("Change status to '{}'", status),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: None,
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }),
                        command: None,
                        is_preferred: Some(status == "Draft"),
                        disabled: None,
                        data: None,
                    });
                }
            }
        }

        actions
    }
}

impl Default for DomainValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Document type classification
#[derive(Debug, Clone, Copy, PartialEq)]
enum DocumentType {
    Adr,
    Unknown,
}
