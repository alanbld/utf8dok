//! Diagram Block Specification Tests
//!
//! These tests verify the parsing of literal blocks with block attributes,
//! including diagram blocks like `[mermaid]`, `[plantuml]`, etc.

use utf8dok_ast::{Block, Document, LiteralBlock};
use utf8dok_core::{generate, parse};

/// Test that block attributes are captured by the parser.
/// Input: `[source,rust]` followed by a literal block.
#[test]
fn test_parse_block_attributes() {
    let input = r#"[source,rust]
----
fn main() {}
----"#;

    let doc = parse(input).expect("Should parse successfully");
    assert_eq!(doc.blocks.len(), 1, "Should have one block");

    match &doc.blocks[0] {
        Block::Literal(lit) => {
            assert_eq!(lit.language, Some("rust".to_string()), "Language should be rust");
            assert!(lit.content.contains("fn main()"), "Content should contain code");
        }
        _ => panic!("Expected Block::Literal, got {:?}", doc.blocks[0]),
    }
}

/// Test parsing a mermaid diagram block.
#[test]
fn test_parse_mermaid_diagram() {
    let input = r#"[mermaid]
----
graph TD;
    A-->B;
----"#;

    let doc = parse(input).expect("Should parse successfully");
    assert_eq!(doc.blocks.len(), 1, "Should have one block");

    match &doc.blocks[0] {
        Block::Literal(lit) => {
            // Mermaid is a style, not a language
            assert_eq!(lit.style_id, Some("mermaid".to_string()), "Style should be mermaid");
            assert!(lit.content.contains("graph TD"), "Content should contain diagram code");
            assert!(lit.content.contains("A-->B"), "Content should contain nodes");
        }
        _ => panic!("Expected Block::Literal, got {:?}", doc.blocks[0]),
    }
}

/// Test parsing a plantuml diagram block.
#[test]
fn test_parse_plantuml_diagram() {
    let input = r#"[plantuml]
----
@startuml
Alice -> Bob: Hello
@enduml
----"#;

    let doc = parse(input).expect("Should parse successfully");
    assert_eq!(doc.blocks.len(), 1, "Should have one block");

    match &doc.blocks[0] {
        Block::Literal(lit) => {
            assert_eq!(lit.style_id, Some("plantuml".to_string()), "Style should be plantuml");
            assert!(lit.content.contains("@startuml"), "Content should contain plantuml");
        }
        _ => panic!("Expected Block::Literal, got {:?}", doc.blocks[0]),
    }
}

/// Test parsing a literal block without attributes.
#[test]
fn test_parse_plain_literal_block() {
    let input = r#"----
Plain literal text.
No formatting.
----"#;

    let doc = parse(input).expect("Should parse successfully");
    assert_eq!(doc.blocks.len(), 1, "Should have one block");

    match &doc.blocks[0] {
        Block::Literal(lit) => {
            assert_eq!(lit.language, None, "Language should be None");
            assert_eq!(lit.style_id, None, "Style should be None");
            assert!(lit.content.contains("Plain literal text"), "Content preserved");
        }
        _ => panic!("Expected Block::Literal, got {:?}", doc.blocks[0]),
    }
}

/// Test that diagrams integrate with other content.
#[test]
fn test_parse_diagram_with_context() {
    let input = r#"== Architecture

Here is the system diagram:

[mermaid]
----
graph LR;
    Client-->Server;
----

As shown above, the client connects to the server."#;

    let doc = parse(input).expect("Should parse successfully");

    // Should have: Heading, Paragraph, Literal, Paragraph
    assert_eq!(doc.blocks.len(), 4, "Should have 4 blocks");

    // Check heading
    assert!(matches!(&doc.blocks[0], Block::Heading(_)));

    // Check first paragraph
    assert!(matches!(&doc.blocks[1], Block::Paragraph(_)));

    // Check diagram
    match &doc.blocks[2] {
        Block::Literal(lit) => {
            assert_eq!(lit.style_id, Some("mermaid".to_string()));
        }
        _ => panic!("Expected Block::Literal for diagram"),
    }

    // Check trailing paragraph
    assert!(matches!(&doc.blocks[3], Block::Paragraph(_)));
}

/// Test round-trip: AST -> AsciiDoc -> AST
/// Verifies that diagram blocks survive the round-trip.
#[test]
fn test_roundtrip_diagram() {
    // Construct AST manually with a diagram block
    let mut doc = Document::new();
    doc.push(Block::Literal(LiteralBlock {
        content: "graph TD;\n    A-->B;".to_string(),
        language: None,
        title: None,
        style_id: Some("mermaid".to_string()),
    }));

    // Generate AsciiDoc
    let asciidoc = generate(&doc);

    // Should contain the mermaid attribute and delimiters
    assert!(asciidoc.contains("[mermaid]") || asciidoc.contains("[source,mermaid]"),
            "Generated AsciiDoc should have mermaid attribute: {}", asciidoc);
    assert!(asciidoc.contains("----"), "Should have delimiters");
    assert!(asciidoc.contains("graph TD"), "Should have content");

    // Parse back
    let parsed = parse(&asciidoc).expect("Should parse generated AsciiDoc");
    assert_eq!(parsed.blocks.len(), 1, "Should have one block after round-trip");

    // Verify fidelity
    match &parsed.blocks[0] {
        Block::Literal(lit) => {
            assert_eq!(lit.style_id, Some("mermaid".to_string()),
                       "Style should be preserved");
            assert!(lit.content.contains("graph TD"),
                    "Content should be preserved");
        }
        _ => panic!("Expected Block::Literal after round-trip"),
    }
}

/// Test source block with language attribute.
#[test]
fn test_roundtrip_source_block() {
    let mut doc = Document::new();
    doc.push(Block::Literal(LiteralBlock {
        content: "println!(\"Hello\");".to_string(),
        language: Some("rust".to_string()),
        title: None,
        style_id: None,
    }));

    let asciidoc = generate(&doc);
    assert!(asciidoc.contains("[source,rust]"), "Should have source,rust attribute");

    let parsed = parse(&asciidoc).expect("Should parse");
    match &parsed.blocks[0] {
        Block::Literal(lit) => {
            assert_eq!(lit.language, Some("rust".to_string()));
        }
        _ => panic!("Expected Block::Literal"),
    }
}

/// Test multiple diagram types in sequence.
#[test]
fn test_multiple_diagrams() {
    let input = r#"[mermaid]
----
graph TD; A-->B;
----

[plantuml]
----
@startuml
Bob -> Alice
@enduml
----"#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.blocks.len(), 2, "Should have two blocks");

    match &doc.blocks[0] {
        Block::Literal(lit) => assert_eq!(lit.style_id, Some("mermaid".to_string())),
        _ => panic!("First should be mermaid"),
    }

    match &doc.blocks[1] {
        Block::Literal(lit) => assert_eq!(lit.style_id, Some("plantuml".to_string())),
        _ => panic!("Second should be plantuml"),
    }
}
