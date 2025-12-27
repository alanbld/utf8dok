//! Conversion from OOXML types to utf8dok-ast types
//!
//! This module provides the bridge between the low-level OOXML parsing
//! and the unified AST representation.

use std::collections::HashMap;

use utf8dok_ast::{
    Block as AstBlock, BreakType as AstBreakType, FormatType, Heading, Inline,
    Paragraph as AstParagraph, Table as AstTable, TableCell as AstTableCell,
    TableRow as AstTableRow,
};

use crate::document::{Block, Document, Paragraph, Run, Table, TableCell, TableRow};
use crate::styles::StyleSheet;

/// Context for conversion, holding style information
pub struct ConversionContext<'a> {
    /// Style sheet for resolving heading levels
    pub styles: Option<&'a StyleSheet>,
}

impl<'a> ConversionContext<'a> {
    /// Create a new context without styles
    pub fn new() -> Self {
        Self { styles: None }
    }

    /// Create a context with style information
    pub fn with_styles(styles: &'a StyleSheet) -> Self {
        Self {
            styles: Some(styles),
        }
    }

    /// Check if a style ID represents a heading and return its level
    pub fn heading_level(&self, style_id: &str) -> Option<u8> {
        // First check style sheet for outline level
        if let Some(styles) = self.styles {
            if let Some(level) = styles.heading_level(style_id) {
                return Some(level);
            }
        }

        // Fallback: parse "Heading1", "Heading2", etc.
        style_id
            .strip_prefix("Heading")
            .or_else(|| style_id.strip_prefix("heading"))
            .and_then(|suffix| suffix.parse::<u8>().ok())
    }
}

impl Default for ConversionContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for converting OOXML types to AST types
pub trait ToAst {
    /// The AST type this converts to
    type Output;

    /// Convert to AST representation
    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output;
}

// =============================================================================
// Run -> Inline conversion
// =============================================================================

impl ToAst for Run {
    type Output = Inline;

    fn to_ast(&self, _ctx: &ConversionContext) -> Self::Output {
        // Start with the base text
        let mut inline = Inline::Text(self.text.clone());

        // Apply formatting in order: monospace, italic, bold
        // This creates nested wrappers: Bold(Italic(Monospace(Text)))
        if self.monospace {
            inline = Inline::Format(FormatType::Monospace, Box::new(inline));
        }

        if self.italic {
            inline = Inline::Format(FormatType::Italic, Box::new(inline));
        }

        if self.bold {
            inline = Inline::Format(FormatType::Bold, Box::new(inline));
        }

        inline
    }
}

// =============================================================================
// Paragraph -> Block conversion
// =============================================================================

impl ToAst for Paragraph {
    type Output = AstBlock;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        // Convert all runs to inlines
        let inlines: Vec<Inline> = self.runs.iter().map(|r| r.to_ast(ctx)).collect();

        // Check if this is a heading
        if let Some(ref style_id) = self.style_id {
            if let Some(level) = ctx.heading_level(style_id) {
                return AstBlock::Heading(Heading {
                    level,
                    text: inlines,
                    style_id: Some(style_id.clone()),
                    anchor: None, // TODO: generate from text
                });
            }
        }

        // Regular paragraph
        AstBlock::Paragraph(AstParagraph {
            inlines,
            style_id: self.style_id.clone(),
            attributes: HashMap::new(),
        })
    }
}

// =============================================================================
// Table -> Block conversion
// =============================================================================

impl ToAst for TableRow {
    type Output = AstTableRow;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        AstTableRow {
            cells: self.cells.iter().map(|c| c.to_ast(ctx)).collect(),
            is_header: self.is_header,
        }
    }
}

impl ToAst for TableCell {
    type Output = AstTableCell;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        AstTableCell {
            content: self.paragraphs.iter().map(|p| p.to_ast(ctx)).collect(),
            colspan: 1,
            rowspan: 1,
            align: None,
        }
    }
}

impl ToAst for Table {
    type Output = AstBlock;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        AstBlock::Table(AstTable {
            rows: self.rows.iter().map(|r| r.to_ast(ctx)).collect(),
            style_id: self.style_id.clone(),
            caption: None,
            columns: Vec::new(),
        })
    }
}

