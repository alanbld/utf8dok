//! ADR (Architecture Decision Record) validation
//!
//! Validates ADR documents for required sections and structure.

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Diagnostic, DiagnosticSeverity,
    Position, Range, TextEdit, WorkspaceEdit,
};

/// Required ADR sections
const REQUIRED_SECTIONS: &[(&str, &str)] = &[
    ("Context", "Describes the forces at play, including technological, political, social, and project-specific factors."),
    ("Decision", "Describes our response to these forces, the decision we've made."),
    ("Consequences", "Describes the resulting context, after applying the decision."),
];

/// Optional but recommended ADR sections
#[allow(dead_code)]
const OPTIONAL_SECTIONS: &[(&str, &str)] = &[
    ("Status", "Current status of the decision (can also be an attribute)"),
    ("Alternatives", "Other options considered"),
    ("Related", "Related decisions or documents"),
];

/// ADR-specific validator
pub struct AdrValidator;

impl AdrValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate an ADR document
    #[allow(dead_code)]
    pub fn validate(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let sections = self.find_sections(text);

        // Check for missing required sections
        for (section_name, description) in REQUIRED_SECTIONS {
            if !sections.iter().any(|(name, _)| name.eq_ignore_ascii_case(section_name)) {
                // Find a good position for the diagnostic (end of document or after last section)
                let position = self.find_insertion_position(text);

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: position,
                        end: position,
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(tower_lsp::lsp_types::NumberOrString::String("ADR001".to_string())),
                    source: Some("utf8dok-domain".to_string()),
                    message: format!(
                        "Missing required ADR section: '{}'. {}",
                        section_name, description
                    ),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    /// Get code actions for ADR issues
    pub fn get_code_actions(&self, text: &str, params: &CodeActionParams) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let sections = self.find_sections(text);

        // Find missing sections and offer to insert them
        for (section_name, _description) in REQUIRED_SECTIONS {
            if !sections.iter().any(|(name, _)| name.eq_ignore_ascii_case(section_name)) {
                let insert_pos = self.find_insertion_position(text);

                // Only offer if cursor is near end of document
                if params.range.start.line >= insert_pos.line.saturating_sub(5) {
                    let section_template = self.generate_section_template(section_name);

                    let mut changes = HashMap::new();
                    changes.insert(
                        params.text_document.uri.clone(),
                        vec![TextEdit {
                            range: Range {
                                start: insert_pos,
                                end: insert_pos,
                            },
                            new_text: section_template,
                        }],
                    );

                    actions.push(CodeAction {
                        title: format!("Insert '{}' section", section_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: None,
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }),
                        command: None,
                        is_preferred: Some(false),
                        disabled: None,
                        data: None,
                    });
                }
            }
        }

        // Offer to insert all missing sections at once
        let missing: Vec<_> = REQUIRED_SECTIONS
            .iter()
            .filter(|(name, _)| !sections.iter().any(|(s, _)| s.eq_ignore_ascii_case(name)))
            .collect();

        if missing.len() > 1 {
            let insert_pos = self.find_insertion_position(text);

            if params.range.start.line >= insert_pos.line.saturating_sub(5) {
                let all_sections: String = missing
                    .iter()
                    .map(|(name, _)| self.generate_section_template(name))
                    .collect();

                let mut changes = HashMap::new();
                changes.insert(
                    params.text_document.uri.clone(),
                    vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text: all_sections,
                    }],
                );

                actions.push(CodeAction {
                    title: "Insert all missing ADR sections".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                });
            }
        }

        actions
    }

    /// Find all sections in the document
    fn find_sections(&self, text: &str) -> Vec<(String, usize)> {
        static SECTION_RE: OnceLock<Regex> = OnceLock::new();
        let section_re = SECTION_RE.get_or_init(|| {
            Regex::new(r"^(=+)\s+(.+)$").unwrap()
        });

        let mut sections = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            if let Some(cap) = section_re.captures(line.trim()) {
                let title = cap.get(2).unwrap().as_str().to_string();
                sections.push((title, line_num));
            }
        }

        sections
    }

    /// Find the best position to insert new content
    fn find_insertion_position(&self, text: &str) -> Position {
        let lines: Vec<&str> = text.lines().collect();
        let last_line = lines.len().saturating_sub(1);

        // Find last non-empty line
        for (i, line) in lines.iter().enumerate().rev() {
            if !line.trim().is_empty() {
                return Position {
                    line: (i + 1) as u32,
                    character: 0,
                };
            }
        }

        Position {
            line: last_line as u32,
            character: 0,
        }
    }

    /// Generate template for a section
    fn generate_section_template(&self, section_name: &str) -> String {
        let description = REQUIRED_SECTIONS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(section_name))
            .map(|(_, desc)| *desc)
            .unwrap_or("Description here.");

        format!(
            "\n== {}\n\n// TODO: {}\n",
            section_name,
            description
        )
    }
}

impl Default for AdrValidator {
    fn default() -> Self {
        Self::new()
    }
}
