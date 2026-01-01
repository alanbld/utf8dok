//! Slide data structures and content types.
//!
//! This module defines the intermediate representation for slides,
//! used between AsciiDoc parsing and PPTX generation.

use serde::{Deserialize, Serialize};

/// A single slide in a presentation
#[derive(Debug, Clone, Default)]
pub struct Slide {
    /// Slide number (1-based)
    pub number: u32,

    /// Slide title (displayed in title placeholder)
    pub title: Option<String>,

    /// Slide subtitle (for title slides or section headers)
    pub subtitle: Option<String>,

    /// Main slide content
    pub content: Vec<SlideContent>,

    /// Speaker notes for this slide
    pub notes: Option<SpeakerNotes>,

    /// Layout type hint (e.g., "title", "content", "image")
    pub layout_hint: SlideLayoutHint,

    /// Source line number for diagnostics
    pub source_line: Option<usize>,
}

/// Hint for layout selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlideLayoutHint {
    /// Title slide (centered, large text)
    Title,

    /// Section header slide
    Section,

    /// Standard title and content
    #[default]
    Content,

    /// Two-column layout
    TwoColumn,

    /// Comparison layout
    Comparison,

    /// Title only (content below)
    TitleOnly,

    /// Blank slide
    Blank,

    /// Picture-focused layout
    Image,

    /// Quote layout
    Quote,
}

/// Content elements within a slide
#[derive(Debug, Clone)]
pub enum SlideContent {
    /// Plain text paragraph
    Paragraph(TextContent),

    /// Unordered (bullet) list
    BulletList(ListContent),

    /// Ordered (numbered) list
    NumberedList(ListContent),

    /// Embedded image
    Image(ImageContent),

    /// Table
    Table(TableContent),

    /// Code block
    Code(CodeContent),

    /// Block quote
    Quote(QuoteContent),

    /// Admonition (note, tip, warning, etc.)
    Admonition(AdmonitionContent),

    /// Diagram (rendered from Mermaid, PlantUML, etc.)
    Diagram(DiagramContent),
}

/// Text content with optional formatting
#[derive(Debug, Clone, Default)]
pub struct TextContent {
    /// Text runs with inline formatting
    pub runs: Vec<TextRun>,
}

/// A run of text with consistent formatting
#[derive(Debug, Clone)]
pub struct TextRun {
    /// The text content
    pub text: String,

    /// Bold formatting
    pub bold: bool,

    /// Italic formatting
    pub italic: bool,

    /// Monospace (code) formatting
    pub monospace: bool,

    /// Hyperlink URL (if this is a link)
    pub link: Option<String>,
}

impl TextRun {
    /// Create a plain text run
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            monospace: false,
            link: None,
        }
    }

    /// Create a bold text run
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: true,
            italic: false,
            monospace: false,
            link: None,
        }
    }

    /// Create an italic text run
    pub fn italic(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: true,
            monospace: false,
            link: None,
        }
    }

    /// Create a monospace text run
    pub fn monospace(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            monospace: true,
            link: None,
        }
    }

    /// Create a hyperlink text run
    pub fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            monospace: false,
            link: Some(url.into()),
        }
    }
}

impl TextContent {
    /// Create text content from a single plain string
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            runs: vec![TextRun::plain(text)],
        }
    }

    /// Create text content from multiple runs
    pub fn from_runs(runs: Vec<TextRun>) -> Self {
        Self { runs }
    }

    /// Get the plain text without formatting
    pub fn as_plain_text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
}

/// List content with items
#[derive(Debug, Clone, Default)]
pub struct ListContent {
    /// List items
    pub items: Vec<ListItem>,
}

/// A single list item (can be nested)
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Item text content
    pub content: TextContent,

    /// Nesting level (0 = top level)
    pub level: u32,

    /// Nested items (for hierarchical lists)
    pub children: Vec<ListItem>,
}

impl ListItem {
    /// Create a simple list item
    pub fn simple(text: impl Into<String>) -> Self {
        Self {
            content: TextContent::plain(text),
            level: 0,
            children: Vec::new(),
        }
    }

