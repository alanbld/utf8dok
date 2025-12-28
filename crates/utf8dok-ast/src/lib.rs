//! utf8dok-ast - Abstract Syntax Tree definitions
//!
//! This crate provides the AST types used by utf8dok for representing
//! parsed document structures. It serves as the Intermediate Representation (IR)
//! bridging OOXML extraction and AsciiDoc rendering.
//!
//! # Module Structure
//!
//! - [`document`] - Document root and metadata
//! - [`block`] - Block-level elements (paragraphs, headings, lists, tables)
//! - [`inline`] - Inline elements (text, formatting, links, images)
//!
//! # Example
//!
//! ```
//! use utf8dok_ast::{Document, Block, Paragraph, Inline};
//! use std::collections::HashMap;
//!
//! let mut doc = Document::with_title("My Document");
//! doc.push(Block::Paragraph(Paragraph {
//!     inlines: vec![Inline::Text("Hello, world!".to_string())],
//!     style_id: None,
//!     attributes: HashMap::new(),
//! }));
//! ```

pub mod block;
pub mod document;
pub mod inline;
pub mod intent;

// Re-export key types for convenience
pub use block::{
    Admonition, AdmonitionType, Alignment, Block, BreakType, ColumnSpec, Heading, List, ListItem,
    ListType, LiteralBlock, Paragraph, Table, TableCell, TableRow,
};
pub use document::{Document, DocumentMeta};
pub use inline::{FormatType, Image, Inline, Link};
pub use intent::{DocumentIntent, Invariant, ValidationLevel};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exports work
        let _doc = Document::new();
        let _inline = Inline::Text("test".to_string());
        let _block = Block::Break(BreakType::Page);
    }
}