// =============================================================================
// Block -> Block conversion
// =============================================================================

impl ToAst for Block {
    type Output = AstBlock;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        match self {
            Block::Paragraph(p) => p.to_ast(ctx),
            Block::Table(t) => t.to_ast(ctx),
            Block::SectionBreak => AstBlock::Break(AstBreakType::Section),
        }
    }
}

// =============================================================================
// Document -> Document conversion
// =============================================================================

impl ToAst for Document {
    type Output = utf8dok_ast::Document;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        utf8dok_ast::Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: self.blocks.iter().map(|b| b.to_ast(ctx)).collect(),
        }
    }
}

// =============================================================================
// Convenience functions
// =============================================================================

/// Convert an OOXML document to AST without style information
pub fn convert_document(doc: &Document) -> utf8dok_ast::Document {
    let ctx = ConversionContext::new();
    doc.to_ast(&ctx)
}

/// Convert an OOXML document to AST with style information
pub fn convert_document_with_styles(
    doc: &Document,
    styles: &StyleSheet,
) -> utf8dok_ast::Document {
    let ctx = ConversionContext::with_styles(styles);
    doc.to_ast(&ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_plain_text() {
        let run = Run {
            text: "Hello".to_string(),
            bold: false,
            italic: false,
            monospace: false,
        };
        let ctx = ConversionContext::new();
        let inline = run.to_ast(&ctx);

        assert_eq!(inline, Inline::Text("Hello".to_string()));
    }

    #[test]
    fn test_run_bold() {
        let run = Run {
            text: "Bold".to_string(),
            bold: true,
            italic: false,
            monospace: false,
        };
        let ctx = ConversionContext::new();
        let inline = run.to_ast(&ctx);

        assert_eq!(
            inline,
            Inline::Format(FormatType::Bold, Box::new(Inline::Text("Bold".to_string())))
        );
    }

    #[test]
    fn test_run_bold_italic() {
        let run = Run {
            text: "Both".to_string(),
            bold: true,
            italic: true,
            monospace: false,
        };
        let ctx = ConversionContext::new();
        let inline = run.to_ast(&ctx);

        // Should be Bold(Italic(Text))
        assert_eq!(
            inline,
            Inline::Format(
                FormatType::Bold,
                Box::new(Inline::Format(
                    FormatType::Italic,
                    Box::new(Inline::Text("Both".to_string()))
                ))
            )
        );
    }

    #[test]
    fn test_run_all_formatting() {
        let run = Run {
            text: "All".to_string(),
            bold: true,
            italic: true,
            monospace: true,
        };
        let ctx = ConversionContext::new();
        let inline = run.to_ast(&ctx);

        // Should be Bold(Italic(Monospace(Text)))
        assert_eq!(
            inline,
            Inline::Format(
                FormatType::Bold,
                Box::new(Inline::Format(
                    FormatType::Italic,
                    Box::new(Inline::Format(
                        FormatType::Monospace,
                        Box::new(Inline::Text("All".to_string()))
                    ))
                ))
            )
        );
    }

    #[test]
    fn test_paragraph_simple() {
        let para = Paragraph {
            style_id: None,
            runs: vec![Run {
                text: "Hello world".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            }],
            numbering: None,
        };
        let ctx = ConversionContext::new();
        let block = para.to_ast(&ctx);

        if let AstBlock::Paragraph(p) = block {
            assert_eq!(p.inlines.len(), 1);
            assert_eq!(p.inlines[0], Inline::Text("Hello world".to_string()));
            assert!(p.style_id.is_none());
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_paragraph_with_mixed_runs() {
        let para = Paragraph {
            style_id: Some("Normal".to_string()),
            runs: vec![
                Run {
                    text: "Normal ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                },
                Run {
                    text: "bold".to_string(),
                    bold: true,
                    italic: false,
                    monospace: false,
                },
                Run {
                    text: " text".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                },
            ],
            numbering: None,
        };
        let ctx = ConversionContext::new();
        let block = para.to_ast(&ctx);

        if let AstBlock::Paragraph(p) = block {
            assert_eq!(p.inlines.len(), 3);
            assert_eq!(p.inlines[0], Inline::Text("Normal ".to_string()));
            assert_eq!(
                p.inlines[1],
                Inline::Format(FormatType::Bold, Box::new(Inline::Text("bold".to_string())))
            );
            assert_eq!(p.inlines[2], Inline::Text(" text".to_string()));
            assert_eq!(p.style_id, Some("Normal".to_string()));
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_heading_detection_by_style_name() {
        let para = Paragraph {
            style_id: Some("Heading1".to_string()),
            runs: vec![Run {
                text: "Chapter One".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            }],
            numbering: None,
        };
        let ctx = ConversionContext::new();
        let block = para.to_ast(&ctx);

        if let AstBlock::Heading(h) = block {
            assert_eq!(h.level, 1);
            assert_eq!(h.text.len(), 1);
            assert_eq!(h.text[0], Inline::Text("Chapter One".to_string()));
            assert_eq!(h.style_id, Some("Heading1".to_string()));
        } else {
            panic!("Expected Heading block, got {:?}", block);
        }
    }

    #[test]
    fn test_heading_level_2() {
        let para = Paragraph {
            style_id: Some("Heading2".to_string()),
            runs: vec![Run {
                text: "Section".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            }],
            numbering: None,
        };
        let ctx = ConversionContext::new();
        let block = para.to_ast(&ctx);

        if let AstBlock::Heading(h) = block {
            assert_eq!(h.level, 2);
        } else {
            panic!("Expected Heading block");
        }
    }

    #[test]
    fn test_document_conversion() {
        let doc = Document {
            blocks: vec![
                Block::Paragraph(Paragraph {
                    style_id: Some("Heading1".to_string()),
                    runs: vec![Run {
                        text: "Title".to_string(),
                        bold: false,
                        italic: false,
                        monospace: false,
                    }],
                    numbering: None,
                }),
                Block::Paragraph(Paragraph {
                    style_id: None,
                    runs: vec![Run {
                        text: "Body text".to_string(),
                        bold: false,
                        italic: false,
                        monospace: false,
                    }],
                    numbering: None,
                }),
                Block::SectionBreak,
            ],
        };

        let ast_doc = convert_document(&doc);

        assert_eq!(ast_doc.blocks.len(), 3);

        // First block should be a heading
        if let AstBlock::Heading(h) = &ast_doc.blocks[0] {
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected Heading");
        }

        // Second block should be a paragraph
        assert!(matches!(&ast_doc.blocks[1], AstBlock::Paragraph(_)));

        // Third block should be a section break
        assert!(matches!(
            &ast_doc.blocks[2],
            AstBlock::Break(AstBreakType::Section)
        ));
    }

    #[test]
    fn test_table_conversion() {
        let table = Table {
            style_id: Some("TableGrid".to_string()),
            rows: vec![
                TableRow {
                    cells: vec![
                        TableCell {
                            paragraphs: vec![Paragraph {
                                style_id: None,
                                runs: vec![Run {
                                    text: "Header".to_string(),
                                    bold: true,
                                    italic: false,
                                    monospace: false,
                                }],
                                numbering: None,
                            }],
                        },
                    ],
                    is_header: true,
                },
                TableRow {
                    cells: vec![
                        TableCell {
                            paragraphs: vec![Paragraph {
                                style_id: None,
                                runs: vec![Run {
                                    text: "Data".to_string(),
                                    bold: false,
                                    italic: false,
                                    monospace: false,
                                }],
                                numbering: None,
                            }],
                        },
                    ],
                    is_header: false,
                },
            ],
        };

        let ctx = ConversionContext::new();
        let block = table.to_ast(&ctx);

        if let AstBlock::Table(t) = block {
            assert_eq!(t.rows.len(), 2);
            assert!(t.rows[0].is_header);
            assert!(!t.rows[1].is_header);
            assert_eq!(t.style_id, Some("TableGrid".to_string()));
        } else {
            panic!("Expected Table block");
        }
    }
}