    /// Create a list item at a specific level
    pub fn at_level(text: impl Into<String>, level: u32) -> Self {
        Self {
            content: TextContent::plain(text),
            level,
            children: Vec::new(),
        }
    }
}

/// Image content
#[derive(Debug, Clone)]
pub struct ImageContent {
    /// Path to image file
    pub path: String,

    /// Alt text / caption
    pub alt: Option<String>,

    /// Width specification (e.g., "50%", "400px")
    pub width: Option<String>,

    /// Height specification
    pub height: Option<String>,

    /// Image should fill the slide
    pub fill_slide: bool,
}

/// Table content
#[derive(Debug, Clone, Default)]
pub struct TableContent {
    /// Table caption/title
    pub caption: Option<String>,

    /// Header row
    pub header: Option<Vec<TextContent>>,

    /// Body rows
    pub rows: Vec<Vec<TextContent>>,

    /// Column widths (relative or absolute)
    pub col_widths: Vec<String>,
}

/// Code block content
#[derive(Debug, Clone)]
pub struct CodeContent {
    /// Source code text
    pub source: String,

    /// Language for syntax highlighting
    pub language: Option<String>,

    /// Title/caption
    pub title: Option<String>,

    /// Show line numbers
    pub line_numbers: bool,

    /// Starting line number (if not 1)
    pub start_line: u32,

    /// Highlighted lines
    pub highlight_lines: Vec<u32>,
}

impl CodeContent {
    /// Create a simple code block
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            language: None,
            title: None,
            line_numbers: false,
            start_line: 1,
            highlight_lines: Vec::new(),
        }
    }

    /// Set the language for syntax highlighting
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}

/// Block quote content
#[derive(Debug, Clone)]
pub struct QuoteContent {
    /// Quote text
    pub text: TextContent,

    /// Attribution (author)
    pub attribution: Option<String>,

    /// Citation source
    pub citation: Option<String>,
}

/// Admonition content (note, tip, warning, etc.)
#[derive(Debug, Clone)]
pub struct AdmonitionContent {
    /// Admonition type
    pub admonition_type: AdmonitionType,

    /// Title (optional override)
    pub title: Option<String>,

    /// Content
    pub content: TextContent,
}

/// Types of admonitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmonitionType {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AdmonitionType {
    /// Get the default title for this admonition type
    pub fn default_title(&self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Important => "Important",
            Self::Warning => "Warning",
            Self::Caution => "Caution",
        }
    }

    /// Get a suggested color for this admonition type (hex RGB)
    pub fn suggested_color(&self) -> &'static str {
        match self {
            Self::Note => "3B82F6",     // Blue
            Self::Tip => "22C55E",      // Green
            Self::Important => "8B5CF6", // Purple
            Self::Warning => "F59E0B",   // Amber
            Self::Caution => "EF4444",   // Red
        }
    }
}

/// Diagram content (rendered from text-based diagram tools)
#[derive(Debug, Clone)]
pub struct DiagramContent {
    /// Diagram source (Mermaid, PlantUML, etc.)
    pub source: String,

    /// Diagram type
    pub diagram_type: DiagramType,

    /// Caption
    pub caption: Option<String>,

    /// Rendered image path (after processing)
    pub rendered_path: Option<String>,
}

/// Types of diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    Mermaid,
    PlantUML,
    Ditaa,
    Graphviz,
    Other,
}

/// Speaker notes for a slide
#[derive(Debug, Clone, Default)]
pub struct SpeakerNotes {
    /// Note content
    pub content: Vec<TextContent>,
}

