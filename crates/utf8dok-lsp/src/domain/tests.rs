//! TDD Test Suite for Domain Intelligence Engine
//!
//! Tests for completion, validation, and code actions.

use tower_lsp::lsp_types::{CompletionItemKind, DiagnosticSeverity, Position, Range};

fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Range {
    Range {
        start: pos(start_line, start_char),
        end: pos(end_line, end_char),
    }
}

// ==================== COMPLETION ENGINE TESTS ====================

mod completion_tests {
    use super::*;
    use crate::domain::completion::CompletionEngine;

    /// TEST 1: Xref completion finds section IDs
    #[test]
    fn test_xref_completion_finds_section_ids() {
        let text = r#"
= Document Title

[[architecture-overview]]
== Architecture Overview
This section describes the system architecture.

[[api-design]]
=== API Design
Details about the API.

[[database-schema]]
== Database Schema
Information about the database.

See <<"#;

        let completions = CompletionEngine::complete(text, pos(15, 6));

        // Should find all section IDs (including auto-generated from Document Title)
        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
        assert!(labels.contains(&"architecture-overview".to_string()), "Should find architecture-overview");
        assert!(labels.contains(&"api-design".to_string()), "Should find api-design");
        assert!(labels.contains(&"database-schema".to_string()), "Should find database-schema");
        assert!(labels.contains(&"document-title".to_string()), "Should find document-title (auto-generated)");
        assert_eq!(completions.len(), 4, "Should have exactly 4 completions (3 anchored + 1 title)");

        // Items should have correct metadata
        for item in &completions {
            assert_eq!(item.kind, Some(CompletionItemKind::REFERENCE));
            assert!(item.detail.is_some(), "Should have detail");
            assert!(item.documentation.is_some(), "Should have documentation");
        }
    }

    /// TEST 2: Xref completion only triggers after <<
    #[test]
    fn test_xref_completion_only_triggers_after_double_angle() {
        let text = "Some text with < but not double angle";

        let completions1 = CompletionEngine::complete(text, pos(0, 15));
        let completions2 = CompletionEngine::complete(text, pos(0, 25));

        assert!(completions1.is_empty(), "Single < should not trigger");
        assert!(completions2.is_empty(), "Middle of text should not trigger");
    }

    /// TEST 3: Xref completion performance
    #[test]
    fn test_xref_completion_performance() {
        // Build large document with 200 sections
        let mut text = String::new();
        for i in 0..200 {
            text.push_str(&format!("[[section-{:03}]]\n", i));
            text.push_str(&format!("== Section {}\n\n", i));
        }
        text.push_str("Reference: <<");

        let start = std::time::Instant::now();
        let completions = CompletionEngine::complete(&text, pos(600, 13));
        let duration = start.elapsed();

        assert_eq!(completions.len(), 200, "Should find all 200 sections");
        assert!(
            duration < std::time::Duration::from_millis(100),
            "Completion took {:?} (should be <100ms)",
            duration
        );
    }

    /// TEST 4: Attribute name completion at line start
    #[test]
    fn test_attribute_name_completion() {
        let text = ":";

        let completions = CompletionEngine::complete(text, pos(0, 1));

        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

        // Should suggest common attributes
        assert!(labels.contains(&"status".to_string()), "Should suggest status");
        assert!(labels.contains(&"author".to_string()), "Should suggest author");
        assert!(labels.contains(&"date".to_string()), "Should suggest date");
        assert!(labels.contains(&"version".to_string()), "Should suggest version");
        assert!(labels.contains(&"toc".to_string()), "Should suggest toc");

        // Should include ADR/Bridge attributes
        assert!(labels.contains(&"context".to_string()), "Should suggest context");
        assert!(labels.contains(&"decision".to_string()), "Should suggest decision");
        assert!(labels.contains(&"consequences".to_string()), "Should suggest consequences");

        // Items should have documentation
        for item in &completions {
            assert!(
                item.documentation.is_some(),
                "Attribute '{}' should have documentation",
                item.label
            );
        }
    }

