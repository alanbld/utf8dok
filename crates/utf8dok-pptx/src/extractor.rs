//! Slide extraction from AsciiDoc AST.
//!
//! This module implements the "Presentation Bridge" that converts utf8dok AST
//! into a slide deck based on ADR-010 rules:
//!
//! - **Explicit Mode**: If `[slides]` blocks exist, only render those
//! - **Implicit Mode**: Map `== Headings` to slides (Reveal.js convention)
//! - **Speaker Notes**: `[.notes]` sidebars become speaker notes

use crate::slide::{
    CodeContent, ListContent, ListItem as SlideListItem, Slide, SlideContent, SlideLayoutHint,
    SpeakerNotes, TextContent, TextRun,
};
use utf8dok_ast::{Block, Document, Heading, Inline, List, ListItem, ListType, Sidebar};

/// Configuration for slide extraction
#[derive(Debug, Clone, Default)]
pub struct ExtractorConfig {
    /// Include document title as first slide
    pub include_title_slide: bool,

    /// Use explicit mode only (ignore content outside [slides] blocks)
    pub explicit_only: bool,
}

impl ExtractorConfig {
    /// Create config that includes title slide
    pub fn with_title_slide() -> Self {
        Self {
            include_title_slide: true,
            ..Default::default()
        }
    }
}

/// A complete slide deck (collection of slides)
#[derive(Debug, Clone, Default)]
pub struct Deck {
    /// Presentation title
    pub title: Option<String>,

    /// Presentation subtitle
    pub subtitle: Option<String>,

    /// Author(s)
    pub authors: Vec<String>,

    /// All slides in the deck
    pub slides: Vec<Slide>,
}

impl Deck {
    /// Create a new empty deck
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a deck with a title
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    /// Add a slide to the deck
    pub fn push(&mut self, slide: Slide) {
        self.slides.push(slide);
    }

    /// Get the number of slides
    pub fn len(&self) -> usize {
        self.slides.len()
    }

    /// Check if the deck is empty
    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }
}

/// Extracts slides from an AsciiDoc AST
pub struct SlideExtractor {
    config: ExtractorConfig,
    current_slide: Option<Slide>,
    slide_number: u32,
    pending_notes: Option<String>,
}

impl SlideExtractor {
    /// Create a new extractor with default configuration
    pub fn new() -> Self {
        Self::with_config(ExtractorConfig::default())
    }

    /// Create an extractor with custom configuration
    pub fn with_config(config: ExtractorConfig) -> Self {
        Self {
            config,
            current_slide: None,
            slide_number: 0,
            pending_notes: None,
        }
    }

    /// Extract slides from a document
    pub fn extract(doc: &Document) -> Deck {
        Self::extract_with_config(doc, ExtractorConfig::with_title_slide())
    }

    /// Extract slides with custom configuration
    pub fn extract_with_config(doc: &Document, config: ExtractorConfig) -> Deck {
        let mut extractor = Self::with_config(config);
        extractor.process_document(doc)
    }

    /// Process the entire document
    fn process_document(&mut self, doc: &Document) -> Deck {
        let mut deck = Deck::new();

        // Set deck metadata
        deck.title = doc.metadata.title.clone();
        deck.authors = doc.metadata.authors.clone();
        if let Some(desc) = doc.metadata.attributes.get("description") {
            deck.subtitle = Some(desc.clone());
        }

        // Check for explicit mode: any [slides] blocks?
        let has_slides_blocks = Self::has_slides_blocks(&doc.blocks);

        if has_slides_blocks {
            // Explicit mode: only process [slides] blocks
            self.process_explicit_mode(doc, &mut deck);
        } else {
            // Implicit mode: map headers to slides
            self.process_implicit_mode(doc, &mut deck);
        }

        // Flush any remaining slide
        if let Some(slide) = self.current_slide.take() {
            deck.push(slide);
        }

        // Renumber slides
        for (i, slide) in deck.slides.iter_mut().enumerate() {
            slide.number = (i + 1) as u32;
        }

        deck
    }