impl SpeakerNotes {
    /// Create speaker notes from plain text
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            content: vec![TextContent::plain(text)],
        }
    }

    /// Create speaker notes from multiple paragraphs
    pub fn from_paragraphs(paragraphs: Vec<String>) -> Self {
        Self {
            content: paragraphs.into_iter().map(TextContent::plain).collect(),
        }
    }

    /// Get all notes as plain text
    pub fn as_plain_text(&self) -> String {
        self.content
            .iter()
            .map(|c| c.as_plain_text())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl Slide {
    /// Create a new empty slide
    pub fn new(number: u32) -> Self {
        Self {
            number,
            ..Default::default()
        }
    }

    /// Create a title slide
    pub fn title_slide(number: u32, title: impl Into<String>, subtitle: Option<String>) -> Self {
        Self {
            number,
            title: Some(title.into()),
            subtitle,
            layout_hint: SlideLayoutHint::Title,
            ..Default::default()
        }
    }

    /// Create a content slide
    pub fn content_slide(number: u32, title: impl Into<String>) -> Self {
        Self {
            number,
            title: Some(title.into()),
            layout_hint: SlideLayoutHint::Content,
            ..Default::default()
        }
    }

    /// Add content to the slide
    pub fn with_content(mut self, content: SlideContent) -> Self {
        self.content.push(content);
        self
    }

    /// Add speaker notes
    pub fn with_notes(mut self, notes: SpeakerNotes) -> Self {
        self.notes = Some(notes);
        self
    }

    /// Set the layout hint
    pub fn with_layout(mut self, hint: SlideLayoutHint) -> Self {
        self.layout_hint = hint;
        self
    }

    /// Check if this is a title slide
    pub fn is_title_slide(&self) -> bool {
        self.layout_hint == SlideLayoutHint::Title
    }

    /// Check if this slide has speaker notes
    pub fn has_notes(&self) -> bool {
        self.notes.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_title_slide() {
        let slide = Slide::title_slide(1, "Welcome", Some("Introduction to utf8dok".to_string()));

        assert_eq!(slide.number, 1);
        assert_eq!(slide.title, Some("Welcome".to_string()));
        assert_eq!(
            slide.subtitle,
            Some("Introduction to utf8dok".to_string())
        );
        assert!(slide.is_title_slide());
    }

    #[test]
    fn test_create_content_slide() {
        let slide = Slide::content_slide(2, "Overview")
            .with_content(SlideContent::Paragraph(TextContent::plain("First point")))
            .with_notes(SpeakerNotes::from_text("Remember to mention X"));

        assert_eq!(slide.number, 2);
        assert_eq!(slide.title, Some("Overview".to_string()));
        assert!(!slide.is_title_slide());
        assert!(slide.has_notes());
        assert_eq!(slide.content.len(), 1);
    }

    #[test]
    fn test_text_runs() {
        let run = TextRun::bold("important");
        assert!(run.bold);
        assert!(!run.italic);

        let run = TextRun::link("click here", "https://example.com");
        assert_eq!(run.link, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_text_content() {
        let content = TextContent::from_runs(vec![
            TextRun::plain("Hello "),
            TextRun::bold("world"),
            TextRun::plain("!"),
        ]);

        assert_eq!(content.as_plain_text(), "Hello world!");
    }

    #[test]
    fn test_list_items() {
        let item = ListItem::at_level("Nested item", 1);
        assert_eq!(item.level, 1);
        assert_eq!(item.content.as_plain_text(), "Nested item");
    }

    #[test]
    fn test_code_content() {
        let code = CodeContent::new("fn main() {}")
            .with_language("rust");

        assert_eq!(code.source, "fn main() {}");
        assert_eq!(code.language, Some("rust".to_string()));
    }

    #[test]
    fn test_speaker_notes() {
        let notes = SpeakerNotes::from_paragraphs(vec![
            "First paragraph".to_string(),
            "Second paragraph".to_string(),
        ]);

        assert_eq!(notes.content.len(), 2);
        assert!(notes.as_plain_text().contains("First paragraph"));
        assert!(notes.as_plain_text().contains("Second paragraph"));
    }

    #[test]
    fn test_admonition_types() {
        assert_eq!(AdmonitionType::Note.default_title(), "Note");
        assert_eq!(AdmonitionType::Warning.default_title(), "Warning");

        // Colors should be valid hex
        let color = AdmonitionType::Tip.suggested_color();
        assert_eq!(color.len(), 6);
    }

    #[test]
    fn test_slide_layout_hints() {
        let slide = Slide::new(1).with_layout(SlideLayoutHint::Quote);
        assert_eq!(slide.layout_hint, SlideLayoutHint::Quote);

        // Default is Content
        let slide = Slide::new(2);
        assert_eq!(slide.layout_hint, SlideLayoutHint::Content);
    }
}
