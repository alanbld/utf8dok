//! Content transformer for dual-nature documents
//!
//! Transforms DualNatureDocument content for specific output formats,
//! applying filtering, simplification, and structural overrides.

use super::types::*;

/// Content transformer for format-specific rendering
pub struct ContentTransformer;

impl ContentTransformer {
    /// Transform a document for a specific output format
    pub fn transform(doc: &DualNatureDocument, format: OutputFormat) -> Vec<DualNatureBlock> {
        let mut result = Vec::new();

        for block in &doc.blocks {
            if let Some(transformed) = Self::transform_block(block, format, doc) {
                result.push(transformed);
            }
        }

        result
    }

    /// Transform a single block for the given format
    fn transform_block(
        block: &DualNatureBlock,
        format: OutputFormat,
        doc: &DualNatureDocument,
    ) -> Option<DualNatureBlock> {
        // Check if block should be included for this format
        if !block.selector.matches_format(format) {
            return None;
        }

        // Apply format-specific transformations
        let transformed_content = match &block.content {
            BlockContent::BulletList(items) => {
                Self::transform_bullet_list(items, format, block, doc)
            }
            BlockContent::Section(section) => Self::transform_section(section, format, block, doc),
            BlockContent::Paragraph(text) => Self::transform_paragraph(text, format),
            BlockContent::Image(img) => Self::transform_image(img, format),
            // Pass through other content types
            other => other.clone(),
        };

        Some(DualNatureBlock {
            selector: block.selector,
            content: transformed_content,
            overrides: block.overrides.clone(),
            source_line: block.source_line,
        })
    }

    /// Transform bullet list for format (may truncate for slides)
    fn transform_bullet_list(
        items: &[String],
        format: OutputFormat,
        block: &DualNatureBlock,
        doc: &DualNatureDocument,
    ) -> BlockContent {
        match format {
            OutputFormat::Slide | OutputFormat::Pptx => {
                // Apply bullet limit for slides
                let limit = block
                    .overrides
                    .slide_bullets
                    .or(doc.attributes.slide.default_bullets)
                    .unwrap_or(5);

                let truncated: Vec<String> = items.iter().take(limit).cloned().collect();

                // Simplify bullet text for slides (remove excess detail)
                let simplified: Vec<String> = truncated
                    .iter()
                    .map(|item| Self::simplify_for_slide(item))
                    .collect();

                BlockContent::BulletList(simplified)
            }
            _ => BlockContent::BulletList(items.to_vec()),
        }
    }

    /// Transform section for format
    fn transform_section(
        section: &SectionContent,
        format: OutputFormat,
        _block: &DualNatureBlock,
        doc: &DualNatureDocument,
    ) -> BlockContent {
        // Transform children recursively
        let transformed_children: Vec<DualNatureBlock> = section
            .children
            .iter()
            .filter_map(|child| Self::transform_block(child, format, doc))
            .collect();

        BlockContent::Section(SectionContent {
            level: section.level,
            title: section.title.clone(),
            id: section.id.clone(),
            children: transformed_children,
        })
    }

    /// Transform paragraph for format
    fn transform_paragraph(text: &str, format: OutputFormat) -> BlockContent {
        match format {
            OutputFormat::Slide | OutputFormat::Pptx => {
                // Shorten paragraphs for slides
                BlockContent::Paragraph(Self::simplify_for_slide(text))
            }
            _ => BlockContent::Paragraph(text.to_string()),
        }
    }

    /// Transform image for format
    fn transform_image(img: &ImageContent, format: OutputFormat) -> BlockContent {
        match format {
            OutputFormat::Slide | OutputFormat::Pptx => {
                // Use slide-specific image if available
                let path = img.slide_path.clone().unwrap_or_else(|| img.path.clone());
                BlockContent::Image(ImageContent {
                    path,
                    alt: img.alt.clone(),
                    width: img.width.clone().or(Some("80%".to_string())),
                    height: img.height.clone(),
                    caption: None, // Slides typically don't show captions
                    slide_path: img.slide_path.clone(),
                })
            }
            _ => BlockContent::Image(img.clone()),
        }
    }

    /// Simplify text for slide format
    fn simplify_for_slide(text: &str) -> String {
        // Remove parenthetical remarks for slides
        let simplified = Self::remove_parentheticals(text);

        // Truncate if too long (slides should be concise)
        const MAX_SLIDE_TEXT: usize = 80;
        if simplified.len() > MAX_SLIDE_TEXT {
            let truncated = &simplified[..MAX_SLIDE_TEXT];
            // Find last word boundary
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        } else {
            simplified
        }
    }

