//! TDD tests for the AsciiDoc parser
//!
//! These tests define the expected behavior based on RENDER_SPEC.md.
//! They are written BEFORE the implementation (Red-Green-Refactor).

use std::collections::HashMap;

use utf8dok_ast::{Block, Document, DocumentMeta, FormatType, Heading, Inline, ListType, Paragraph};
use utf8dok_core::parser;

/// Test basic document parsing flow
///
/// Input:
/// ```asciidoc
/// = Test Document
/// :version: 1.0
///
/// == Section One
///
/// Hello *world*.
/// ```
#[test]
fn test_parse_basic_flow() {
    let input = r#"= Test Document
:version: 1.0

== Section One

Hello *world*."#;

    // Build expected AST
    let mut expected_attrs = HashMap::new();
    expected_attrs.insert("version".to_string(), "1.0".to_string());

    let expected = Document {
        metadata: DocumentMeta {
            title: Some("Test Document".to_string()),
            authors: vec![],
            revision: None,
            attributes: expected_attrs,
        },
        blocks: vec![
            Block::Heading(Heading {
                level: 1,
                text: vec![Inline::Text("Section One".to_string())],
                style_id: None,
                anchor: None,
            }),
            Block::Paragraph(Paragraph {
                inlines: vec![
                    Inline::Text("Hello ".to_string()),
                    Inline::Format(
                        FormatType::Bold,
                        Box::new(Inline::Text("world".to_string())),
                    ),
                    Inline::Text(".".to_string()),
                ],
                style_id: None,
                attributes: HashMap::new(),
            }),
        ],
    };

    let result = parser::parse(input).expect("Parser should not error");
    assert_eq!(result, expected);
}

/// Test parsing headings at different levels
#[test]
fn test_parse_heading_levels() {
    let input = r#"== Level 1
=== Level 2
==== Level 3"#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 3);

    if let Block::Heading(h) = &result.blocks[0] {
        assert_eq!(h.level, 1);
        assert_eq!(h.text, vec![Inline::Text("Level 1".to_string())]);
    } else {
        panic!("Expected Heading block");
    }

    if let Block::Heading(h) = &result.blocks[1] {
        assert_eq!(h.level, 2);
    } else {
        panic!("Expected Heading block");
    }

    if let Block::Heading(h) = &result.blocks[2] {
        assert_eq!(h.level, 3);
    } else {
        panic!("Expected Heading block");
    }
}

/// Test parsing simple paragraphs
#[test]
fn test_parse_paragraphs() {
    let input = r#"First paragraph.

Second paragraph."#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 2);
    assert!(matches!(&result.blocks[0], Block::Paragraph(_)));
    assert!(matches!(&result.blocks[1], Block::Paragraph(_)));
}

/// Test parsing inline formatting
#[test]
fn test_parse_inline_formatting() {
    let input = "This has *bold*, _italic_, and `mono` text.";

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 1);

    if let Block::Paragraph(p) = &result.blocks[0] {
        // Should contain: Text, Bold, Text, Italic, Text, Mono, Text
        assert!(p.inlines.len() >= 7);

        // Check for bold
        let has_bold = p.inlines.iter().any(|i| {
            matches!(i, Inline::Format(FormatType::Bold, _))
        });
        assert!(has_bold, "Should have bold formatting");

        // Check for italic
        let has_italic = p.inlines.iter().any(|i| {
            matches!(i, Inline::Format(FormatType::Italic, _))
        });
        assert!(has_italic, "Should have italic formatting");

        // Check for monospace
        let has_mono = p.inlines.iter().any(|i| {
            matches!(i, Inline::Format(FormatType::Monospace, _))
        });
        assert!(has_mono, "Should have monospace formatting");
    } else {
        panic!("Expected Paragraph block");
    }
}

/// Test parsing unordered lists
#[test]
fn test_parse_unordered_list() {
    let input = r#"* First item
* Second item
* Third item"#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 1);

    if let Block::List(list) = &result.blocks[0] {
        assert_eq!(list.list_type, ListType::Unordered);
        assert_eq!(list.items.len(), 3);
    } else {
        panic!("Expected List block");
    }
}

/// Test parsing ordered lists
#[test]
fn test_parse_ordered_list() {
    let input = r#". First step
. Second step
. Third step"#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 1);

    if let Block::List(list) = &result.blocks[0] {
        assert_eq!(list.list_type, ListType::Ordered);
        assert_eq!(list.items.len(), 3);
    } else {
        panic!("Expected List block");
    }
}

/// Test parsing document attributes
#[test]
fn test_parse_attributes() {
    let input = r#"= My Document
:author: Jane Doe
:version: 2.0
:toc: left

Content here."#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.metadata.title, Some("My Document".to_string()));
    assert_eq!(
        result.metadata.attributes.get("author"),
        Some(&"Jane Doe".to_string())
    );
    assert_eq!(
        result.metadata.attributes.get("version"),
        Some(&"2.0".to_string())
    );
    assert_eq!(
        result.metadata.attributes.get("toc"),
        Some(&"left".to_string())
    );
}

/// Test nested list items
#[test]
fn test_parse_nested_list() {
    let input = r#"* Parent item
** Child item
** Another child
* Back to parent"#;

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.blocks.len(), 1);

    if let Block::List(list) = &result.blocks[0] {
        assert_eq!(list.list_type, ListType::Unordered);
        // Should have items at different levels
        let has_nested = list.items.iter().any(|item| item.level > 0);
        assert!(has_nested, "Should have nested items");
    } else {
        panic!("Expected List block");
    }
}

/// Test empty document
#[test]
fn test_parse_empty_document() {
    let input = "";

    let result = parser::parse(input).expect("Parser should not error on empty input");

    assert!(result.blocks.is_empty());
    assert!(result.metadata.title.is_none());
}

/// Test document with only title
#[test]
fn test_parse_title_only() {
    let input = "= Just a Title";

    let result = parser::parse(input).expect("Parser should not error");

    assert_eq!(result.metadata.title, Some("Just a Title".to_string()));
    assert!(result.blocks.is_empty());
}
