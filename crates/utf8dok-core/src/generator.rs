//! AsciiDoc Generator
//!
//! This module converts a `utf8dok_ast::Document` into AsciiDoc text format.
//!
//! # Example
//!
//! ```
//! use utf8dok_ast::{Document, Block, Heading, Inline};
//! use utf8dok_core::generate;
//!
//! let mut doc = Document::new();
//! doc.push(Block::Heading(Heading {
//!     level: 1,
//!     text: vec![Inline::Text("My Title".to_string())],
//!     style_id: None,
//!     anchor: None,
//! }));
//!
//! let asciidoc = generate(&doc);
//! assert!(asciidoc.contains("= My Title"));
//! ```

use std::fmt::Write;

use utf8dok_ast::{
    Admonition, AdmonitionType, Block, BreakType, Document, FormatType, Heading, Inline, List,
    ListItem, ListType, LiteralBlock, Paragraph, Table,
};

/// AsciiDoc generator configuration
#[derive(Debug, Clone, Default)]
pub struct GeneratorConfig {
    /// Whether to include document header attributes
    pub include_header: bool,
    /// Whether to generate anchors for headings
    pub generate_anchors: bool,
}

/// AsciiDoc generator
pub struct AsciiDocGenerator {
    config: GeneratorConfig,
    output: String,
}

impl AsciiDocGenerator {
    /// Create a new generator with default configuration
    pub fn new() -> Self {
        Self {
            config: GeneratorConfig::default(),
            output: String::new(),
        }
    }

    /// Create a generator with custom configuration
    pub fn with_config(config: GeneratorConfig) -> Self {
        Self {
            config,
            output: String::new(),
        }
    }

    /// Generate AsciiDoc from a document
    pub fn generate(&mut self, doc: &Document) -> String {
        self.output.clear();

        // Generate document header if title exists
        if self.config.include_header {
            if let Some(ref title) = doc.metadata.title {
                writeln!(self.output, "= {}", title).unwrap();

                // Add authors
                if !doc.metadata.authors.is_empty() {
                    writeln!(self.output, "{}", doc.metadata.authors.join("; ")).unwrap();
                }

                // Add revision
                if let Some(ref rev) = doc.metadata.revision {
                    writeln!(self.output, "v{}", rev).unwrap();
                }

                // Add attributes
                for (key, value) in &doc.metadata.attributes {
                    writeln!(self.output, ":{}: {}", key, value).unwrap();
                }

                writeln!(self.output).unwrap();
            }
        }

        // Generate blocks
        for (i, block) in doc.blocks.iter().enumerate() {
            if i > 0 {
                // Add blank line between blocks
                writeln!(self.output).unwrap();
            }
            self.generate_block(block);
        }

        self.output.trim_end().to_string()
    }

    /// Generate a single block
    fn generate_block(&mut self, block: &Block) {
        match block {
            Block::Heading(h) => self.generate_heading(h),
            Block::Paragraph(p) => self.generate_paragraph(p),
            Block::List(l) => self.generate_list(l),
            Block::Table(t) => self.generate_table(t),
            Block::Admonition(a) => self.generate_admonition(a),
            Block::Literal(l) => self.generate_literal(l),
            Block::Break(b) => self.generate_break(b),
            Block::Open(open) => self.generate_open_block(open),
            Block::Sidebar(sidebar) => self.generate_sidebar(sidebar),
            Block::Quote(quote) => self.generate_quote(quote),
            Block::ThematicBreak => self.generate_thematic_break(),
        }
    }

    /// Generate a heading
    fn generate_heading(&mut self, heading: &Heading) {
        // Optional anchor
        if self.config.generate_anchors {
            if let Some(ref anchor) = heading.anchor {
                writeln!(self.output, "[[{}]]", anchor).unwrap();
            }
        }

        // Heading prefix: == for level 1, === for level 2, etc.
        // AsciiDoc uses = for doc title (level 0), == for section level 1, etc.
        // So we add 1 to the level to get the correct number of = signs.
        let prefix = "=".repeat(heading.level as usize + 1);
        write!(self.output, "{} ", prefix).unwrap();

        // Generate heading text
        for inline in &heading.text {
            self.generate_inline(inline);
        }
        writeln!(self.output).unwrap();
    }

