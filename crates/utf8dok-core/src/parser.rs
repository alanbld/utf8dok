//! AsciiDoc Parser
//!
//! This module parses AsciiDoc text into a `utf8dok_ast::Document`.
//!
//! # Supported Syntax (MVP)
//!
//! See `docs/RENDER_SPEC.md` for the full specification.
//!
//! - Document title: `= Title`
//! - Attributes: `:key: value`
//! - Headings: `== Level 1`, `=== Level 2`, etc.
//! - Paragraphs: Text separated by blank lines
//! - Formatting: `*bold*`, `_italic_`, `` `mono` ``
//! - Lists: `* unordered`, `. ordered`
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_core::parser;
//!
//! let input = r#"= My Document
//!
//! == Introduction
//!
//! Hello *world*.
//! "#;
//!
//! let doc = parser::parse(input)?;
//! assert_eq!(doc.metadata.title, Some("My Document".to_string()));
//! ```

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use utf8dok_ast::{
    Block, Document, DocumentMeta, FormatType, Heading, Inline, Link, List, ListItem, ListType,
    LiteralBlock, Paragraph, Table, TableCell, TableRow,
};

/// Parser state for tracking what kind of block we're currently building
#[derive(Debug, Clone, PartialEq)]
enum ParserState {
    /// At the root level, not in any block
    Root,
    /// Building a paragraph with accumulated lines
    Paragraph(Vec<String>),
    /// Building a list with accumulated items
    List(ListType, Vec<ListItem>),
    /// Building a table with rows and current row being built
    /// (completed_rows, current_row_cells, expected_column_count)
    Table {
        rows: Vec<Vec<TableCell>>,
        current_row: Vec<TableCell>,
        col_count: Option<usize>,
    },
    /// Building a literal block (delimited by ----)
    Literal(Vec<String>),
}

/// AsciiDoc parser using a state machine approach
struct Parser {
    /// Document metadata
    metadata: DocumentMeta,
    /// Accumulated blocks
    blocks: Vec<Block>,
    /// Current parser state
    state: ParserState,
    /// Whether we've parsed the document header (title + attributes)
    header_done: bool,
    /// Pending block attributes (e.g., [source,rust], [mermaid])
    pending_attributes: Vec<String>,
}

impl Parser {
    fn new() -> Self {
        Self {
            metadata: DocumentMeta::default(),
            blocks: Vec::new(),
            state: ParserState::Root,
            header_done: false,
            pending_attributes: Vec::new(),
        }
    }

    /// Parse the entire document
    fn parse(mut self, text: &str) -> Result<Document> {
        // Normalize line endings
        let text = text.replace("\r\n", "\n");

        for line in text.lines() {
            self.process_line(line);
        }

        // Flush any remaining state
        self.flush_state();

        Ok(Document {
            metadata: self.metadata,
            blocks: self.blocks,
            intent: None,
        })
    }

