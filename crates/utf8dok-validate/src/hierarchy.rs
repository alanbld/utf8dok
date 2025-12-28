//! Section hierarchy validator
//!
//! This module validates that document headings follow a proper hierarchy,
//! without skipping levels (e.g., going from Level 2 directly to Level 4).

use utf8dok_ast::{Block, Document};
use utf8dok_core::diagnostics::Diagnostic;

use crate::Validator;

/// Validates section heading hierarchy
///
/// This validator checks that:
/// - Heading levels don't skip (e.g., Level 2 -> Level 4 is invalid)
/// - The first heading doesn't start too deep (should typically be Level 1)
///
/// # Diagnostic Codes
///
/// - `DOC101`: Section level jump detected
///
/// # Example
///
/// ```
/// use utf8dok_validate::{Validator, SectionHierarchyValidator};
/// use utf8dok_ast::{Document, Block, Heading, Inline};
///
/// let validator = SectionHierarchyValidator;
///
/// // Valid document
/// let doc = Document {
///     metadata: utf8dok_ast::DocumentMeta::default(),
///     blocks: vec![
///         Block::Heading(Heading { level: 1, text: vec![], style_id: None, anchor: None }),
///         Block::Heading(Heading { level: 2, text: vec![], style_id: None, anchor: None }),
///     ],
///     intent: None,
/// };
///
/// assert!(validator.validate(&doc).is_empty());
/// ```
pub struct SectionHierarchyValidator;

impl Validator for SectionHierarchyValidator {
    fn code(&self) -> &'static str {
        "DOC1"
    }

    fn name(&self) -> &'static str {
        "section-hierarchy"
    }

    fn validate(&self, doc: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut current_level: u8 = 0; // Start at 0 (before any heading)

        for (block_index, block) in doc.blocks.iter().enumerate() {
            if let Block::Heading(heading) = block {
                let new_level = heading.level;

                // Check for level jump (increase by more than 1)
                if new_level > current_level + 1 {
                    let message = if current_level == 0 {
                        format!(
                            "Document starts at heading level {} (expected level 1). \
                             Consider starting with a level 1 heading.",
                            new_level
                        )
                    } else {
                        format!(
                            "Section level jump detected (Level {} -> Level {}). \
                             Missing Level {}?",
                            current_level,
                            new_level,
                            current_level + 1
                        )
                    };

                    let diagnostic = Diagnostic::warning(message)
                        .with_code("DOC101")
                        .with_help(
                            "Heading hierarchy should not skip levels. \
                             Add intermediate heading(s) or adjust the level."
                                .to_string(),
                        )
                        .with_note(format!("Found at block index {}", block_index));

                    diagnostics.push(diagnostic);
                }

                // Update current level (allow going back up to any level)
                current_level = new_level;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use utf8dok_ast::{Heading, Inline, Paragraph};

    fn heading(level: u8, text: &str) -> Block {
        Block::Heading(Heading {
            level,
            text: vec![Inline::Text(text.to_string())],
            style_id: None,
            anchor: None,
        })
    }

    fn paragraph(text: &str) -> Block {
        Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text(text.to_string())],
            style_id: None,
            attributes: HashMap::new(),
        })
    }

    #[test]
    fn test_validator_code() {
        let validator = SectionHierarchyValidator;
        assert_eq!(validator.code(), "DOC1");
    }

    #[test]
    fn test_validator_name() {
        let validator = SectionHierarchyValidator;
        assert_eq!(validator.name(), "section-hierarchy");
    }

    #[test]
    fn test_empty_document() {
        let validator = SectionHierarchyValidator;
        let doc = Document::new();
        assert!(validator.validate(&doc).is_empty());
    }

    #[test]
    fn test_single_level_1_heading() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![heading(1, "Title")],
            intent: None,
        };
        assert!(validator.validate(&doc).is_empty());
    }

    #[test]
    fn test_proper_sequential_hierarchy() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Chapter 1"),
                heading(2, "Section 1.1"),
                heading(3, "Subsection 1.1.1"),
                heading(2, "Section 1.2"),
                heading(1, "Chapter 2"),
            ],
            intent: None,
        };
        assert!(validator.validate(&doc).is_empty());
    }

    #[test]
    fn test_jump_from_1_to_3() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Chapter"),
                heading(3, "Subsection"), // Jump!
            ],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, Some("DOC101".to_string()));
        assert!(diagnostics[0].message.contains("Level 1"));
        assert!(diagnostics[0].message.contains("Level 3"));
        assert!(diagnostics[0].message.contains("Level 2"));
    }

    #[test]
    fn test_jump_from_2_to_4() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Chapter"),
                heading(2, "Section"),
                heading(4, "Deep"), // Jump!
            ],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Level 2"));
        assert!(diagnostics[0].message.contains("Level 4"));
    }

    #[test]
    fn test_starting_at_level_2() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![heading(2, "Starting at Level 2")],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("starts at heading level 2"));
    }

    #[test]
    fn test_starting_at_level_3() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![heading(3, "Starting at Level 3")],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("starts at heading level 3"));
    }

    #[test]
    fn test_mixed_content_valid() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Intro"),
                paragraph("Some text"),
                heading(2, "Details"),
                paragraph("More text"),
                heading(3, "Specifics"),
            ],
            intent: None,
        };
        assert!(validator.validate(&doc).is_empty());
    }

    #[test]
    fn test_mixed_content_with_jump() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Intro"),
                paragraph("Some text"),
                heading(4, "Deep"), // Jump from 1 to 4!
            ],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_going_back_up_is_valid() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Chapter 1"),
                heading(2, "Section 1.1"),
                heading(3, "Subsection 1.1.1"),
                heading(1, "Chapter 2"), // Going back up is fine
            ],
            intent: None,
        };
        assert!(validator.validate(&doc).is_empty());
    }

    #[test]
    fn test_multiple_jumps() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                heading(1, "Chapter"),
                heading(3, "Jump 1"), // First jump
                heading(5, "Jump 2"), // Second jump
            ],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 2, "Should detect both jumps");
    }

    #[test]
    fn test_diagnostic_has_help() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![heading(1, "Chapter"), heading(3, "Jump")],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_severity_is_warning() {
        let validator = SectionHierarchyValidator;
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![heading(3, "Deep Start")],
            intent: None,
        };

        let diagnostics = validator.validate(&doc);
        assert!(diagnostics[0].is_warning());
        assert!(!diagnostics[0].is_error());
    }
}
