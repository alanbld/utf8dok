//! utf8dok-core - Plain text, powerful docs
//!
//! Core library for utf8dok, providing AsciiDoc generation from AST.
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
//! assert!(asciidoc.contains("= Hello"));
//! assert!(asciidoc.contains("World"));
//! ```

pub mod generator;

// Re-export main types and functions
pub use generator::{generate, generate_with_config, AsciiDocGenerator, GeneratorConfig};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