    /// Process a single line
    fn process_line(&mut self, line: &str) {
        // Check for document title (level 0 heading)
        if !self.header_done && line.starts_with("= ") && !line.starts_with("== ") {
            self.flush_state();
            let title = line[2..].trim().to_string();
            self.metadata.title = Some(title);
            return;
        }

        // Check for document attributes (only in header)
        if !self.header_done && line.starts_with(':') && line.contains(": ") {
            if let Some((key, value)) = self.parse_attribute(line) {
                self.metadata.attributes.insert(key, value);
                return;
            }
        }

        // Skip block-level attribute lines (appear after headings, not content)
        // These are metadata like :slide-layout:, :slide-bullets:, etc.
        if line.starts_with(':') && line.ends_with(':') {
            // Boolean attribute like :toc:
            return;
        }
        if line.starts_with(':') && line.contains(": ") {
            // Key-value attribute like :slide-layout: Title
            // Skip known block attributes that shouldn't be rendered
            if let Some((key, _)) = self.parse_attribute(line) {
                if Self::is_block_attribute(&key) {
                    return;
                }
            }
        }

        // Empty line handling
        if line.trim().is_empty() {
            // If we're inside a table, empty lines are row separators
            if let ParserState::Table { rows, current_row, col_count: _ } = &mut self.state {
                if !current_row.is_empty() {
                    // Push current row to rows and start a new row
                    rows.push(std::mem::take(current_row));
                }
                return;
            }
            self.flush_state();
            self.header_done = true;
            return;
        }

        // Once we see a non-header element, header is done
        self.header_done = true;

        // Check for table delimiter |===
        if line.trim() == "|===" {
            match &self.state {
                ParserState::Table { .. } => {
                    // End of table - flush it
                    self.flush_state();
                }
                _ => {
                    // Start of table - flush any previous state and start table
                    self.flush_state();
                    self.state = ParserState::Table {
                        rows: Vec::new(),
                        current_row: Vec::new(),
                        col_count: None,
                    };
                }
            }
            return;
        }

        // If we're in a table, handle table cell lines
        if let ParserState::Table { rows, current_row, col_count } = &mut self.state {
            if let Some(cell_content) = line.strip_prefix('|') {
                // Split by | to handle multiple cells on one line: | A | B | C
                let cell_parts: Vec<&str> = cell_content.split('|').collect();

                // Collect cells from this line
                let mut line_cells = Vec::new();
                let is_multicell_line = cell_parts.len() > 1;

                if cell_parts.len() == 1 {
                    // Single cell - content may be empty (for empty cells like "| ")
                    let content = cell_content.trim();
                    let inlines = parse_inlines(content);
                    line_cells.push(TableCell {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines,
                            style_id: None,
                            attributes: HashMap::new(),
                        })],
                        colspan: 1,
                        rowspan: 1,
                        align: None,
                    });
                } else {
                    // Multiple cells on this line: | A | B | C
                    for cell_text in cell_parts {
                        let trimmed = cell_text.trim();
                        let inlines = parse_inlines(trimmed);
                        line_cells.push(TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines,
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 1,
                            align: None,
                        });
                    }
                }

                // Set column count only if this is a multi-cell line
                // (cells on separate lines use blank-line row separators)
                if col_count.is_none() && is_multicell_line {
                    *col_count = Some(line_cells.len());
                }

                // Add cells to current row
                current_row.extend(line_cells);

                // If we have multi-cell rows and filled a row, push it and start new
                if let Some(cols) = *col_count {
                    if current_row.len() >= cols {
                        rows.push(std::mem::take(current_row));
                    }
                }
            }
            return;
        }

        // Check for literal block delimiter (---- or more dashes)
        if line.starts_with("----") && line.chars().all(|c| c == '-') {
            match &self.state {
                ParserState::Literal(_) => {
                    // End of literal block - flush it
                    self.flush_state();
                }
                _ => {
                    // Start of literal block
                    self.flush_state();
                    self.state = ParserState::Literal(Vec::new());
                }
            }
            return;
        }

        // If we're in a literal block, capture lines verbatim
        if let ParserState::Literal(lines) = &mut self.state {
            lines.push(line.to_string());
            return;
        }

        // Check for block attributes [...]
        if line.starts_with('[') && line.ends_with(']') && !line.contains("[[") {
            // Don't flush state - attributes accumulate
            let attr_content = &line[1..line.len() - 1];
            self.pending_attributes.push(attr_content.to_string());
            return;
        }

        // Check for headings (== Level 1, === Level 2, etc.)
        if let Some(heading) = self.try_parse_heading(line) {
            self.flush_state();
            self.pending_attributes.clear(); // Headings don't use block attributes in MVP
            self.blocks.push(Block::Heading(heading));
            return;
        }

        // Check for unordered list item (* item or ** item)
        if let Some((level, content)) = self.try_parse_unordered_item(line) {
            self.handle_list_item(ListType::Unordered, level, content);
            return;
        }

        // Check for ordered list item (. item or .. item)
        if let Some((level, content)) = self.try_parse_ordered_item(line) {
            self.handle_list_item(ListType::Ordered, level, content);
            return;
        }

        // Otherwise, it's paragraph content
        self.handle_paragraph_line(line);
    }

    /// Check if an attribute key is a block-level attribute that should not be rendered
    fn is_block_attribute(key: &str) -> bool {
        // Dual-nature attributes
        let block_attrs = [
            "slide-layout",
            "slide-bullets",
            "slide-master",
            "slide-notes",
            "slide-transition",
            "slide-background",
            "document-style",
            "document-class",
            // Common AsciiDoc block attributes
            "source-highlighter",
            "icons",
            "icon",
            "caption",
            "title",
            "id",
            "role",
            "options",
            "cols",
            "frame",
            "grid",
            "width",
            "height",
            "align",
            "float",
            "language",
        ];
        block_attrs.contains(&key.to_lowercase().as_str())
    }

    /// Parse an attribute line like `:key: value`
    fn parse_attribute(&self, line: &str) -> Option<(String, String)> {
        let line = line.trim_start_matches(':');
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            if !key.is_empty() {
                return Some((key, value));
            }
        }
        None
    }

    /// Try to parse a heading line
    fn try_parse_heading(&self, line: &str) -> Option<Heading> {
        // Count leading '=' characters
        let mut level = 0;
        for ch in line.chars() {
            if ch == '=' {
                level += 1;
            } else {
                break;
            }
        }

        // Must have at least 2 '=' for a heading (== is level 1)
        // and must be followed by a space
        if level >= 2 && line.len() > level && line.chars().nth(level) == Some(' ') {
            let text = line[level + 1..].trim().to_string();
            return Some(Heading {
                level: (level - 1) as u8, // == is level 1, === is level 2, etc.
                text: vec![Inline::Text(text)],
                style_id: None,
                anchor: None,
            });
        }

        None
    }

    /// Try to parse an unordered list item
    fn try_parse_unordered_item(&self, line: &str) -> Option<(usize, String)> {
        // Count leading '*' characters
        let mut level = 0;
        for ch in line.chars() {
            if ch == '*' {
                level += 1;
            } else {
                break;
            }
        }

        // Must have at least one '*' followed by a space
        if level >= 1 && line.len() > level && line.chars().nth(level) == Some(' ') {
            let content = line[level + 1..].trim().to_string();
            return Some((level - 1, content)); // level 0 = *, level 1 = **, etc.
        }

        None
    }

    /// Try to parse an ordered list item
    fn try_parse_ordered_item(&self, line: &str) -> Option<(usize, String)> {
        // Count leading '.' characters
        let mut level = 0;
        for ch in line.chars() {
            if ch == '.' {
                level += 1;
            } else {
                break;
            }
        }

        // Must have at least one '.' followed by a space
        if level >= 1 && line.len() > level && line.chars().nth(level) == Some(' ') {
            let content = line[level + 1..].trim().to_string();
            return Some((level - 1, content)); // level 0 = ., level 1 = .., etc.
        }

        None
    }

    /// Handle a list item
    fn handle_list_item(&mut self, list_type: ListType, level: usize, content: String) {
        let inlines = parse_inlines(&content);
        let item = ListItem {
            content: vec![Block::Paragraph(Paragraph {
                inlines,
                style_id: None,
                attributes: HashMap::new(),
            })],
            level: level as u8,
            term: None,
        };

        match &mut self.state {
            ParserState::List(current_type, items) if *current_type == list_type => {
                // Continue the current list
                items.push(item);
            }
            _ => {
                // Start a new list (flush any previous state)
                self.flush_state();
                self.state = ParserState::List(list_type, vec![item]);
            }
        }
    }

    /// Handle a paragraph line
    fn handle_paragraph_line(&mut self, line: &str) {
        match &mut self.state {
            ParserState::Paragraph(lines) => {
                // Continue the current paragraph
                lines.push(line.to_string());
            }
            _ => {
                // Start a new paragraph
                self.flush_state();
                self.state = ParserState::Paragraph(vec![line.to_string()]);
            }
        }
    }

    /// Flush the current state to blocks
    fn flush_state(&mut self) {
        let state = std::mem::replace(&mut self.state, ParserState::Root);

        match state {
            ParserState::Root => {}
            ParserState::Paragraph(lines) => {
                if !lines.is_empty() {
                    let text = lines.join(" ");
                    let inlines = parse_inlines(&text);
                    self.blocks.push(Block::Paragraph(Paragraph {
                        inlines,
                        style_id: None,
                        attributes: HashMap::new(),
                    }));
                }
            }
            ParserState::List(list_type, items) => {
                if !items.is_empty() {
                    self.blocks.push(Block::List(List {
                        list_type,
                        items,
                        style_id: None,
                    }));
                }
            }
            ParserState::Table {
                mut rows,
                current_row,
                col_count: _,
            } => {
                // Push any remaining current_row to rows
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                if !rows.is_empty() {
                    // Convert Vec<Vec<TableCell>> to Vec<TableRow>
                    let table_rows: Vec<TableRow> = rows
                        .into_iter()
                        .map(|cells| TableRow {
                            cells,
                            is_header: false,
                        })
                        .collect();
                    self.blocks.push(Block::Table(Table {
                        rows: table_rows,
                        style_id: None,
                        caption: None,
                        columns: vec![],
                    }));
                }
            }
            ParserState::Literal(lines) => {
                // Create literal block with content and pending attributes
                let content = lines.join("\n");

                // Parse pending attributes to extract language and style
                let (language, style_id) = self.parse_block_attributes();

                self.blocks.push(Block::Literal(LiteralBlock {
                    content,
                    language,
                    title: None,
                    style_id,
                }));

                // Clear pending attributes after use
                self.pending_attributes.clear();
            }
        }
    }

    /// Parse pending block attributes to extract language and style_id.
    /// Handles formats like: [source,rust], [mermaid], [plantuml], etc.
    fn parse_block_attributes(&self) -> (Option<String>, Option<String>) {
        if self.pending_attributes.is_empty() {
            return (None, None);
        }

        // Take the first attribute (most recent/relevant)
        let attr = &self.pending_attributes[0];

        // Check for source block: [source,lang] or [source]
        if attr.starts_with("source") {
            if let Some(comma_pos) = attr.find(',') {
                let lang = attr[comma_pos + 1..].trim().to_string();
                return (Some(lang), None);
            }
            return (None, None);
        }

        // Check for known diagram types
        let known_diagram_types = [
            "mermaid",
            "plantuml",
            "graphviz",
            "ditaa",
            "d2",
            "blockdiag",
            "seqdiag",
            "actdiag",
            "nwdiag",
            "c4plantuml",
            "svgbob",
            "vega",
            "vegalite",
            "wavedrom",
            "bytefield",
            "erd",
            "nomnoml",
            "pikchr",
        ];

        let attr_lower = attr.to_lowercase();
        for diagram_type in known_diagram_types {
            if attr_lower == diagram_type {
                return (None, Some(attr.to_string()));
            }
        }

        // Default: treat as style_id
        (None, Some(attr.to_string()))
    }
}

