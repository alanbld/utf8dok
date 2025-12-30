//! Dual-Nature Documentation System
//!
//! Enables single-source documentation that produces both detailed documents (DOCX)
//! and executive presentations (PPTX) through intelligent content annotations.
//!
//! # Content Selectors
//!
//! - `[.slide]` - Content for presentation slides
//! - `[.document]` - Content for detailed document
//! - `[.slide-only]` - Appears only in slides
//! - `[.document-only]` - Appears only in documents
//! - `[.both]` or no annotation - Appears in both formats
//!
//! # Conditional Blocks
//!
//! - `[.if-slide]` - Include content only when rendering slides
//! - `[.if-document]` - Include content only when rendering documents
//!
//! # Structural Overrides
//!
//! - `:slide-layout:` - Specify PowerPoint layout (e.g., "Title-And-Content")
//! - `:slide-bullets:` - Limit bullet points for slides (e.g., "3")
//! - `:document-style:` - Document-specific styling hints
//!
//! # Example
//!
//! ```asciidoc
//! = ADR-001: Architecture Decision
//! :slide-master: Executive-Deck
//!
//! [.slide]
//! == Executive Summary
//! :slide-layout: Title-And-Content
//! :slide-bullets: 3
//! * Key point 1
//! * Key point 2
//! * Key point 3
//!
//! [.document-only]
//! == Detailed Analysis
//! This section contains comprehensive analysis...
//! ```

mod parser;
mod transformer;
mod types;
mod validator;

pub use parser::DualNatureParser;
pub use transformer::{ContentTransformer, DocumentView};
pub use types::*;
pub use validator::{DualNatureValidator, ValidationResult};

/// Parse dual-nature annotations from content and create a DualNatureDocument
pub fn parse_dual_nature(content: &str) -> DualNatureDocument {
    DualNatureParser::parse(content)
}

/// Transform a DualNatureDocument for a specific output format
pub fn transform_for_format(
    doc: &DualNatureDocument,
    format: OutputFormat,
) -> Vec<DualNatureBlock> {
    ContentTransformer::transform(doc, format)
}

/// Validate consistency between document and slide views
pub fn validate_dual_nature(doc: &DualNatureDocument) -> ValidationResult {
    DualNatureValidator::validate(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_slide_annotation() {
        let content = r#"= Title

[.slide]
== Slide Content
* Point 1
* Point 2
"#;
        let doc = parse_dual_nature(content);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_transform_for_slide_format() {
        let content = r#"= Title

[.slide]
== For Slides Only

[.document-only]
== For Document Only
"#;
        let doc = parse_dual_nature(content);
        let slide_blocks = transform_for_format(&doc, OutputFormat::Slide);
        let doc_blocks = transform_for_format(&doc, OutputFormat::Document);

        // Slide format should exclude document-only
        // Document format should exclude slide-only
        assert!(slide_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::Slide)));
        assert!(doc_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::DocumentOnly)));
    }

    #[test]
    fn test_validate_dual_nature() {
        let content = r#"= Title

[.slide]
== Summary
* Key points

[.document]
== Details
Full explanation here.
"#;
        let doc = parse_dual_nature(content);
        let result = validate_dual_nature(&doc);
        assert!(result.is_valid);
    }
}
