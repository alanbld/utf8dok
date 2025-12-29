//! Code Actions for Compliance Violations (Phase 15)
//!
//! Provides quick fixes for compliance rule violations.

use std::collections::HashMap;

use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Range, TextEdit, Url, WorkspaceEdit};

/// A quick fix for a compliance violation
#[derive(Debug, Clone)]
pub struct ComplianceFix {
    /// Title displayed to the user
    pub title: String,
    /// The URI to edit
    pub uri: Url,
    /// The text edits to apply
    pub edits: Vec<TextEdit>,
    /// Kind of code action (quickfix, refactor, etc.)
    pub kind: CodeActionKind,
}

impl ComplianceFix {
    /// Convert to LSP CodeAction
    pub fn to_code_action(self) -> CodeAction {
        let mut changes = HashMap::new();
        changes.insert(self.uri, self.edits);

        CodeAction {
            title: self.title,
            kind: Some(self.kind),
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
        }
    }
}

/// Find the range of a status attribute in document text
pub fn find_status_range(text: &str) -> Option<(Range, String)> {
    for (line_num, line) in text.lines().enumerate() {
        // Look for :status: VALUE pattern
        if let Some(rest) = line.strip_prefix(":status:") {
            let value = rest.trim();
            // Calculate the range of just the value
            let value_start = line.find(value).unwrap_or(0) as u32;
            let value_end = value_start + value.len() as u32;

            return Some((
                Range {
                    start: tower_lsp::lsp_types::Position {
                        line: line_num as u32,
                        character: value_start,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: line_num as u32,
                        character: value_end,
                    },
                },
                value.to_string(),
            ));
        }
    }
    None
}

/// Find where to insert a link in an index document
pub fn find_index_insert_position(text: &str) -> Range {
    // Find the last non-empty line to append after
    let lines: Vec<&str> = text.lines().collect();
    let last_line = lines.len().saturating_sub(1) as u32;
    let last_col = lines.last().map(|l| l.len()).unwrap_or(0) as u32;

    Range {
        start: tower_lsp::lsp_types::Position {
            line: last_line,
            character: last_col,
        },
        end: tower_lsp::lsp_types::Position {
            line: last_line,
            character: last_col,
        },
    }
}

/// Generate a link to add an orphan document to an index
pub fn generate_orphan_link(orphan_uri: &str) -> String {
    // Extract filename from URI
    let filename = orphan_uri
        .rsplit('/')
        .next()
        .unwrap_or("document.adoc");

    // Generate a link entry
    format!("\n* link:{}[]", filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== STATUS FIX TESTS ====================

    #[test]
    fn test_find_status_range() {
        let text = "= Title\n:status: Accepted\n\nContent here.";

        let result = find_status_range(text);
        assert!(result.is_some());

        let (range, value) = result.unwrap();
        assert_eq!(value, "Accepted");
        assert_eq!(range.start.line, 1);
        assert_eq!(range.end.line, 1);
    }

    #[test]
    fn test_find_status_range_with_spaces() {
        let text = "= Title\n:status:   Proposed  \n\nContent.";

        let result = find_status_range(text);
        assert!(result.is_some());

        let (_, value) = result.unwrap();
        assert_eq!(value, "Proposed");
    }

    #[test]
    fn test_find_status_range_not_found() {
        let text = "= Title\n\nNo status attribute here.";

        let result = find_status_range(text);
        assert!(result.is_none());
    }

    // ==================== ORPHAN FIX TESTS ====================

    #[test]
    fn test_find_index_insert_position() {
        let text = "= Index\n\n* link:adr-001.adoc[]\n* link:adr-002.adoc[]";

        let pos = find_index_insert_position(text);
        assert_eq!(pos.start.line, 3); // Last line
    }

    #[test]
    fn test_generate_orphan_link() {
        let link = generate_orphan_link("file:///project/docs/adr-003.adoc");
        assert_eq!(link, "\n* link:adr-003.adoc[]");
    }

    #[test]
    fn test_generate_orphan_link_nested_path() {
        let link = generate_orphan_link("file:///project/deep/nested/path/document.adoc");
        assert_eq!(link, "\n* link:document.adoc[]");
    }

    // ==================== COMPLIANCE FIX TESTS ====================

    #[test]
    fn test_compliance_fix_to_code_action() {
        let fix = ComplianceFix {
            title: "Mark as Deprecated".to_string(),
            uri: Url::parse("file:///test.adoc").unwrap(),
            edits: vec![TextEdit {
                range: Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 9,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 17,
                    },
                },
                new_text: "Deprecated".to_string(),
            }],
            kind: CodeActionKind::QUICKFIX,
        };

        let action = fix.to_code_action();

        assert_eq!(action.title, "Mark as Deprecated");
        assert!(action.is_preferred.unwrap());
        assert!(action.edit.is_some());

        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        assert!(changes.contains_key(&Url::parse("file:///test.adoc").unwrap()));
    }

    // ==================== INTEGRATION TESTS ====================

    #[test]
    fn test_fix_status_creates_valid_edit() {
        let text = "= ADR 001\n:status: Accepted\n\nThis ADR was superseded.";

        // Find the status
        let (range, current_value) = find_status_range(text).unwrap();
        assert_eq!(current_value, "Accepted");

        // Create a fix
        let fix = ComplianceFix {
            title: "Mark as Deprecated".to_string(),
            uri: Url::parse("file:///adr-001.adoc").unwrap(),
            edits: vec![TextEdit {
                range,
                new_text: "Deprecated".to_string(),
            }],
            kind: CodeActionKind::QUICKFIX,
        };

        let action = fix.to_code_action();
        let changes = action.edit.unwrap().changes.unwrap();
        let edits = changes
            .get(&Url::parse("file:///adr-001.adoc").unwrap())
            .unwrap();

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "Deprecated");
    }

    #[test]
    fn test_fix_orphan_creates_valid_edit() {
        let index_text = "= ADR Index\n\n* link:adr-001.adoc[]";
        let orphan_uri = "file:///project/adr-002.adoc";

        // Find insert position
        let insert_pos = find_index_insert_position(index_text);

        // Generate link
        let link = generate_orphan_link(orphan_uri);
        assert_eq!(link, "\n* link:adr-002.adoc[]");

        // Create a fix
        let fix = ComplianceFix {
            title: "Add link to index".to_string(),
            uri: Url::parse("file:///project/index.adoc").unwrap(),
            edits: vec![TextEdit {
                range: insert_pos,
                new_text: link,
            }],
            kind: CodeActionKind::QUICKFIX,
        };

        let action = fix.to_code_action();
        let changes = action.edit.unwrap().changes.unwrap();
        let edits = changes
            .get(&Url::parse("file:///project/index.adoc").unwrap())
            .unwrap();

        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("adr-002.adoc"));
    }
}
