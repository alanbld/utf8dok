//! Conversion from OOXML types to utf8dok-ast types
//!
//! This module provides the bridge between the low-level OOXML parsing
//! and the unified AST representation.

use std::collections::HashMap;

use utf8dok_ast::{
    Block as AstBlock, BreakType as AstBreakType, FormatType, Heading, Inline, Link as AstLink,
    Paragraph as AstParagraph, Table as AstTable, TableCell as AstTableCell,
    TableRow as AstTableRow,
};

use crate::document::{
    Block, Document, Hyperlink, Paragraph, ParagraphChild, Run, Table, TableCell, TableRow,
};
use crate::relationships::Relationships;
use crate::styles::StyleSheet;

/// Context for conversion, holding style and relationship information
pub struct ConversionContext<'a> {
    /// Style sheet for resolving heading levels
    pub styles: Option<&'a StyleSheet>,
    /// Relationships for resolving hyperlink targets
    pub relationships: Option<&'a Relationships>,
}

impl<'a> ConversionContext<'a> {
    /// Create a new context without styles or relationships
    pub fn new() -> Self {
        Self {
            styles: None,
            relationships: None,
        }
    }

    /// Create a context with style information
    pub fn with_styles(styles: &'a StyleSheet) -> Self {
        Self {
            styles: Some(styles),
            relationships: None,
        }
    }

    /// Create a context with styles and relationships
    pub fn with_styles_and_rels(styles: &'a StyleSheet, rels: &'a Relationships) -> Self {
        Self {
            styles: Some(styles),
            relationships: Some(rels),
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
// Hyperlink -> Inline conversion
// =============================================================================

impl ToAst for Hyperlink {
    type Output = Inline;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        // Resolve the target URL
        let target = if let Some(ref id) = self.id {
            // External link - look up in relationships
            ctx.relationships
                .and_then(|rels| rels.get(id))
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("#{}", id))
        } else if let Some(ref anchor) = self.anchor {
            // Internal anchor link
            format!("#{}", anchor)
        } else {
            "#".to_string()
        };

        // Convert child runs to inlines
        let children: Vec<Inline> = self.runs.iter().map(|r| r.to_ast(ctx)).collect();

        // Create link inline using the Link struct
        Inline::Link(AstLink {
            url: target,
            text: children,
        })
    }
}

// =============================================================================
// ParagraphChild -> Inline conversion
// =============================================================================

impl ToAst for ParagraphChild {
    type Output = Vec<Inline>;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        match self {
            ParagraphChild::Run(run) => vec![run.to_ast(ctx)],
            ParagraphChild::Hyperlink(hyperlink) => vec![hyperlink.to_ast(ctx)],
            ParagraphChild::Image(img) => {
                // Convert image to inline - placeholder text with alt if available
                let text = img.alt.clone().unwrap_or_else(|| "[image]".to_string());
                vec![Inline::Text(text)]
            }
            ParagraphChild::Bookmark(bookmark) => {
                // Convert bookmark to inline anchor
                vec![Inline::Anchor(bookmark.name.clone())]
            }
        }
    }
}

// =============================================================================
// Paragraph -> Block conversion
// =============================================================================

impl ToAst for Paragraph {
    type Output = AstBlock;

    fn to_ast(&self, ctx: &ConversionContext) -> Self::Output {
        // Convert all children to inlines
        let inlines: Vec<Inline> = self
            .children
            .iter()
            .flat_map(|child| child.to_ast(ctx))
            .collect();

        // Check if this is a heading
        if let Some(ref style_id) = self.style_id {
            if let Some(level) = ctx.heading_level(style_id) {
                // Generate anchor from heading text
                let plain_text = extract_plain_text(&inlines);
                let anchor = generate_anchor(&plain_text);

                return AstBlock::Heading(Heading {
                    level,
                    text: inlines,
                    style_id: Some(style_id.clone()),
                    anchor,
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
            intent: None,
        }
    }
}

// =============================================================================
// Convenience functions
// =============================================================================

/// Generate a slug/anchor ID from text
///
/// Converts text to a URL-friendly anchor:
/// - Lowercase
/// - Spaces become hyphens
/// - Remove non-alphanumeric (except hyphens)
/// - Collapse multiple hyphens
/// - Trim leading/trailing hyphens
fn generate_anchor(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let slug: String = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                // Skip other characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect();

    // Collapse multiple hyphens and trim
    let mut result = String::new();
    let mut prev_hyphen = true; // Start true to skip leading hyphens
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
                prev_hyphen = true;
            }
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extract plain text from inlines for anchor generation
fn extract_plain_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => text.push_str(t),
            Inline::Format(_, inner) => {
                text.push_str(&extract_plain_text(&[(**inner).clone()]));
            }
            Inline::Span(inner) => {
                text.push_str(&extract_plain_text(inner));
            }
            Inline::Link(link) => {
                text.push_str(&extract_plain_text(&link.text));
            }
            _ => {} // Skip images, breaks, anchors
        }
    }
    text
}