    /// Generate a paragraph
    fn generate_paragraph(&mut self, para: &Paragraph) {
        for inline in &para.inlines {
            self.generate_inline(inline);
        }
        writeln!(self.output).unwrap();
    }

    /// Generate inline content
    fn generate_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text(text) => {
                // TODO: Escape special AsciiDoc characters if necessary
                // For now, write text as-is
                write!(self.output, "{}", text).unwrap();
            }
            Inline::Format(format_type, inner) => {
                let (open, close) = match format_type {
                    FormatType::Bold => ("*", "*"),
                    FormatType::Italic => ("_", "_"),
                    FormatType::Monospace => ("`", "`"),
                    FormatType::Highlight => ("#", "#"),
                    FormatType::Superscript => ("^", "^"),
                    FormatType::Subscript => ("~", "~"),
                };
                write!(self.output, "{}", open).unwrap();
                self.generate_inline(inner);
                write!(self.output, "{}", close).unwrap();
            }
            Inline::Span(inlines) => {
                for inner in inlines {
                    self.generate_inline(inner);
                }
            }
            Inline::Link(link) => {
                if link.url.starts_with('#') {
                    // Internal cross-reference: <<anchor,text>>
                    let anchor = link.url.trim_start_matches('#');
                    write!(self.output, "<<{},", anchor).unwrap();
                    for inner in &link.text {
                        self.generate_inline(inner);
                    }
                    write!(self.output, ">>").unwrap();
                } else {
                    // External link: url[text]
                    write!(self.output, "{}[", link.url).unwrap();
                    for inner in &link.text {
                        self.generate_inline(inner);
                    }
                    write!(self.output, "]").unwrap();
                }
            }
            Inline::Image(image) => {
                write!(self.output, "image::{}[", image.src).unwrap();
                if let Some(ref alt) = image.alt {
                    write!(self.output, "{}", alt).unwrap();
                }
                write!(self.output, "]").unwrap();
            }
            Inline::Break => {
                writeln!(self.output, " +").unwrap();
            }
            Inline::Anchor(name) => {
                // Generate inline anchor: [[name]]
                write!(self.output, "[[{}]]", name).unwrap();
            }
        }
    }

    /// Generate a list
    fn generate_list(&mut self, list: &List) {
        for item in &list.items {
            self.generate_list_item(item, &list.list_type);
        }
    }

    /// Generate a list item
    fn generate_list_item(&mut self, item: &ListItem, list_type: &ListType) {
        // Generate marker based on type and level
        let marker = match list_type {
            ListType::Unordered => "*".repeat(item.level as usize + 1),
            ListType::Ordered => ".".repeat(item.level as usize + 1),
            ListType::Description => {
                // For description lists, handle term separately
                if let Some(ref term) = item.term {
                    for inline in term {
                        self.generate_inline(inline);
                    }
                    write!(self.output, ":: ").unwrap();
                }
                String::new()
            }
        };

        if !marker.is_empty() {
            write!(self.output, "{} ", marker).unwrap();
        }

        // Generate item content
        for (i, block) in item.content.iter().enumerate() {
            if i > 0 {
                // Continuation for multi-block list items
                writeln!(self.output, "+").unwrap();
            }
            match block {
                Block::Paragraph(p) => {
                    for inline in &p.inlines {
                        self.generate_inline(inline);
                    }
                    writeln!(self.output).unwrap();
                }
                _ => {
                    self.generate_block(block);
                }
            }
        }
    }

    /// Generate a table
    fn generate_table(&mut self, table: &Table) {
        // Determine column count
        let col_count = table.rows.first().map(|r| r.cells.len()).unwrap_or(0);
        if col_count == 0 {
            return;
        }

        // Table options
        if let Some(ref caption) = table.caption {
            write!(self.output, ".").unwrap();
            for inline in caption {
                self.generate_inline(inline);
            }
            writeln!(self.output).unwrap();
        }

        // Check if first row is header
        let has_header = table.rows.first().map(|r| r.is_header).unwrap_or(false);
        if has_header {
            writeln!(self.output, "[%header]").unwrap();
        }

        writeln!(self.output, "|===").unwrap();

        for row in &table.rows {
            for cell in &row.cells {
                write!(self.output, "| ").unwrap();
                for block in &cell.content {
                    if let Block::Paragraph(p) = block {
                        for inline in &p.inlines {
                            self.generate_inline(inline);
                        }
                    }
                }
                writeln!(self.output).unwrap();
            }
            writeln!(self.output).unwrap();
        }

        writeln!(self.output, "|===").unwrap();
    }

    /// Generate an admonition
    fn generate_admonition(&mut self, admonition: &Admonition) {
        let label = match admonition.admonition_type {
            AdmonitionType::Note => "NOTE",
            AdmonitionType::Tip => "TIP",
            AdmonitionType::Important => "IMPORTANT",
            AdmonitionType::Warning => "WARNING",
            AdmonitionType::Caution => "CAUTION",
        };

        // Optional title
        if let Some(ref title) = admonition.title {
            write!(self.output, ".").unwrap();
            for inline in title {
                self.generate_inline(inline);
            }
            writeln!(self.output).unwrap();
        }

        writeln!(self.output, "[{}]", label).unwrap();
        writeln!(self.output, "====").unwrap();

        for block in &admonition.content {
            self.generate_block(block);
        }

        writeln!(self.output, "====").unwrap();
    }

    /// Generate a literal/code block
    fn generate_literal(&mut self, literal: &LiteralBlock) {
        // Optional title
        if let Some(ref title) = literal.title {
            writeln!(self.output, ".{}", title).unwrap();
        }

        // Block attribute: [style_id] or [source,language]
        if let Some(ref style) = literal.style_id {
            // Diagram or custom style: [mermaid], [plantuml], etc.
            writeln!(self.output, "[{}]", style).unwrap();
        } else if let Some(ref lang) = literal.language {
            // Source code block: [source,rust]
            writeln!(self.output, "[source,{}]", lang).unwrap();
        }

        writeln!(self.output, "----").unwrap();
        write!(self.output, "{}", literal.content).unwrap();
        if !literal.content.ends_with('\n') {
            writeln!(self.output).unwrap();
        }
        writeln!(self.output, "----").unwrap();
    }

    /// Generate a break
    fn generate_break(&mut self, break_type: &BreakType) {
        match break_type {
            BreakType::Page => writeln!(self.output, "<<<").unwrap(),
            BreakType::Section => writeln!(self.output, "'''").unwrap(),
        }
    }

    /// Generate an open block
    fn generate_open_block(&mut self, open: &utf8dok_ast::OpenBlock) {
        // Role attribute
        if let Some(ref role) = open.role {
            writeln!(self.output, "[{}]", role).unwrap();
        }

        // Optional title
        if let Some(ref title) = open.title {
            writeln!(self.output, ".{}", title).unwrap();
        }

        writeln!(self.output, "--").unwrap();
        for block in &open.blocks {
            self.generate_block(block);
        }
        writeln!(self.output, "--").unwrap();
    }

    /// Generate a sidebar block
    fn generate_sidebar(&mut self, sidebar: &utf8dok_ast::Sidebar) {
        // Optional title (e.g., .Notes for speaker notes)
        if let Some(ref title) = sidebar.title {
            writeln!(self.output, ".{}", title).unwrap();
        }

        writeln!(self.output, "****").unwrap();
        for block in &sidebar.blocks {
            self.generate_block(block);
        }
        writeln!(self.output, "****").unwrap();
    }

    /// Generate a quote block
    fn generate_quote(&mut self, quote: &utf8dok_ast::QuoteBlock) {
        write!(self.output, "[quote").unwrap();
        if let Some(ref attribution) = quote.attribution {
            write!(self.output, ", {}", attribution).unwrap();
        }
        if let Some(ref cite) = quote.cite {
            write!(self.output, ", {}", cite).unwrap();
        }
        writeln!(self.output, "]").unwrap();

        writeln!(self.output, "____").unwrap();
        for block in &quote.blocks {
            self.generate_block(block);
        }
        writeln!(self.output, "____").unwrap();
    }

    /// Generate a thematic break
    fn generate_thematic_break(&mut self) {
        writeln!(self.output, "'''").unwrap();
    }
}

