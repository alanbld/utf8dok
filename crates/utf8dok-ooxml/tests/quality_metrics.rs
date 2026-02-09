//! Quality Metrics Integration Tests
//!
//! End-to-end tests that verify the quality of DOCX output by parsing AsciiDoc
//! and checking the generated XML for expected elements (hyperlinks, bookmarks,
//! tables, lists, etc.).

use std::collections::HashMap;

use utf8dok_ast::{Block, Document, DocumentMeta, Heading, Inline, Link, Paragraph};
use utf8dok_ooxml::test_utils::{create_minimal_template, extract_document_xml};
use utf8dok_ooxml::writer::DocxWriter;

/// Helper: parse AsciiDoc, render to DOCX, extract document.xml
fn render_asciidoc(input: &str) -> String {
    let doc = utf8dok_core::parser::parse(input).expect("parse should succeed");
    let template = create_minimal_template();
    let result = DocxWriter::generate(&doc, &template).expect("generate should succeed");
    extract_document_xml(&result)
}

/// Helper: render an AST Document to document.xml string
fn render_doc(doc: &Document) -> String {
    let template = create_minimal_template();
    let result = DocxWriter::generate(doc, &template).expect("generate should succeed");
    extract_document_xml(&result)
}

/// Count occurrences of a substring in text
fn count_occurrences(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}

// =============================================================================
// Paragraph Count
// =============================================================================

#[test]
fn test_quality_paragraph_count() {
    let input = r#"= Test Document

== Introduction

First paragraph.

Second paragraph.

Third paragraph.
"#;
    let xml = render_asciidoc(input);

    // Count <w:p> elements (includes heading paragraph + 3 body paragraphs)
    let p_count = count_occurrences(&xml, "<w:p>");
    assert!(
        p_count >= 4,
        "Expected at least 4 <w:p> elements (1 heading + 3 paragraphs), got {}",
        p_count
    );
}

// =============================================================================
// Hyperlink Generation
// =============================================================================

#[test]
fn test_quality_hyperlink_url_with_text() {
    let input = r#"= Links Test

Visit https://example.com[Example Site] for more info.
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("w:hyperlink"),
        "DOCX should contain <w:hyperlink> for URL links"
    );
    assert!(
        xml.contains("Example Site"),
        "Link display text should appear in DOCX"
    );
}

#[test]
fn test_quality_hyperlink_link_macro() {
    let input = r#"= Links Test

See link:https://docs.rs[Rust Docs] for API reference.
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("w:hyperlink"),
        "DOCX should contain <w:hyperlink> for link: macro"
    );
    assert!(
        xml.contains("Rust Docs"),
        "Link macro text should appear in DOCX"
    );
}

#[test]
fn test_quality_hyperlink_bare_url() {
    let input = r#"= Links Test

Check out https://github.com for repositories.
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("w:hyperlink"),
        "DOCX should contain <w:hyperlink> for bare URLs"
    );
    assert!(
        xml.contains("https://github.com"),
        "Bare URL should appear as link text in DOCX"
    );
}

#[test]
fn test_quality_multiple_hyperlinks() {
    let input = r#"= Multi-Link Test

Visit https://example.com[Example] and https://rust-lang.org[Rust].
"#;
    let xml = render_asciidoc(input);

    let hyperlink_count = count_occurrences(&xml, "w:hyperlink");
    assert!(
        hyperlink_count >= 2,
        "Expected at least 2 hyperlinks, got {}",
        hyperlink_count / 2 // start + end tags
    );
}

// =============================================================================
// Bookmark Generation (Heading Anchors)
// =============================================================================

#[test]
fn test_quality_bookmark_generation() {
    let input = r#"= Document

== Introduction

Some text.

== Conclusion

Final text.
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("w:bookmarkStart"),
        "Headings should generate <w:bookmarkStart> bookmarks"
    );
    assert!(
        xml.contains("w:bookmarkEnd"),
        "Headings should generate <w:bookmarkEnd> bookmarks"
    );

    // Both headings should produce bookmarks
    let bookmark_count = count_occurrences(&xml, "w:bookmarkStart");
    assert!(
        bookmark_count >= 2,
        "Expected at least 2 bookmarks (one per heading), got {}",
        bookmark_count
    );
}

#[test]
fn test_quality_bookmark_names_match_anchors() {
    let input = r#"= Test

== Getting Started

Content here.
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("getting-started"),
        "Bookmark name should be the generated anchor 'getting-started'"
    );
}