/// Convert an OOXML document to AST without style information
pub fn convert_document(doc: &Document) -> utf8dok_ast::Document {
    let ctx = ConversionContext::new();
    doc.to_ast(&ctx)
}

/// Convert an OOXML document to AST with style information
pub fn convert_document_with_styles(doc: &Document, styles: &StyleSheet) -> utf8dok_ast::Document {
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
            children: vec![ParagraphChild::Run(Run {
                text: "Hello world".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            })],
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
            children: vec![
                ParagraphChild::Run(Run {
                    text: "Normal ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Run(Run {
                    text: "bold".to_string(),
                    bold: true,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Run(Run {
                    text: " text".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
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
            children: vec![ParagraphChild::Run(Run {
                text: "Chapter One".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            })],
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
            children: vec![ParagraphChild::Run(Run {
                text: "Section".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            })],
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
                    children: vec![ParagraphChild::Run(Run {
                        text: "Title".to_string(),
                        bold: false,
                        italic: false,
                        monospace: false,
                    })],
                    numbering: None,
                }),
                Block::Paragraph(Paragraph {
                    style_id: None,
                    children: vec![ParagraphChild::Run(Run {
                        text: "Body text".to_string(),
                        bold: false,
                        italic: false,
                        monospace: false,
                    })],
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
    fn test_generate_anchor_simple() {
        assert_eq!(generate_anchor("Hello World"), Some("hello-world".into()));
        assert_eq!(generate_anchor("Introduction"), Some("introduction".into()));
        assert_eq!(
            generate_anchor("Chapter 1: Getting Started"),
            Some("chapter-1-getting-started".into())
        );
    }

    #[test]
    fn test_generate_anchor_special_chars() {
        assert_eq!(
            generate_anchor("What's New?"),
            Some("whats-new".into())
        );
        assert_eq!(
            generate_anchor("C++ Programming"),
            Some("c-programming".into())
        );
        assert_eq!(
            generate_anchor("  Multiple   Spaces  "),
            Some("multiple-spaces".into())
        );
    }

    #[test]
    fn test_generate_anchor_edge_cases() {
        assert_eq!(generate_anchor(""), None);
        assert_eq!(generate_anchor("   "), None);
        assert_eq!(generate_anchor("???"), None);
        assert_eq!(generate_anchor("123"), Some("123".into()));
        assert_eq!(generate_anchor("a"), Some("a".into()));
    }

    #[test]
    fn test_extract_plain_text_simple() {
        let inlines = vec![Inline::Text("Hello World".to_string())];
        assert_eq!(extract_plain_text(&inlines), "Hello World");
    }

    #[test]
    fn test_extract_plain_text_formatted() {
        let inlines = vec![
            Inline::Text("Plain ".to_string()),
            Inline::Format(
                FormatType::Bold,
                Box::new(Inline::Text("bold".to_string())),
            ),
            Inline::Text(" text".to_string()),
        ];
        assert_eq!(extract_plain_text(&inlines), "Plain bold text");
    }

    #[test]
    fn test_heading_anchor_generation() {
        let para = Paragraph {
            style_id: Some("Heading1".to_string()),
            children: vec![ParagraphChild::Run(Run {
                text: "Introduction to Rust".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            })],
            numbering: None,
        };
        let ctx = ConversionContext::new();
        let block = para.to_ast(&ctx);

        if let AstBlock::Heading(h) = block {
            assert_eq!(h.level, 1);
            assert_eq!(h.anchor, Some("introduction-to-rust".to_string()));
        } else {
            panic!("Expected Heading block");
        }
    }

    #[test]
    fn test_table_conversion() {
        let table = Table {
            style_id: Some("TableGrid".to_string()),
            rows: vec![
                TableRow {
                    cells: vec![TableCell {
                        paragraphs: vec![Paragraph {
                            style_id: None,
                            children: vec![ParagraphChild::Run(Run {
                                text: "Header".to_string(),
                                bold: true,
                                italic: false,
                                monospace: false,
                            })],
                            numbering: None,
                        }],
                    }],
                    is_header: true,
                },
                TableRow {
                    cells: vec![TableCell {
                        paragraphs: vec![Paragraph {
                            style_id: None,
                            children: vec![ParagraphChild::Run(Run {
                                text: "Data".to_string(),
                                bold: false,
                                italic: false,
                                monospace: false,
                            })],
                            numbering: None,
                        }],
                    }],
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

    // ==================== Sprint 7: ConversionContext Tests ====================

    #[test]
    fn test_context_with_styles() {
        use crate::styles::StyleSheet;

        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:style w:type="paragraph" w:styleId="CustomHeading1" w:customStyle="1">
                <w:name w:val="Custom Heading 1"/>
                <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
            </w:style>
            <w:style w:type="paragraph" w:styleId="CustomHeading2" w:customStyle="1">
                <w:name w:val="Custom Heading 2"/>
                <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
            </w:style>
        </w:styles>"#;

        let styles = StyleSheet::parse(xml).unwrap();
        let ctx = ConversionContext::with_styles(&styles);

        // Should detect heading level from outline level in stylesheet
        assert_eq!(ctx.heading_level("CustomHeading1"), Some(1));
        assert_eq!(ctx.heading_level("CustomHeading2"), Some(2));
        assert_eq!(ctx.heading_level("NonExistent"), None);
    }

    #[test]
    fn test_context_with_styles_and_rels() {
        use crate::relationships::Relationships;
        use crate::styles::StyleSheet;

        let styles_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:style w:type="paragraph" w:styleId="Normal">
                <w:name w:val="Normal"/>
            </w:style>
        </w:styles>"#;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
        </Relationships>"#;

        let styles = StyleSheet::parse(styles_xml).unwrap();
        let rels = Relationships::parse(rels_xml).unwrap();
        let ctx = ConversionContext::with_styles_and_rels(&styles, &rels);

        // Verify both styles and relationships are accessible
        assert!(ctx.styles.is_some());
        assert!(ctx.relationships.is_some());
        assert_eq!(ctx.relationships.unwrap().get("rId1"), Some("https://example.com"));
    }

    #[test]
    fn test_heading_level_fallback() {
        // Without styles, should fall back to parsing style ID
        let ctx = ConversionContext::new();

        assert_eq!(ctx.heading_level("Heading1"), Some(1));
        assert_eq!(ctx.heading_level("Heading2"), Some(2));
        assert_eq!(ctx.heading_level("Heading9"), Some(9));
        assert_eq!(ctx.heading_level("heading3"), Some(3)); // lowercase
        assert_eq!(ctx.heading_level("Normal"), None);
        assert_eq!(ctx.heading_level("HeadingX"), None); // Not a number
    }

    #[test]
    fn test_heading_level_stylesheet_takes_precedence() {
        use crate::styles::StyleSheet;

        // Custom style with outline level 2, even though ID suggests level 1
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:style w:type="paragraph" w:styleId="Heading1">
                <w:name w:val="heading 1"/>
                <w:pPr><w:outlineLvl w:val="2"/></w:pPr>
            </w:style>
        </w:styles>"#;

        let styles = StyleSheet::parse(xml).unwrap();
        let ctx = ConversionContext::with_styles(&styles);

        // Should use outline level (3), not ID-based (1)
        assert_eq!(ctx.heading_level("Heading1"), Some(3));
    }

    #[test]
    fn test_context_default() {
        let ctx = ConversionContext::default();
        assert!(ctx.styles.is_none());
        assert!(ctx.relationships.is_none());
    }

    #[test]
    fn test_hyperlink_with_context() {
        use crate::relationships::Relationships;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://rust-lang.org" TargetMode="External"/>
        </Relationships>"#;

        let rels = Relationships::parse(rels_xml).unwrap();
        let ctx = ConversionContext {
            styles: None,
            relationships: Some(&rels),
        };

        // Create hyperlink with relationship ID
        let hyperlink = Hyperlink {
            id: Some("rId5".to_string()),
            anchor: None,
            runs: vec![Run {
                text: "Rust".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            }],
        };

        let inline = hyperlink.to_ast(&ctx);

        if let Inline::Link(link) = inline {
            assert_eq!(link.url, "https://rust-lang.org");
        } else {
            panic!("Expected Link inline");
        }
    }

    #[test]
    fn test_section_break_conversion_explicit() {
        let doc = Document {
            blocks: vec![Block::SectionBreak],
        };

        let ast_doc = convert_document(&doc);
        assert_eq!(ast_doc.blocks.len(), 1);
        assert!(matches!(
            &ast_doc.blocks[0],
            AstBlock::Break(AstBreakType::Section)
        ));
    }
}
