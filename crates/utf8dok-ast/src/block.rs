//! Block-level elements for document structure
//!
//! This module defines block-level elements that form the document structure,
//! such as paragraphs, headings, lists, tables, and admonitions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::inline::Inline;

/// Block-level content element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Block {
    /// A paragraph of text
    Paragraph(Paragraph),
    /// A section heading
    Heading(Heading),
    /// An ordered or unordered list
    List(List),
    /// A table
    Table(Table),
    /// An admonition block (note, warning, etc.)
    Admonition(Admonition),
    /// A literal/code block
    Literal(LiteralBlock),
    /// A page or section break
    Break(BreakType),
}

/// A paragraph block
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Paragraph {
    /// Inline content within the paragraph
    pub inlines: Vec<Inline>,
    /// Style ID from source document (e.g., OOXML style reference)
    pub style_id: Option<String>,
    /// Additional attributes (AsciiDoc roles, etc.)
    pub attributes: HashMap<String, String>,
}

/// A section heading
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heading {
    /// Heading level (1-6, where 1 is the highest)
    pub level: u8,
    /// Heading text content
    pub text: Vec<Inline>,
    /// Style ID from source document
    pub style_id: Option<String>,
    /// Anchor/ID for cross-references
    pub anchor: Option<String>,
}

/// A list (ordered or unordered)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
    /// Type of list
    pub list_type: ListType,
    /// List items
    pub items: Vec<ListItem>,
    /// Style ID from source document
    pub style_id: Option<String>,
}

/// List type variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListType {
    /// Unordered/bullet list
    Unordered,
    /// Ordered/numbered list
    Ordered,
    /// Description/definition list
    Description,
}

/// A single list item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// Item content (can contain nested blocks)
    pub content: Vec<Block>,
    /// Nesting level (0-based)
    pub level: u8,
    /// For description lists: the term being defined
    pub term: Option<Vec<Inline>>,
}

/// A table
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// Table rows
    pub rows: Vec<TableRow>,
    /// Style ID from source document
    pub style_id: Option<String>,
    /// Table caption
    pub caption: Option<Vec<Inline>>,
    /// Column specifications
    pub columns: Vec<ColumnSpec>,
}

/// A table row
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    /// Cells in this row
    pub cells: Vec<TableCell>,
    /// Whether this is a header row
    pub is_header: bool,
}

/// A table cell
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Cell content (blocks)
    pub content: Vec<Block>,
    /// Column span
    pub colspan: u32,
    /// Row span
    pub rowspan: u32,
    /// Horizontal alignment
    pub align: Option<Alignment>,
}

/// Column specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnSpec {
    /// Relative width (e.g., 1, 2, 3 for proportional sizing)
    pub width: Option<u32>,
    /// Default alignment for this column
    pub align: Option<Alignment>,
}

/// Text alignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// An admonition block (note, warning, tip, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Admonition {
    /// Type of admonition
    pub admonition_type: AdmonitionType,
    /// Admonition content
    pub content: Vec<Block>,
    /// Optional title
    pub title: Option<Vec<Inline>>,
}

/// Admonition type variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdmonitionType {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// A literal/code block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiteralBlock {
    /// The literal content
    pub content: String,
    /// Language for syntax highlighting
    pub language: Option<String>,
    /// Optional title/caption
    pub title: Option<String>,
    /// Style ID from source document
    pub style_id: Option<String>,
}

/// Break type variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BreakType {
    /// Page break
    Page,
    /// Section break
    Section,
}

impl Default for Heading {
    fn default() -> Self {
        Self {
            level: 1,
            text: Vec::new(),
            style_id: None,
            anchor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_default() {
        let para = Paragraph::default();
        assert!(para.inlines.is_empty());
        assert!(para.style_id.is_none());
    }

    #[test]
    fn test_heading_levels() {
        let h1 = Heading {
            level: 1,
            text: vec![Inline::Text("Title".to_string())],
            style_id: None,
            anchor: Some("title".to_string()),
        };
        assert_eq!(h1.level, 1);
        assert_eq!(h1.anchor, Some("title".to_string()));
    }

    #[test]
    fn test_list_types() {
        let list = List {
            list_type: ListType::Ordered,
            items: vec![],
            style_id: None,
        };
        assert_eq!(list.list_type, ListType::Ordered);
    }

    #[test]
    fn test_table_structure() {
        let table = Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    content: vec![],
                    colspan: 1,
                    rowspan: 1,
                    align: None,
                }],
                is_header: true,
            }],
            style_id: None,
            caption: None,
            columns: vec![],
        };
        assert!(table.rows[0].is_header);
    }
}