// =============================================================================
// Table Preservation
// =============================================================================

#[test]
fn test_quality_table_preservation() {
    let input = r#"= Tables Test

|===
| Header 1 | Header 2

| Cell A | Cell B
| Cell C | Cell D
|===
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("<w:tbl>"),
        "Tables should produce <w:tbl> elements"
    );
    assert!(
        xml.contains("<w:tr>") || xml.contains("<w:tr "),
        "Tables should produce <w:tr> row elements"
    );
    assert!(
        xml.contains("<w:tc>") || xml.contains("<w:tc "),
        "Tables should produce <w:tc> cell elements"
    );
}

// =============================================================================
// List Rendering
// =============================================================================

#[test]
fn test_quality_list_rendering() {
    let input = r#"= Lists Test

* First item
* Second item
* Third item
"#;
    let xml = render_asciidoc(input);

    // List items are rendered as styled paragraphs
    let p_count = count_occurrences(&xml, "<w:p>");
    assert!(
        p_count >= 3,
        "List with 3 items should produce at least 3 paragraphs, got {}",
        p_count
    );

    // List items should have content
    assert!(xml.contains("First item"), "List item text should appear");
    assert!(xml.contains("Third item"), "Last list item should appear");
}

#[test]
fn test_quality_ordered_list() {
    let input = r#"= Ordered List

. Step one
. Step two
. Step three
"#;
    let xml = render_asciidoc(input);

    assert!(
        xml.contains("Step one"),
        "Ordered list items should appear in DOCX"
    );
    assert!(
        xml.contains("Step three"),
        "Last ordered list item should appear"
    );
}

// =============================================================================
// Cross-reference Roundtrip
// =============================================================================

#[test]
fn test_quality_crossref_roundtrip() {
    // Build AST directly to test cross-reference rendering
    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![
            Block::Heading(Heading {
                level: 1,
                text: vec![Inline::Text("Target Section".to_string())],
                style_id: None,
                anchor: Some("target-section".to_string()),
            }),
            Block::Paragraph(Paragraph {
                inlines: vec![
                    Inline::Text("See ".to_string()),
                    Inline::Link(Link {
                        url: "#target-section".to_string(),
                        text: vec![Inline::Text("Target Section".to_string())],
                    }),
                    Inline::Text(" for details.".to_string()),
                ],
                style_id: None,
                attributes: HashMap::new(),
            }),
        ],
    };

    let xml = render_doc(&doc);

    // Heading should have bookmark
    assert!(
        xml.contains("w:bookmarkStart"),
        "Heading with anchor should produce bookmark"
    );
    assert!(
        xml.contains("target-section"),
        "Bookmark name should match anchor"
    );

    // Cross-reference should produce internal hyperlink
    assert!(
        xml.contains("w:anchor=\"target-section\""),
        "Cross-reference should produce w:hyperlink with w:anchor"
    );
}

#[test]
fn test_quality_crossref_parsed_from_asciidoc() {
    let input = r#"= Doc

== Target Section

Go back to <<target-section,Target Section>>.
"#;
    let xml = render_asciidoc(input);

    // The heading should produce a bookmark
    assert!(
        xml.contains("w:bookmarkStart"),
        "Parsed heading should auto-generate bookmark"
    );

    // The <<ref>> should produce an internal hyperlink
    assert!(
        xml.contains("w:anchor="),
        "Cross-reference should produce w:anchor hyperlink"
    );
}

// =============================================================================
// Combined Quality Check
// =============================================================================

#[test]
fn test_quality_comprehensive_document() {
    let input = r#"= Comprehensive Document
:author: Test Author

== Introduction

This is the first section with a link to https://example.com[Example].

== Features

Key features include:

* Fast processing
* Template injection
* Round-trip editing

=== Technical Details

|===
| Feature | Status
| Parsing | Complete
| Rendering | Complete
|===

See <<introduction>> for context.
"#;
    let xml = render_asciidoc(input);

    // Verify all element types present
    assert!(xml.contains("<w:p>"), "Should have paragraphs");
    assert!(xml.contains("w:bookmarkStart"), "Should have bookmarks");
    assert!(xml.contains("w:hyperlink"), "Should have hyperlinks");
    assert!(xml.contains("<w:tbl>"), "Should have tables");
    assert!(xml.contains("Fast processing"), "List items preserved");
    assert!(xml.contains("Example"), "Link text preserved");
}
