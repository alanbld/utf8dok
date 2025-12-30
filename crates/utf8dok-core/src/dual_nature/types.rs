//! Core types for the Dual-Nature Documentation System

use std::collections::HashMap;

/// A document with dual-nature content annotations
#[derive(Debug, Clone, Default)]
pub struct DualNatureDocument {
    /// Document title
    pub title: Option<String>,
    /// Document-level attributes
    pub attributes: DocumentAttributes,
    /// Content blocks with dual-nature annotations
    pub blocks: Vec<DualNatureBlock>,
}

impl DualNatureDocument {
    /// Create a new empty dual-nature document
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all blocks for a specific format
    pub fn blocks_for_format(&self, format: OutputFormat) -> Vec<&DualNatureBlock> {
        self.blocks
            .iter()
            .filter(|b| b.selector.matches_format(format))
            .collect()
    }

    /// Get slide-specific attributes
    pub fn slide_attributes(&self) -> &SlideAttributes {
        &self.attributes.slide
    }

    /// Get document-specific attributes
    pub fn document_attributes(&self) -> &DocumentSpecificAttributes {
        &self.attributes.document
    }
}

/// Document-level attributes for dual-nature rendering
#[derive(Debug, Clone, Default)]
pub struct DocumentAttributes {
    /// Author name
    pub author: Option<String>,
    /// Date
    pub date: Option<String>,
    /// Slide-specific attributes
    pub slide: SlideAttributes,
    /// Document-specific attributes
    pub document: DocumentSpecificAttributes,
    /// Generic attributes
    pub generic: HashMap<String, String>,
}

/// Slide-specific document attributes
#[derive(Debug, Clone, Default)]
pub struct SlideAttributes {
    /// PowerPoint template path
    pub template: Option<String>,
    /// Slide master name
    pub slide_master: Option<String>,
    /// Default slide layout
    pub default_layout: Option<String>,
    /// Default bullet count limit
    pub default_bullets: Option<usize>,
}

/// Document-specific attributes
#[derive(Debug, Clone, Default)]
pub struct DocumentSpecificAttributes {
    /// Word template path
    pub template: Option<String>,
    /// Default paragraph style
    pub default_style: Option<String>,
}

/// A content block with dual-nature annotation
#[derive(Debug, Clone)]
pub struct DualNatureBlock {
    /// Content selector determining where this block appears
    pub selector: ContentSelector,
    /// The actual content
    pub content: BlockContent,
    /// Block-level overrides
    pub overrides: BlockOverrides,
    /// Source location for error reporting
    pub source_line: usize,
}

impl DualNatureBlock {
    /// Create a new universal block (appears in all formats)
    pub fn universal(content: BlockContent, line: usize) -> Self {
        Self {
            selector: ContentSelector::Both,
            content,
            overrides: BlockOverrides::default(),
            source_line: line,
        }
    }

    /// Create a slide-only block
    pub fn slide_only(content: BlockContent, line: usize) -> Self {
        Self {
            selector: ContentSelector::SlideOnly,
            content,
            overrides: BlockOverrides::default(),
            source_line: line,
        }
    }

    /// Create a document-only block
    pub fn document_only(content: BlockContent, line: usize) -> Self {
        Self {
            selector: ContentSelector::DocumentOnly,
            content,
            overrides: BlockOverrides::default(),
            source_line: line,
        }
    }

    /// Set slide layout override
    pub fn with_slide_layout(mut self, layout: impl Into<String>) -> Self {
        self.overrides.slide_layout = Some(layout.into());
        self
    }

    /// Set bullet limit override
    pub fn with_bullet_limit(mut self, limit: usize) -> Self {
        self.overrides.slide_bullets = Some(limit);
        self
    }
}

/// Content selector for dual-nature blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentSelector {
    /// Appears in both document and slides
    #[default]
    Both,
    /// Appears in slides, may have simplified content
    Slide,
    /// Appears only in slides
    SlideOnly,
    /// Appears in document, may have detailed content
    Document,
    /// Appears only in document
    DocumentOnly,
    /// Conditional based on format
    Conditional(FormatCondition),
}

impl ContentSelector {
    /// Check if this selector matches the given output format
    pub fn matches_format(&self, format: OutputFormat) -> bool {
        match (self, format) {
            (ContentSelector::Both, _) => true,
            (ContentSelector::Slide, OutputFormat::Slide) => true,
            (ContentSelector::Slide, OutputFormat::Pptx) => true,
            (ContentSelector::SlideOnly, OutputFormat::Slide) => true,
            (ContentSelector::SlideOnly, OutputFormat::Pptx) => true,
            (ContentSelector::Document, OutputFormat::Document) => true,
            (ContentSelector::Document, OutputFormat::Docx) => true,
            (ContentSelector::DocumentOnly, OutputFormat::Document) => true,
            (ContentSelector::DocumentOnly, OutputFormat::Docx) => true,
            (ContentSelector::Conditional(cond), format) => cond.matches(format),
            _ => false,
        }
    }

    /// Parse from annotation string
    pub fn from_annotation(annotation: &str) -> Self {
        match annotation.trim().to_lowercase().as_str() {
            "slide" | ".slide" => ContentSelector::Slide,
            "slide-only" | ".slide-only" => ContentSelector::SlideOnly,
            "document" | ".document" => ContentSelector::Document,
            "document-only" | ".document-only" => ContentSelector::DocumentOnly,
            "both" | ".both" => ContentSelector::Both,
            s if s.starts_with("if-slide") => {
                ContentSelector::Conditional(FormatCondition::IfSlide)
            }
            s if s.starts_with("if-document") => {
                ContentSelector::Conditional(FormatCondition::IfDocument)
            }
            _ => ContentSelector::Both,
        }
    }
}