    /// Remove parenthetical content from text
    fn remove_parentheticals(text: &str) -> String {
        let mut result = String::new();
        let mut depth = 0;

        for ch in text.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ if depth == 0 => {
                    result.push(ch);
                }
                _ => {}
            }
        }

        // Clean up double spaces
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Get the slide layout for a block
    pub fn get_slide_layout(block: &DualNatureBlock, doc: &DualNatureDocument) -> Option<String> {
        block
            .overrides
            .slide_layout
            .clone()
            .or_else(|| doc.attributes.slide.default_layout.clone())
    }

    /// Check if content needs speaker notes
    pub fn needs_speaker_notes(block: &DualNatureBlock) -> bool {
        // Long paragraphs or detailed sections might need speaker notes
        match &block.content {
            BlockContent::Paragraph(text) => text.len() > 200,
            BlockContent::BulletList(items) => items.len() > 5,
            _ => false,
        }
    }

    /// Generate speaker notes from document content
    pub fn generate_speaker_notes(block: &DualNatureBlock) -> Option<String> {
        match &block.content {
            BlockContent::Paragraph(text) if text.len() > 80 => {
                Some(format!("Key points: {}", text))
            }
            BlockContent::BulletList(items) if items.len() > 3 => {
                let additional: Vec<_> = items.iter().skip(3).cloned().collect();
                if !additional.is_empty() {
                    Some(format!(
                        "Additional points not shown:\n{}",
                        additional.join("\n")
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// View of a dual-nature document for a specific format
pub struct DocumentView<'a> {
    /// The source document
    pub source: &'a DualNatureDocument,
    /// The target format
    pub format: OutputFormat,
    /// Transformed blocks
    pub blocks: Vec<DualNatureBlock>,
}

impl<'a> DocumentView<'a> {
    /// Create a new document view for the given format
    pub fn new(source: &'a DualNatureDocument, format: OutputFormat) -> Self {
        let blocks = ContentTransformer::transform(source, format);
        Self {
            source,
            format,
            blocks,
        }
    }

    /// Get the title
    pub fn title(&self) -> Option<&str> {
        self.source.title.as_deref()
    }

    /// Get blocks for iteration
    pub fn iter(&self) -> impl Iterator<Item = &DualNatureBlock> {
        self.blocks.iter()
    }

    /// Get slide-specific metadata
    pub fn slide_metadata(&self) -> Option<&SlideAttributes> {
        match self.format {
            OutputFormat::Slide | OutputFormat::Pptx => Some(&self.source.attributes.slide),
            _ => None,
        }
    }

    /// Get document-specific metadata
    pub fn document_metadata(&self) -> Option<&DocumentSpecificAttributes> {
        match self.format {
            OutputFormat::Document | OutputFormat::Docx => Some(&self.source.attributes.document),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_nature::DualNatureParser;

    #[test]
    fn test_transform_filters_by_format() {
        let content = r#"= Title

[.slide-only]
== Slide Section

[.document-only]
== Document Section
"#;
        let doc = DualNatureParser::parse(content);

        let slide_blocks = ContentTransformer::transform(&doc, OutputFormat::Slide);
        let doc_blocks = ContentTransformer::transform(&doc, OutputFormat::Document);

        // Slide format should have slide-only but not document-only
        assert!(slide_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::SlideOnly)));
        assert!(!slide_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::DocumentOnly)));

        // Document format should have document-only but not slide-only
        assert!(doc_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::DocumentOnly)));
        assert!(!doc_blocks
            .iter()
            .any(|b| matches!(b.selector, ContentSelector::SlideOnly)));
    }

    #[test]
    fn test_bullet_truncation_for_slides() {
        let content = r#"= Title
:slide-bullets: 3

[.slide]
== Summary

* Point 1
* Point 2
* Point 3
* Point 4
* Point 5
"#;
        let doc = DualNatureParser::parse(content);
        let slide_blocks = ContentTransformer::transform(&doc, OutputFormat::Slide);

        let list_block = slide_blocks
            .iter()
            .find(|b| matches!(b.content, BlockContent::BulletList(_)));

        assert!(list_block.is_some());
        if let BlockContent::BulletList(items) = &list_block.unwrap().content {
            assert_eq!(items.len(), 3); // Truncated to 3
        }
    }

    #[test]
    fn test_simplify_for_slide() {
        let text = "This is a detailed explanation (with some additional notes) that goes on for quite a while";
        let simplified = ContentTransformer::simplify_for_slide(text);

        assert!(!simplified.contains("with some additional notes"));
        assert!(simplified.len() <= 83); // MAX + "..."
    }

    #[test]
    fn test_remove_parentheticals() {
        assert_eq!(
            ContentTransformer::remove_parentheticals("Hello (world) there"),
            "Hello there"
        );
        assert_eq!(
            ContentTransformer::remove_parentheticals("No parens here"),
            "No parens here"
        );
    }

    #[test]
    fn test_document_view() {
        let content = r#"= My Presentation
:slide-master: Corporate

[.slide]
== Overview

* Key point
"#;
        let doc = DualNatureParser::parse(content);
        let view = DocumentView::new(&doc, OutputFormat::Slide);

        assert_eq!(view.title(), Some("My Presentation"));
        assert!(view.slide_metadata().is_some());
        assert_eq!(
            view.slide_metadata().unwrap().slide_master,
            Some("Corporate".to_string())
        );
    }

    #[test]
    fn test_generate_speaker_notes() {
        let block = DualNatureBlock {
            selector: ContentSelector::Slide,
            content: BlockContent::BulletList(vec![
                "Point 1".to_string(),
                "Point 2".to_string(),
                "Point 3".to_string(),
                "Point 4".to_string(),
                "Point 5".to_string(),
            ]),
            overrides: BlockOverrides::default(),
            source_line: 1,
        };

        let notes = ContentTransformer::generate_speaker_notes(&block);
        assert!(notes.is_some());
        assert!(notes.unwrap().contains("Point 4"));
    }
}