    /// Check if any blocks are [slides] blocks
    fn has_slides_blocks(blocks: &[Block]) -> bool {
        for block in blocks {
            match block {
                Block::Open(open) if open.is_slides() => return true,
                Block::Open(open) => {
                    if Self::has_slides_blocks(&open.blocks) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Process document in explicit mode (only [slides] blocks)
    fn process_explicit_mode(&mut self, doc: &Document, deck: &mut Deck) {
        // Create title slide if configured and title exists
        if self.config.include_title_slide {
            if let Some(title) = &doc.metadata.title {
                let subtitle = doc
                    .metadata
                    .attributes
                    .get("description")
                    .or_else(|| doc.metadata.attributes.get("subtitle"))
                    .cloned();

                let mut title_slide = Slide::title_slide(1, title.clone(), subtitle);
                title_slide.layout_hint = SlideLayoutHint::Title;
                deck.push(title_slide);
            }
        }

        // Extract only [slides] blocks
        self.extract_slides_blocks(&doc.blocks, deck);
    }

    /// Extract content from [slides] blocks
    fn extract_slides_blocks(&mut self, blocks: &[Block], deck: &mut Deck) {
        for block in blocks {
            if let Block::Open(open) = block {
                if open.is_slides() {
                    // Process the content inside [slides] block
                    self.process_blocks_as_slides(&open.blocks, deck);
                } else {
                    // Recurse into nested open blocks
                    self.extract_slides_blocks(&open.blocks, deck);
                }
            }
        }
    }

    /// Process document in implicit mode (map headers to slides)
    fn process_implicit_mode(&mut self, doc: &Document, deck: &mut Deck) {
        // Create title slide from document title
        if self.config.include_title_slide {
            if let Some(title) = &doc.metadata.title {
                let subtitle = doc
                    .metadata
                    .attributes
                    .get("description")
                    .or_else(|| doc.metadata.attributes.get("subtitle"))
                    .cloned();

                let mut title_slide = Slide::title_slide(1, title.clone(), subtitle);
                title_slide.layout_hint = SlideLayoutHint::Title;
                deck.push(title_slide);
            }
        }

        // Process all blocks
        self.process_blocks_as_slides(&doc.blocks, deck);
    }

    /// Process blocks as slides (shared between explicit and implicit modes)
    fn process_blocks_as_slides(&mut self, blocks: &[Block], deck: &mut Deck) {
        for block in blocks {
            match block {
                Block::Heading(heading) => {
                    self.handle_heading(heading, deck);
                }
                Block::Paragraph(para) => {
                    self.handle_paragraph(para);
                }
                Block::List(list) => {
                    self.handle_list(list);
                }
                Block::Literal(literal) => {
                    self.handle_code_block(literal);
                }
                Block::Sidebar(sidebar) => {
                    self.handle_sidebar(sidebar);
                }
                Block::ThematicBreak => {
                    // `---` creates a slide break
                    self.flush_current_slide(deck);
                }
                Block::Open(open) if !open.is_slides() => {
                    // Recurse into non-slides open blocks
                    self.process_blocks_as_slides(&open.blocks, deck);
                }
                Block::Admonition(admonition) => {
                    self.handle_admonition(admonition);
                }
                _ => {
                    // Other blocks: ignore for now
                }
            }
        }
    }

    /// Handle a heading block
    fn handle_heading(&mut self, heading: &Heading, deck: &mut Deck) {
        match heading.level {
            1 => {
                // Level 1 heading: start a new title slide (or section)
                self.flush_current_slide(deck);
                self.slide_number += 1;

                let title = inlines_to_text(&heading.text);
                let mut slide = Slide::title_slide(self.slide_number, title, None);
                slide.layout_hint = SlideLayoutHint::Section;
                self.current_slide = Some(slide);
            }
            2 => {
                // Level 2 heading: new content slide
                self.flush_current_slide(deck);
                self.slide_number += 1;

                let title = inlines_to_text(&heading.text);
                let slide = Slide::content_slide(self.slide_number, title);
                self.current_slide = Some(slide);
            }
            3 => {
                // Level 3 heading: could be a vertical slide or subtitle
                if let Some(ref mut slide) = self.current_slide {
                    // Add as subtitle content
                    let text = inlines_to_text(&heading.text);
                    slide.subtitle = Some(text);
                } else {
                    // No current slide, create a section slide
                    self.flush_current_slide(deck);
                    self.slide_number += 1;

                    let title = inlines_to_text(&heading.text);
                    let mut slide = Slide::content_slide(self.slide_number, title);
                    slide.layout_hint = SlideLayoutHint::Section;
                    self.current_slide = Some(slide);
                }
            }
            _ => {
                // Deeper headings: add as content
                if let Some(ref mut slide) = self.current_slide {
                    let text = inlines_to_text(&heading.text);
                    slide
                        .content
                        .push(SlideContent::Paragraph(TextContent::plain(text)));
                }
            }
        }
    }

    /// Handle a paragraph block
    fn handle_paragraph(&mut self, para: &utf8dok_ast::Paragraph) {
        let content = inlines_to_text_content(&para.inlines);

        if let Some(ref mut slide) = self.current_slide {
            slide.content.push(SlideContent::Paragraph(content));
        }
        // If no current slide, paragraph is discarded (before first heading)
    }

    /// Handle a list block
    fn handle_list(&mut self, list: &List) {
        // Convert items first (before borrowing current_slide mutably)
        let items: Vec<SlideListItem> = list.items.iter().flat_map(convert_list_item).collect();

        if let Some(ref mut slide) = self.current_slide {
            let list_content = ListContent { items };

            match list.list_type {
                ListType::Ordered => {
                    slide.content.push(SlideContent::NumberedList(list_content));
                }
                ListType::Unordered | ListType::Description => {
                    slide.content.push(SlideContent::BulletList(list_content));
                }
            }
        }
    }

    /// Handle a code/literal block
    fn handle_code_block(&mut self, literal: &utf8dok_ast::LiteralBlock) {
        if let Some(ref mut slide) = self.current_slide {
            let code = CodeContent::new(&literal.content);
            let code = if let Some(lang) = &literal.language {
                code.with_language(lang)
            } else {
                code
            };
            slide.content.push(SlideContent::Code(code));
        }
    }

    /// Handle a sidebar block (potential speaker notes)
    fn handle_sidebar(&mut self, sidebar: &Sidebar) {
        // Check if this is a notes block
        if sidebar.is_notes() || sidebar.title.as_deref() == Some(".Notes") {
            let notes_text = sidebar.as_text();
            if let Some(ref mut slide) = self.current_slide {
                // Attach notes to current slide
                slide.notes = Some(SpeakerNotes::from_text(notes_text));
            } else {
                // Store for next slide
                self.pending_notes = Some(notes_text);
            }
        } else {
            // Non-notes sidebar: treat as content
            for block in &sidebar.blocks {
                if let Block::Paragraph(para) = block {
                    self.handle_paragraph(para);
                }
            }
        }
    }

    /// Handle an admonition block
    fn handle_admonition(&mut self, admonition: &utf8dok_ast::Admonition) {
        if let Some(ref mut slide) = self.current_slide {
            // Convert admonition content to text
            let text: String = admonition
                .content
                .iter()
                .filter_map(|b| {
                    if let Block::Paragraph(p) = b {
                        Some(inlines_to_text(&p.inlines))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let admon_type = match admonition.admonition_type {
                utf8dok_ast::AdmonitionType::Note => crate::slide::AdmonitionType::Note,
                utf8dok_ast::AdmonitionType::Tip => crate::slide::AdmonitionType::Tip,
                utf8dok_ast::AdmonitionType::Important => crate::slide::AdmonitionType::Important,
                utf8dok_ast::AdmonitionType::Warning => crate::slide::AdmonitionType::Warning,
                utf8dok_ast::AdmonitionType::Caution => crate::slide::AdmonitionType::Caution,
            };

            slide
                .content
                .push(SlideContent::Admonition(crate::slide::AdmonitionContent {
                    admonition_type: admon_type,
                    title: admonition.title.as_ref().map(|t| inlines_to_text(t)),
                    content: TextContent::plain(text),
                }));
        }
    }

    /// Flush current slide to deck
    fn flush_current_slide(&mut self, deck: &mut Deck) {
        if let Some(mut slide) = self.current_slide.take() {
            // Apply pending notes if any
            if slide.notes.is_none() {
                if let Some(notes) = self.pending_notes.take() {
                    slide.notes = Some(SpeakerNotes::from_text(notes));
                }
            }
            deck.push(slide);
        }
    }
}

impl Default for SlideExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert inline elements to plain text
fn inlines_to_text(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => result.push_str(text),
            Inline::Format(_, inner) => {
                if let Inline::Text(text) = inner.as_ref() {
                    result.push_str(text);
                }
            }
            Inline::Span(children) => result.push_str(&inlines_to_text(children)),
            Inline::Link(link) => result.push_str(&inlines_to_text(&link.text)),
            Inline::Break => result.push(' '),
            _ => {}
        }
    }
    result
}

/// Convert inline elements to TextContent with formatting
fn inlines_to_text_content(inlines: &[Inline]) -> TextContent {
    let runs: Vec<TextRun> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text(text) => Some(TextRun::plain(text)),
            Inline::Format(format_type, inner) => {
                if let Inline::Text(text) = inner.as_ref() {
                    Some(match format_type {
                        utf8dok_ast::FormatType::Bold => TextRun::bold(text),
                        utf8dok_ast::FormatType::Italic => TextRun::italic(text),
                        utf8dok_ast::FormatType::Monospace => TextRun::monospace(text),
                        _ => TextRun::plain(text),
                    })
                } else {
                    None
                }
            }
            Inline::Link(link) => {
                let text = inlines_to_text(&link.text);
                Some(TextRun::link(text, &link.url))
            }
            _ => None,
        })
        .collect();

    TextContent::from_runs(runs)
}

/// Convert AST list item to slide list items (handles nesting)
fn convert_list_item(item: &ListItem) -> Vec<SlideListItem> {
    let mut result = Vec::new();

    // Get text from the first paragraph in content
    let text = item
        .content
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(inlines_to_text(&p.inlines))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    result.push(SlideListItem::at_level(text, item.level as u32));

    // Handle nested lists
    for block in &item.content {
        if let Block::List(nested) = block {
            for nested_item in &nested.items {
                result.extend(convert_list_item(nested_item));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use utf8dok_ast::{Document, DocumentMeta, OpenBlock, Paragraph};

    /// Helper to create a simple document with a title
    fn doc_with_title(title: &str) -> Document {
        Document {
            metadata: DocumentMeta {
                title: Some(title.to_string()),
                ..Default::default()
            },
            blocks: Vec::new(),
            intent: None,
        }
    }

    /// Helper to create a heading block
    fn heading(level: u8, text: &str) -> Block {
        Block::Heading(Heading {
            level,
            text: vec![Inline::Text(text.to_string())],
            style_id: None,
            anchor: None,
        })
    }

    /// Helper to create a paragraph block
    fn para(text: &str) -> Block {
        Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text(text.to_string())],
            style_id: None,
            attributes: HashMap::new(),
        })
    }

    /// Helper to create a bullet list
    fn bullet_list(items: &[&str]) -> Block {
        Block::List(List {
            list_type: ListType::Unordered,
            items: items
                .iter()
                .map(|text| ListItem {
                    content: vec![para(text)],
                    level: 0,
                    term: None,
                })
                .collect(),
            style_id: None,
        })
    }

    /// Helper to create a [slides] block
    fn slides_block(blocks: Vec<Block>) -> Block {
        Block::Open(OpenBlock {
            role: Some("slides".to_string()),
            title: None,
            blocks,
            attributes: HashMap::new(),
        })
    }

    /// Helper to create a sidebar/notes block
    fn notes_block(text: &str) -> Block {
        Block::Sidebar(Sidebar {
            title: Some(".Notes".to_string()),
            blocks: vec![para(text)],
        })
    }

    // =========================================================================
    // Test 1: Implicit Mode (Header Mapping)
    // =========================================================================

    #[test]
    fn test_implicit_slide_extraction() {
        let mut doc = doc_with_title("Title");
        doc.blocks.push(heading(2, "Slide 1"));
        doc.blocks.push(bullet_list(&["Bullet 1", "Bullet 2"]));

        let deck = SlideExtractor::extract(&doc);

        // Title slide + Slide 1
        assert_eq!(deck.slides.len(), 2);
        assert_eq!(deck.slides[0].title.as_deref(), Some("Title"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("Slide 1"));
    }

    #[test]
    fn test_implicit_mode_multiple_slides() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Introduction"));
        doc.blocks.push(para("Welcome everyone"));
        doc.blocks.push(heading(2, "Topics"));
        doc.blocks.push(bullet_list(&["Topic A", "Topic B"]));
        doc.blocks.push(heading(2, "Conclusion"));
        doc.blocks.push(para("Thank you"));

        let deck = SlideExtractor::extract(&doc);

        // Title + 3 content slides
        assert_eq!(deck.slides.len(), 4);
        assert_eq!(deck.slides[0].title.as_deref(), Some("Presentation"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("Introduction"));
        assert_eq!(deck.slides[2].title.as_deref(), Some("Topics"));
        assert_eq!(deck.slides[3].title.as_deref(), Some("Conclusion"));
    }

    #[test]
    fn test_thematic_break_creates_slide() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Slide 1"));
        doc.blocks.push(para("Content"));
        doc.blocks.push(Block::ThematicBreak); // ---
        doc.blocks.push(para("More content after break"));
        doc.blocks.push(heading(2, "Slide 2"));

        let deck = SlideExtractor::extract(&doc);

        // The thematic break flushes Slide 1, then "More content" has no slide
        // Then Slide 2 starts
        assert_eq!(deck.slides.len(), 3); // Title + Slide 1 + Slide 2
    }

    // =========================================================================
    // Test 2: Explicit Mode ([slides] Block)
    // =========================================================================

    #[test]
    fn test_explicit_slide_block() {
        let mut doc = doc_with_title("Doc Title");
        doc.blocks.push(para("This is prose for the document."));
        doc.blocks.push(slides_block(vec![
            heading(2, "Slide A"),
            bullet_list(&["Point 1", "Point 2"]),
        ]));

        let deck = SlideExtractor::extract(&doc);

        // Title + Slide A (prose is ignored)
        assert_eq!(deck.slides.len(), 2);
        assert_eq!(deck.slides[0].title.as_deref(), Some("Doc Title"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("Slide A"));

        // Verify prose is NOT in the slides
        for slide in &deck.slides {
            for content in &slide.content {
                if let SlideContent::Paragraph(tc) = content {
                    let text = tc.as_plain_text();
                    assert!(
                        !text.contains("prose"),
                        "Prose should not be in slides: {}",
                        text
                    );
                }
            }
        }
    }

    #[test]
    fn test_multiple_slides_blocks() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(para("Prose 1"));
        doc.blocks.push(slides_block(vec![
            heading(2, "Section 1"),
            para("Slide content 1"),
        ]));
        doc.blocks.push(para("Prose 2"));
        doc.blocks.push(slides_block(vec![
            heading(2, "Section 2"),
            para("Slide content 2"),
        ]));

        let deck = SlideExtractor::extract(&doc);

        // Title + Section 1 + Section 2
        assert_eq!(deck.slides.len(), 3);
        assert_eq!(deck.slides[1].title.as_deref(), Some("Section 1"));
        assert_eq!(deck.slides[2].title.as_deref(), Some("Section 2"));
    }

    // =========================================================================
    // Test 3: Speaker Notes Extraction
    // =========================================================================

    #[test]
    fn test_speaker_notes() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Slide 1"));
        doc.blocks.push(para("Content."));
        doc.blocks.push(notes_block("Don't forget to smile."));

        let deck = SlideExtractor::extract(&doc);

        assert_eq!(deck.slides.len(), 2);
        let slide = &deck.slides[1];
        assert!(slide.notes.is_some());
        assert_eq!(
            slide.notes.as_ref().unwrap().as_plain_text(),
            "Don't forget to smile."
        );
    }

    #[test]
    fn test_speaker_notes_in_explicit_mode() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(slides_block(vec![
            heading(2, "Important Slide"),
            para("Key point"),
            notes_block("Spend 2 minutes here"),
        ]));

        let deck = SlideExtractor::extract(&doc);

        assert_eq!(deck.slides.len(), 2);
        let slide = &deck.slides[1];
        assert!(slide.notes.is_some());
        assert!(slide
            .notes
            .as_ref()
            .unwrap()
            .as_plain_text()
            .contains("2 minutes"));
    }

    // =========================================================================
    // Test 4: Content Mapping
    // =========================================================================

    #[test]
    fn test_list_content_mapping() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Features"));
        doc.blocks.push(bullet_list(&["Fast", "Reliable", "Easy"]));

        let deck = SlideExtractor::extract(&doc);

        let slide = &deck.slides[1];
        assert_eq!(slide.content.len(), 1);

        if let SlideContent::BulletList(list) = &slide.content[0] {
            assert_eq!(list.items.len(), 3);
            assert_eq!(list.items[0].content.as_plain_text(), "Fast");
            assert_eq!(list.items[1].content.as_plain_text(), "Reliable");
            assert_eq!(list.items[2].content.as_plain_text(), "Easy");
        } else {
            panic!("Expected BulletList");
        }
    }