    /// TEST 5: Attribute name completion with partial name
    #[test]
    fn test_attribute_name_completion_with_prefix() {
        let text = ":sta";

        let completions = CompletionEngine::complete(text, pos(0, 4));

        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

        assert!(labels.contains(&"status".to_string()), "Should match 'status'");
        assert!(labels.contains(&"stage".to_string()), "Should match 'stage'");
        assert!(!labels.contains(&"author".to_string()), "Should not match 'author'");
    }

    /// TEST 6: Attribute value completion for status
    #[test]
    fn test_attribute_value_completion_for_status() {
        let text = ":status: ";

        let completions = CompletionEngine::complete(text, pos(0, 9));

        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

        assert!(labels.contains(&"Draft".to_string()), "Should suggest Draft");
        assert!(labels.contains(&"Accepted".to_string()), "Should suggest Accepted");
        assert!(labels.contains(&"Rejected".to_string()), "Should suggest Rejected");
        assert!(labels.contains(&"Deprecated".to_string()), "Should suggest Deprecated");
        assert!(labels.contains(&"Superseded".to_string()), "Should suggest Superseded");

        for item in &completions {
            assert!(item.detail.is_some(), "Status '{}' should have detail", item.label);
        }
    }

    /// TEST 7: Attribute value completion for toc
    #[test]
    fn test_attribute_value_completion_for_toc() {
        let text = ":toc: ";

        let completions = CompletionEngine::complete(text, pos(0, 6));

        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

        assert!(labels.contains(&"left".to_string()));
        assert!(labels.contains(&"right".to_string()));
        assert!(labels.contains(&"preamble".to_string()));
    }

    /// TEST 8: Block type completion
    #[test]
    fn test_block_type_completion() {
        let text = "[";

        let completions = CompletionEngine::complete(text, pos(0, 1));

        let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

        assert!(labels.contains(&"source".to_string()));
        assert!(labels.contains(&"listing".to_string()));
        assert!(labels.contains(&"example".to_string()));
    }

    /// TEST 9: No completion in middle of word
    #[test]
    fn test_no_completion_in_middle_of_word() {
        let text = "This is a normal paragraph with no triggers.";

        let completions1 = CompletionEngine::complete(text, pos(0, 10));
        let completions2 = CompletionEngine::complete(text, pos(0, 20));

        assert!(completions1.is_empty());
        assert!(completions2.is_empty());
    }
}

// ==================== DOMAIN VALIDATION TESTS ====================

mod validation_tests {
    use super::*;
    use crate::domain::validation::DomainValidator;

    /// TEST 1: Valid ADR has no diagnostics
    #[test]
    fn test_adr_template_detection_valid() {
        let text = r#"= ADR 001: Use AsciiDoc for documentation
:status: Draft
:date: 2024-01-01
:author: Team Docs

== Context
We need to choose a documentation format.

== Decision
We will use AsciiDoc.

== Consequences
Positive: Portable, extensible.
Negative: Learning curve."#;

        let validator = DomainValidator::new();
        let diagnostics = validator.validate_document(text);

        assert!(
            diagnostics.is_empty(),
            "Valid ADR should have no diagnostics, got: {:?}",
            diagnostics
        );
    }

    /// TEST 2: ADR missing required section
    #[test]
    fn test_adr_missing_required_sections() {
        let text = r#"= ADR 001: Use AsciiDoc
:status: Draft

== Context
We need to choose.

== Consequences
Will be good."#;

        let validator = DomainValidator::new();
        let diagnostics = validator.validate_document(text);

        assert!(!diagnostics.is_empty(), "Should have diagnostics");

        let missing_decision: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Decision"))
            .collect();

        assert!(
            !missing_decision.is_empty(),
            "Should have diagnostic for missing Decision section"
        );

        assert_eq!(
            missing_decision[0].severity,
            Some(DiagnosticSeverity::WARNING)
        );
    }

    /// TEST 3: Invalid status value
    #[test]
    fn test_adr_invalid_status_value() {
        let text = ":status: InvalidStatus\n\n== Context\nTest.\n\n== Decision\nTest.\n\n== Consequences\nTest.";

        let validator = DomainValidator::new();
        let diagnostics = validator.validate_document(text);

        assert!(!diagnostics.is_empty(), "Should have diagnostics");

        let status_diag: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("status") && d.message.contains("InvalidStatus"))
            .collect();

