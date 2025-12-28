//! utf8dok-core - Plain text, powerful docs
//!
//! Core library for utf8dok, providing AsciiDoc parsing and generation.
//!
//! # Modules
//!
//! - [`parser`] - Parse AsciiDoc text into AST
//! - [`generator`] - Generate AsciiDoc text from AST
//!
//! # Example
//!
//! ```
//! use utf8dok_ast::{Document, Block, Heading, Paragraph, Inline};
//! use utf8dok_core::generate;
//! use std::collections::HashMap;
//!
//! let mut doc = Document::new();
//! doc.push(Block::Heading(Heading {
//!     level: 1,
//!     text: vec![Inline::Text("Hello".to_string())],
//!     style_id: None,
//!     anchor: None,
//! }));
//! doc.push(Block::Paragraph(Paragraph {
//!     inlines: vec![Inline::Text("World".to_string())],
//!     style_id: None,
//!     attributes: HashMap::new(),
//! }));
//!
//! let asciidoc = generate(&doc);
//! // Level 1 heading generates == prefix (level + 1 equals signs)
//! assert!(asciidoc.contains("== Hello"));
//! assert!(asciidoc.contains("World"));
//! ```

pub mod diagnostics;
pub mod generator;
pub mod parser;

// Re-export main types and functions
pub use diagnostics::{Diagnostic, Diagnostics, Severity, Span};
pub use generator::{generate, generate_with_config, AsciiDocGenerator, GeneratorConfig};
pub use parser::parse;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use utf8dok_ast::{Block, Document, Heading, Inline, Link, Paragraph};
    use std::collections::HashMap;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    /// Test round-trip fidelity: AST -> AsciiDoc -> AST
    /// Heading levels should be preserved exactly.
    #[test]
    fn test_roundtrip_heading_levels() {
        // Create AST with heading level 1
        let mut doc = Document::new();
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Section One".to_string())],
            style_id: None,
            anchor: None,
        }));

        // Generate AsciiDoc
        let asciidoc = generate(&doc);
        assert!(asciidoc.contains("== Section One"), "Level 1 should generate ==");

        // Parse back to AST
        let parsed = parse(&asciidoc).unwrap();
        assert_eq!(parsed.blocks.len(), 1);

        // Verify heading level is preserved
        if let Block::Heading(h) = &parsed.blocks[0] {
            assert_eq!(h.level, 1, "Heading level should be preserved as 1");
        } else {
            panic!("Expected Heading block");
        }
    }

    /// Test round-trip fidelity for cross-references
    #[test]
    fn test_roundtrip_cross_reference() {
        // Create AST with internal link
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Text("Go to ".to_string()),
                Inline::Link(Link {
                    url: "#section1".to_string(),
                    text: vec![Inline::Text("Section One".to_string())],
                }),
                Inline::Text(".".to_string()),
            ],
            style_id: None,
            attributes: HashMap::new(),
        }));

        // Generate AsciiDoc
        let asciidoc = generate(&doc);
        assert!(asciidoc.contains("<<section1,Section One>>"), "Should generate xref syntax");

        // Parse back to AST
        let parsed = parse(&asciidoc).unwrap();
        assert_eq!(parsed.blocks.len(), 1);

        // Verify link is preserved
        if let Block::Paragraph(p) = &parsed.blocks[0] {
            // Find the Link inline
            let has_link = p.inlines.iter().any(|inline| {
                if let Inline::Link(link) = inline {
                    link.url == "#section1"
                } else {
                    false
                }
            });
            assert!(has_link, "Link should be preserved with #section1 url");
        } else {
            panic!("Expected Paragraph block");
        }
    }

    /// Test complete round-trip with multiple elements
    #[test]
    fn test_roundtrip_complete() {
        let mut doc = Document::new();

        // Heading level 1
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Introduction".to_string())],
            style_id: None,
            anchor: None,
        }));

        // Paragraph with link
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Text("See ".to_string()),
                Inline::Link(Link {
                    url: "#details".to_string(),
                    text: vec![Inline::Text("details section".to_string())],
                }),
                Inline::Text(" for more.".to_string()),
            ],
            style_id: None,
            attributes: HashMap::new(),
        }));

        // Heading level 2
        doc.push(Block::Heading(Heading {
            level: 2,
            text: vec![Inline::Text("Details".to_string())],
            style_id: None,
            anchor: None,
        }));

        // Generate and parse
        let asciidoc = generate(&doc);
        let parsed = parse(&asciidoc).unwrap();

        // Verify structure
        assert_eq!(parsed.blocks.len(), 3);

        // Check heading levels
        if let Block::Heading(h) = &parsed.blocks[0] {
            assert_eq!(h.level, 1);
        }
        if let Block::Heading(h) = &parsed.blocks[2] {
            assert_eq!(h.level, 2);
        }

        // Check link is preserved
        if let Block::Paragraph(p) = &parsed.blocks[1] {
            let has_correct_link = p.inlines.iter().any(|inline| {
                matches!(inline, Inline::Link(link) if link.url == "#details")
            });
            assert!(has_correct_link);
        }
    }
}