    #[test]
    fn test_code_block_mapping() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Code Example"));
        doc.blocks.push(Block::Literal(utf8dok_ast::LiteralBlock {
            content: "fn main() {}".to_string(),
            language: Some("rust".to_string()),
            title: None,
            style_id: None,
        }));

        let deck = SlideExtractor::extract(&doc);

        let slide = &deck.slides[1];
        assert_eq!(slide.content.len(), 1);

        if let SlideContent::Code(code) = &slide.content[0] {
            assert_eq!(code.source, "fn main() {}");
            assert_eq!(code.language, Some("rust".to_string()));
        } else {
            panic!("Expected Code");
        }
    }

    // =========================================================================
    // Test 5: Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        let deck = SlideExtractor::extract(&doc);
        assert!(deck.is_empty());
    }

    #[test]
    fn test_no_title_slide_config() {
        let mut doc = doc_with_title("Title");
        doc.blocks.push(heading(2, "Slide 1"));

        let config = ExtractorConfig {
            include_title_slide: false,
            ..Default::default()
        };
        let deck = SlideExtractor::extract_with_config(&doc, config);

        // Only Slide 1, no title slide
        assert_eq!(deck.slides.len(), 1);
        assert_eq!(deck.slides[0].title.as_deref(), Some("Slide 1"));
    }

    #[test]
    fn test_level_1_heading_creates_section() {
        let mut doc = doc_with_title("Presentation");
        doc.blocks.push(heading(2, "Intro"));
        doc.blocks.push(heading(1, "Part 1"));
        doc.blocks.push(heading(2, "Details"));

        let deck = SlideExtractor::extract(&doc);

        // Title + Intro + Part 1 (section) + Details
        assert_eq!(deck.slides.len(), 4);
        assert_eq!(deck.slides[2].title.as_deref(), Some("Part 1"));
        assert_eq!(deck.slides[2].layout_hint, SlideLayoutHint::Section);
    }

    #[test]
    fn test_deck_metadata() {
        let mut doc = Document::new();
        doc.metadata.title = Some("My Presentation".to_string());
        doc.metadata.authors = vec!["Alice".to_string(), "Bob".to_string()];
        doc.metadata
            .attributes
            .insert("description".to_string(), "A great talk".to_string());

        let deck = SlideExtractor::extract(&doc);

        assert_eq!(deck.title, Some("My Presentation".to_string()));
        assert_eq!(deck.authors, vec!["Alice".to_string(), "Bob".to_string()]);
        assert_eq!(deck.subtitle, Some("A great talk".to_string()));
    }

    #[test]
    fn test_slide_numbering() {
        let mut doc = doc_with_title("Title");
        doc.blocks.push(heading(2, "Slide 1"));
        doc.blocks.push(heading(2, "Slide 2"));
        doc.blocks.push(heading(2, "Slide 3"));

        let deck = SlideExtractor::extract(&doc);

        assert_eq!(deck.slides[0].number, 1);
        assert_eq!(deck.slides[1].number, 2);
        assert_eq!(deck.slides[2].number, 3);
        assert_eq!(deck.slides[3].number, 4);
    }
}