impl Default for AsciiDocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to generate AsciiDoc from a document
pub fn generate(doc: &Document) -> String {
    let mut generator = AsciiDocGenerator::new();
    generator.generate(doc)
}

/// Generate AsciiDoc with custom configuration
pub fn generate_with_config(doc: &Document, config: GeneratorConfig) -> String {
    let mut generator = AsciiDocGenerator::with_config(config);
    generator.generate(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use utf8dok_ast::{DocumentMeta, TableCell, TableRow};

    #[test]
    fn test_simple_heading() {
        let mut doc = Document::new();
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("My Title".to_string())],
            style_id: None,
            anchor: None,
        }));

        let output = generate(&doc);
        // Level 1 heading -> == (level + 1 equals signs)
        assert_eq!(output, "== My Title");
    }

    #[test]
    fn test_heading_levels() {
        let mut doc = Document::new();
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Level 1".to_string())],
            style_id: None,
            anchor: None,
        }));
        doc.push(Block::Heading(Heading {
            level: 2,
            text: vec![Inline::Text("Level 2".to_string())],
            style_id: None,
            anchor: None,
        }));
        doc.push(Block::Heading(Heading {
            level: 3,
            text: vec![Inline::Text("Level 3".to_string())],
            style_id: None,
            anchor: None,
        }));

        let output = generate(&doc);
        // Levels map to: 1->==, 2->===, 3->====
        assert!(output.contains("== Level 1"));
        assert!(output.contains("=== Level 2"));
        assert!(output.contains("==== Level 3"));
    }

    #[test]
    fn test_paragraph() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text("This is a paragraph.".to_string())],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "This is a paragraph.");
    }

    #[test]
    fn test_bold_text() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Text("This is ".to_string()),
                Inline::Format(FormatType::Bold, Box::new(Inline::Text("bold".to_string()))),
                Inline::Text(" text.".to_string()),
            ],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "This is *bold* text.");
    }

    #[test]
    fn test_italic_text() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Format(
                FormatType::Italic,
                Box::new(Inline::Text("italic".to_string())),
            )],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "_italic_");
    }

    #[test]
    fn test_monospace_text() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Format(
                FormatType::Monospace,
                Box::new(Inline::Text("code".to_string())),
            )],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "`code`");
    }

    #[test]
    fn test_nested_formatting() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Format(
                FormatType::Bold,
                Box::new(Inline::Format(
                    FormatType::Italic,
                    Box::new(Inline::Text("bold italic".to_string())),
                )),
            )],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "*_bold italic_*");
    }

    #[test]
    fn test_unordered_list() {
        let mut doc = Document::new();
        doc.push(Block::List(List {
            list_type: ListType::Unordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Item 1".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Item 2".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
            ],
            style_id: None,
        }));

        let output = generate(&doc);
        assert!(output.contains("* Item 1"));
        assert!(output.contains("* Item 2"));
    }

    #[test]
    fn test_ordered_list() {
        let mut doc = Document::new();
        doc.push(Block::List(List {
            list_type: ListType::Ordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("First".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Second".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
            ],
            style_id: None,
        }));

        let output = generate(&doc);
        assert!(output.contains(". First"));
        assert!(output.contains(". Second"));
    }

    #[test]
    fn test_nested_list() {
        let mut doc = Document::new();
        doc.push(Block::List(List {
            list_type: ListType::Unordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Parent".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Child".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 1,
                    term: None,
                },
            ],
            style_id: None,
        }));

        let output = generate(&doc);
        assert!(output.contains("* Parent"));
        assert!(output.contains("** Child"));
    }

    #[test]
    fn test_code_block() {
        let mut doc = Document::new();
        doc.push(Block::Literal(LiteralBlock {
            content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            language: Some("rust".to_string()),
            title: None,
            style_id: None,
        }));

        let output = generate(&doc);
        assert!(output.contains("[source,rust]"));
        assert!(output.contains("----"));
        assert!(output.contains("fn main()"));
    }

    #[test]
    fn test_link() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Link(utf8dok_ast::Link {
                url: "https://example.com".to_string(),
                text: vec![Inline::Text("Example".to_string())],
            })],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "https://example.com[Example]");
    }

    #[test]
    fn test_image() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Image(utf8dok_ast::Image {
                src: "logo.png".to_string(),
                alt: Some("Company Logo".to_string()),
            })],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let output = generate(&doc);
        assert_eq!(output, "image::logo.png[Company Logo]");
    }

    #[test]
    fn test_page_break() {
        let mut doc = Document::new();
        doc.push(Block::Break(BreakType::Page));

        let output = generate(&doc);
        assert_eq!(output, "<<<");
    }

    #[test]
    fn test_section_break() {
        let mut doc = Document::new();
        doc.push(Block::Break(BreakType::Section));

        let output = generate(&doc);
        assert_eq!(output, "'''");
    }

    #[test]
    fn test_admonition() {
        let mut doc = Document::new();
        doc.push(Block::Admonition(Admonition {
            admonition_type: AdmonitionType::Warning,
            content: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Be careful!".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            })],
            title: None,
        }));

        let output = generate(&doc);
        assert!(output.contains("[WARNING]"));
        assert!(output.contains("===="));
        assert!(output.contains("Be careful!"));
    }

    #[test]
    fn test_complete_document() {
        // Create a document with heading, paragraph with bold text, and a list
        let mut doc = Document::new();

        // Heading
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Getting Started".to_string())],
            style_id: None,
            anchor: None,
        }));

        // Paragraph with bold text
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Text("This is ".to_string()),
                Inline::Format(
                    FormatType::Bold,
                    Box::new(Inline::Text("important".to_string())),
                ),
                Inline::Text(" information.".to_string()),
            ],
            style_id: None,
            attributes: HashMap::new(),
        }));

        // Unordered list
        doc.push(Block::List(List {
            list_type: ListType::Unordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Install dependencies".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Run the build".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
            ],
            style_id: None,
        }));

        let output = generate(&doc);

        // Verify structure (level 1 -> ==)
        assert!(output.contains("== Getting Started"));
        assert!(output.contains("This is *important* information."));
        assert!(output.contains("* Install dependencies"));
        assert!(output.contains("* Run the build"));

        // Verify proper formatting
        let expected = "== Getting Started