/// Format condition for conditional blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCondition {
    /// Include only when rendering slides
    IfSlide,
    /// Include only when rendering documents
    IfDocument,
    /// Include only for specific format
    IfFormat(OutputFormat),
}

impl FormatCondition {
    /// Check if this condition matches the given format
    pub fn matches(&self, format: OutputFormat) -> bool {
        match self {
            FormatCondition::IfSlide => {
                matches!(format, OutputFormat::Slide | OutputFormat::Pptx)
            }
            FormatCondition::IfDocument => {
                matches!(format, OutputFormat::Document | OutputFormat::Docx)
            }
            FormatCondition::IfFormat(f) => *f == format,
        }
    }
}

/// Output format for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Generic slide format
    Slide,
    /// PowerPoint format
    Pptx,
    /// Generic document format
    Document,
    /// Word document format
    Docx,
    /// HTML format (can show both views)
    Html,
}

/// Block-level rendering overrides
#[derive(Debug, Clone, Default)]
pub struct BlockOverrides {
    /// PowerPoint layout name
    pub slide_layout: Option<String>,
    /// Maximum bullet points for slides
    pub slide_bullets: Option<usize>,
    /// Document style name
    pub document_style: Option<String>,
    /// Slide-specific style
    pub slide_style: Option<String>,
}

/// The actual content of a block
#[derive(Debug, Clone)]
pub enum BlockContent {
    /// Section heading
    Section(SectionContent),
    /// Paragraph text
    Paragraph(String),
    /// Bullet list
    BulletList(Vec<String>),
    /// Numbered list
    NumberedList(Vec<String>),
    /// Code block
    Code(CodeContent),
    /// Image
    Image(ImageContent),
    /// Table
    Table(TableContent),
    /// Include directive
    Include(IncludeContent),
    /// Raw content passthrough
    Raw(String),
}

/// Section/heading content
#[derive(Debug, Clone)]
pub struct SectionContent {
    /// Heading level (1-6)
    pub level: usize,
    /// Heading text
    pub title: String,
    /// Section ID for cross-references
    pub id: Option<String>,
    /// Nested content
    pub children: Vec<DualNatureBlock>,
}

/// Code block content
#[derive(Debug, Clone)]
pub struct CodeContent {
    /// Language identifier
    pub language: Option<String>,
    /// Code text
    pub code: String,
    /// Optional caption
    pub caption: Option<String>,
}

/// Image content
#[derive(Debug, Clone)]
pub struct ImageContent {
    /// Image path or URL
    pub path: String,
    /// Alt text
    pub alt: Option<String>,
    /// Width specification
    pub width: Option<String>,
    /// Height specification
    pub height: Option<String>,
    /// Caption
    pub caption: Option<String>,
    /// Slide-specific image (e.g., simplified version)
    pub slide_path: Option<String>,
}

/// Table content
#[derive(Debug, Clone)]
pub struct TableContent {
    /// Table headers
    pub headers: Vec<String>,
    /// Table rows
    pub rows: Vec<Vec<String>>,
    /// Table caption
    pub caption: Option<String>,
}

/// Include directive content
#[derive(Debug, Clone)]
pub struct IncludeContent {
    /// Path to include
    pub path: String,
    /// Line range (optional)
    pub lines: Option<(usize, usize)>,
    /// Tag to include (optional)
    pub tag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_selector_matching() {
        assert!(ContentSelector::Both.matches_format(OutputFormat::Slide));
        assert!(ContentSelector::Both.matches_format(OutputFormat::Document));

        assert!(ContentSelector::Slide.matches_format(OutputFormat::Slide));
        assert!(ContentSelector::Slide.matches_format(OutputFormat::Pptx));
        assert!(!ContentSelector::Slide.matches_format(OutputFormat::Document));

        assert!(ContentSelector::DocumentOnly.matches_format(OutputFormat::Document));
        assert!(!ContentSelector::DocumentOnly.matches_format(OutputFormat::Slide));
    }

    #[test]
    fn test_content_selector_from_annotation() {
        assert_eq!(
            ContentSelector::from_annotation(".slide"),
            ContentSelector::Slide
        );
        assert_eq!(
            ContentSelector::from_annotation("document-only"),
            ContentSelector::DocumentOnly
        );
        assert_eq!(
            ContentSelector::from_annotation("unknown"),
            ContentSelector::Both
        );
    }

    #[test]
    fn test_format_condition_matching() {
        assert!(FormatCondition::IfSlide.matches(OutputFormat::Slide));
        assert!(FormatCondition::IfSlide.matches(OutputFormat::Pptx));
        assert!(!FormatCondition::IfSlide.matches(OutputFormat::Document));

        assert!(FormatCondition::IfDocument.matches(OutputFormat::Document));
        assert!(FormatCondition::IfDocument.matches(OutputFormat::Docx));
        assert!(!FormatCondition::IfDocument.matches(OutputFormat::Slide));
    }

    #[test]
    fn test_dual_nature_block_builders() {
        let block = DualNatureBlock::slide_only(BlockContent::Paragraph("Test".to_string()), 1)
            .with_slide_layout("Title-And-Content")
            .with_bullet_limit(3);

        assert_eq!(block.selector, ContentSelector::SlideOnly);
        assert_eq!(
            block.overrides.slide_layout,
            Some("Title-And-Content".to_string())
        );
        assert_eq!(block.overrides.slide_bullets, Some(3));
    }
}