/// Parse inline formatting in text
fn parse_inlines(text: &str) -> Vec<Inline> {
    // Regex patterns for inline formatting
    // Order matters: we process left-to-right
    let bold_re = Regex::new(r"\*([^*]+)\*").unwrap();
    let italic_re = Regex::new(r"_([^_]+)_").unwrap();
    let mono_re = Regex::new(r"`([^`]+)`").unwrap();
    // Cross-reference: <<anchor,text>> or <<anchor>>
    let xref_re = Regex::new(r"<<([^,>]+),([^>]+)>>|<<([^>]+)>>").unwrap();

    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest match of any formatting
        let bold_match = bold_re.find(remaining);
        let italic_match = italic_re.find(remaining);
        let mono_match = mono_re.find(remaining);
        let xref_match = xref_re.find(remaining);

        // Determine which match comes first
        let earliest = [
            bold_match.map(|m| (m.start(), m.end(), "bold")),
            italic_match.map(|m| (m.start(), m.end(), "italic")),
            mono_match.map(|m| (m.start(), m.end(), "mono")),
            xref_match.map(|m| (m.start(), m.end(), "xref")),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(start, _, _)| *start);

        match earliest {
            Some((start, end, format_type)) => {
                // Add any text before the match
                if start > 0 {
                    result.push(Inline::Text(remaining[..start].to_string()));
                }

                // Extract the content inside the markers
                let matched = &remaining[start..end];

                // Create the appropriate inline element
                let inline = match format_type {
                    "bold" => {
                        let content = &matched[1..matched.len() - 1]; // Remove * markers
                        Inline::Format(
                            FormatType::Bold,
                            Box::new(Inline::Text(content.to_string())),
                        )
                    }
                    "italic" => {
                        let content = &matched[1..matched.len() - 1]; // Remove _ markers
                        Inline::Format(
                            FormatType::Italic,
                            Box::new(Inline::Text(content.to_string())),
                        )
                    }
                    "mono" => {
                        let content = &matched[1..matched.len() - 1]; // Remove ` markers
                        Inline::Format(
                            FormatType::Monospace,
                            Box::new(Inline::Text(content.to_string())),
                        )
                    }
                    "xref" => {
                        // Parse cross-reference: <<anchor,text>> or <<anchor>>
                        if let Some(caps) = xref_re.captures(matched) {
                            if let (Some(anchor), Some(text_match)) = (caps.get(1), caps.get(2)) {
                                // <<anchor,text>> format
                                Inline::Link(Link {
                                    url: format!("#{}", anchor.as_str()),
                                    text: vec![Inline::Text(text_match.as_str().to_string())],
                                })
                            } else if let Some(anchor) = caps.get(3) {
                                // <<anchor>> format (no text, use anchor as text)
                                let anchor_str = anchor.as_str();
                                Inline::Link(Link {
                                    url: format!("#{}", anchor_str),
                                    text: vec![Inline::Text(anchor_str.to_string())],
                                })
                            } else {
                                // Fallback: treat as plain text
                                Inline::Text(matched.to_string())
                            }
                        } else {
                            Inline::Text(matched.to_string())
                        }
                    }
                    _ => unreachable!(),
                };
                result.push(inline);

                // Continue with the rest
                remaining = &remaining[end..];
            }
            None => {
                // No more formatting, add remaining text
                if !remaining.is_empty() {
                    result.push(Inline::Text(remaining.to_string()));
                }
                break;
            }
        }
    }

    // Handle empty input
    if result.is_empty() && text.is_empty() {
        result.push(Inline::Text(String::new()));
    }

    result
}