This is *important* information.

* Install dependencies
* Run the build";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_document_with_header() {
        let doc = Document {
            metadata: DocumentMeta {
                title: Some("My Document".to_string()),
                authors: vec!["Author One".to_string()],
                revision: Some("1.0".to_string()),
                attributes: HashMap::new(),
            },
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Content here.".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            })],
            intent: None,
        };

        let config = GeneratorConfig {
            include_header: true,
            generate_anchors: false,
        };
        let output = generate_with_config(&doc, config);

        assert!(output.contains("= My Document"));
        assert!(output.contains("Author One"));
        assert!(output.contains("v1.0"));
        assert!(output.contains("Content here."));
    }

    #[test]
    fn test_table() {
        let mut doc = Document::new();
        doc.push(Block::Table(Table {
            rows: vec![
                TableRow {
                    cells: vec![
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Header 1".to_string())],
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 1,
                            align: None,
                        },
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Header 2".to_string())],
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 1,
                            align: None,
                        },
                    ],
                    is_header: true,
                },
                TableRow {
                    cells: vec![
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Data 1".to_string())],
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 1,
                            align: None,
                        },
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Data 2".to_string())],
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 1,
                            align: None,
                        },
                    ],
                    is_header: false,
                },
            ],
            style_id: None,
            caption: None,
            columns: vec![],
        }));

        let output = generate(&doc);
        assert!(output.contains("[%header]"));
        assert!(output.contains("|==="));
        assert!(output.contains("| Header 1"));
        assert!(output.contains("| Data 1"));
    }
}
