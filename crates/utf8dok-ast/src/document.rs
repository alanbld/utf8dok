//! Document root and metadata definitions
//!
//! This module defines the top-level document structure and metadata
//! that represents both OOXML and AsciiDoc documents.

use std::collections::HashMap;

use crate::block::Block;

/// A complete document
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Document metadata (title, authors, attributes)
    pub metadata: DocumentMeta,
    /// Document content blocks
    pub blocks: Vec<Block>,
}

/// Document metadata
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentMeta {
    /// Document title
    pub title: Option<String>,
    /// Document authors
    pub authors: Vec<String>,
    /// Revision/version string
    pub revision: Option<String>,
    /// Additional attributes (AsciiDoc attributes, OOXML properties)
    pub attributes: HashMap<String, String>,
}

impl Document {
    /// Create a new empty document
    pub fn new() -> Self {
        Self {
            metadata: DocumentMeta::default(),
            blocks: Vec::new(),
        }
    }

    /// Create a document with a title
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            metadata: DocumentMeta {
                title: Some(title.into()),
                ..Default::default()
            },
            blocks: Vec::new(),
        }
    }

    /// Add a block to the document
    pub fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Check if the document is empty (no blocks)
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get the number of blocks
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentMeta {
    /// Create metadata with just a title
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    /// Add an author
    pub fn add_author(&mut self, author: impl Into<String>) {
        self.authors.push(author.into());
    }

    /// Set an attribute
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Get an attribute
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Paragraph;
    use crate::inline::Inline;

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        assert!(doc.is_empty());
        assert_eq!(doc.len(), 0);
    }

    #[test]
    fn test_document_with_title() {
        let doc = Document::with_title("My Document");
        assert_eq!(doc.metadata.title, Some("My Document".to_string()));
    }

    #[test]
    fn test_document_push_block() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text("Hello".to_string())],
            style_id: None,
            attributes: HashMap::new(),
        }));
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_metadata_attributes() {
        let mut meta = DocumentMeta::default();
        meta.set_attribute("lang", "en");
        assert_eq!(meta.get_attribute("lang"), Some("en"));
    }

    #[test]
    fn test_metadata_authors() {
        let mut meta = DocumentMeta::default();
        meta.add_author("Alice");
        meta.add_author("Bob");
        assert_eq!(meta.authors.len(), 2);
    }
}