/// Parse AsciiDoc text into an AST Document.
///
/// # Arguments
///
/// * `text` - The AsciiDoc source text to parse
///
/// # Returns
///
/// * `Ok(Document)` - The parsed document AST
/// * `Err(anyhow::Error)` - If parsing fails
///
/// # Errors
///
/// Currently, the parser is lenient and will not fail on unknown syntax.
/// Unknown constructs are treated as plain paragraph text.
pub fn parse(text: &str) -> Result<Document> {
    let parser = Parser::new();
    parser.parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_placeholder() {
        let result = parse("= Test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_inlines_simple() {
        let inlines = parse_inlines("Hello world");
        assert_eq!(inlines, vec![Inline::Text("Hello world".to_string())]);
    }

    #[test]
    fn test_parse_inlines_bold() {
        let inlines = parse_inlines("Hello *world*");
        assert_eq!(inlines.len(), 2);
        assert_eq!(inlines[0], Inline::Text("Hello ".to_string()));
        assert!(matches!(inlines[1], Inline::Format(FormatType::Bold, _)));
    }

    #[test]
    fn test_parse_inlines_xref_with_text() {
        let inlines = parse_inlines("See <<section1,Section One>> for details");
        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[0], Inline::Text("See ".to_string()));

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "#section1");
            assert_eq!(link.text.len(), 1);
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "Section One");
            } else {
                panic!("Expected Text inline in link");
            }
        } else {
            panic!("Expected Link inline");
        }

        assert_eq!(inlines[2], Inline::Text(" for details".to_string()));
    }

    #[test]
    fn test_parse_inlines_xref_without_text() {
        let inlines = parse_inlines("See <<section1>> for details");
        assert_eq!(inlines.len(), 3);

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "#section1");
            assert_eq!(link.text.len(), 1);
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "section1"); // Uses anchor as text
            } else {
                panic!("Expected Text inline in link");
            }
        } else {
            panic!("Expected Link inline");
        }
    }

    #[test]
    fn test_parse_heading_levels() {
        // == should parse as level 1
        let doc = parse("== Level 1 Heading").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Heading(h) = &doc.blocks[0] {
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected Heading block");
        }

        // === should parse as level 2
        let doc = parse("=== Level 2 Heading").unwrap();
        if let Block::Heading(h) = &doc.blocks[0] {
            assert_eq!(h.level, 2);
        } else {
            panic!("Expected Heading block");
        }

        // ==== should parse as level 3
        let doc = parse("==== Level 3 Heading").unwrap();
        if let Block::Heading(h) = &doc.blocks[0] {
            assert_eq!(h.level, 3);
        } else {
            panic!("Expected Heading block");
        }
    }
}