        assert!(!status_diag.is_empty(), "Should flag invalid status");
    }

    /// TEST 4: Non-ADR document has no validation
    #[test]
    fn test_non_adr_no_validation() {
        let text = r#"= Regular Document
:not-status: something

== Some Section
Content."#;

        let validator = DomainValidator::new();
        let diagnostics = validator.validate_document(text);

        assert!(diagnostics.is_empty(), "Non-ADR should have no diagnostics");
    }
}

// ==================== CODE ACTIONS TESTS ====================

mod code_action_tests {
    use super::*;
    use crate::domain::validation::DomainValidator;
    use tower_lsp::lsp_types::{CodeActionContext, CodeActionParams, TextDocumentIdentifier, Url};

    fn make_params(uri: &str, line: u32) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier::new(Url::parse(uri).unwrap()),
            range: range(line, 0, line, 0),
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }
    }

    /// TEST 1: Code action to insert missing ADR sections
    #[test]
    fn test_code_action_insert_missing_sections() {
        let text = r#"= ADR 001: Test
:status: Draft

== Context
Test."#;

        let validator = DomainValidator::new();
        let params = make_params("file:///test.adoc", 5);

        let actions = validator.get_code_actions(text, &params);

        assert!(!actions.is_empty(), "Should have code actions");

        let titles: Vec<String> = actions.iter().map(|a| a.title.clone()).collect();

        assert!(
            titles.iter().any(|t| t.contains("Decision")),
            "Should offer to insert Decision"
        );
        assert!(
            titles.iter().any(|t| t.contains("Consequences")),
            "Should offer to insert Consequences"
        );

        // Actions should have edits
        for action in &actions {
            assert!(action.edit.is_some(), "Code action should have edit");
        }
    }

    /// TEST 2: Code action to fix invalid status
    #[test]
    fn test_code_action_fix_invalid_status() {
        let text = ":status: Invalid\n\n== Context\nTest.\n\n== Decision\nTest.\n\n== Consequences\nTest.";

        let validator = DomainValidator::new();
        let params = make_params("file:///test.adoc", 0);

        let actions = validator.get_code_actions(text, &params);

        let titles: Vec<String> = actions.iter().map(|a| a.title.clone()).collect();

        assert!(
            titles.iter().any(|t| t.contains("Draft")),
            "Should offer to change to Draft"
        );
        assert!(
            titles.iter().any(|t| t.contains("Accepted")),
            "Should offer to change to Accepted"
        );
    }
}

// ==================== INTEGRATION TESTS ====================

mod integration_tests {
    use super::*;
    use crate::domain::DomainEngine;

    /// TEST 1: Domain engine coordinates all features
    #[test]
    fn test_domain_engine_integration() {
        let text = r#"
[[intro]]
== Introduction

[[details]]
=== Details

See <<"#;

        let engine = DomainEngine::new();

        // Completions should work
        let completions = engine.get_completions(text, pos(7, 6));
        assert_eq!(completions.len(), 2, "Should find 2 xrefs");

        // Validation should work (this is not an ADR)
        let diagnostics = engine.validate_document(text);
        assert!(diagnostics.is_empty(), "Non-ADR should have no diagnostics");
    }

    /// TEST 2: Performance test
    #[test]
    fn test_domain_engine_performance() {
        let mut text = String::new();
        for i in 0..100 {
            text.push_str(&format!("[[section-{:03}]]\n", i));
            text.push_str(&format!("== Section {}\n\n", i));
        }
        text.push_str("Reference: <<");

        let engine = DomainEngine::new();

        let start = std::time::Instant::now();
        let _completions = engine.get_completions(&text, pos(300, 13));
        let _diagnostics = engine.validate_document(&text);
        let duration = start.elapsed();

        assert!(
            duration < std::time::Duration::from_millis(200),
            "Full analysis should be <200ms, took {:?}",
            duration
        );
    }
}
