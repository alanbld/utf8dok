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
    Block, Document, DocumentMeta, FormatType, Heading, Inline, List, ListItem, ListType,
    Paragraph,
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
}

impl Parser {
    fn new() -> Self {
        Self {
            metadata: DocumentMeta::default(),
            blocks: Vec::new(),
            state: ParserState::Root,
            header_done: false,
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

        // Empty line handling
        if line.trim().is_empty() {
            self.flush_state();
            self.header_done = true;
            return;
        }

        // Once we see a non-header element, header is done
        self.header_done = true;

        // Check for headings (== Level 1, === Level 2, etc.)
        if let Some(heading) = self.try_parse_heading(line) {
            self.flush_state();
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
        }
    }
}

/// Parse inline formatting in text
fn parse_inlines(text: &str) -> Vec<Inline> {
    // Regex patterns for inline formatting
    // Order matters: we process left-to-right
    let bold_re = Regex::new(r"\*([^*]+)\*").unwrap();
    let italic_re = Regex::new(r"_([^_]+)_").unwrap();
    let mono_re = Regex::new(r"`([^`]+)`").unwrap();

    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest match of any formatting
        let bold_match = bold_re.find(remaining);
        let italic_match = italic_re.find(remaining);
        let mono_match = mono_re.find(remaining);

        // Determine which match comes first
        let earliest = [
            bold_match.map(|m| (m.start(), m.end(), "bold")),
            italic_match.map(|m| (m.start(), m.end(), "italic")),
            mono_match.map(|m| (m.start(), m.end(), "mono")),
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
                let content = &matched[1..matched.len() - 1]; // Remove markers

                // Create the formatted inline
                let inline = match format_type {
                    "bold" => {
                        Inline::Format(FormatType::Bold, Box::new(Inline::Text(content.to_string())))
                    }
                    "italic" => Inline::Format(
                        FormatType::Italic,
                        Box::new(Inline::Text(content.to_string())),
                    ),
                    "mono" => Inline::Format(
                        FormatType::Monospace,
                        Box::new(Inline::Text(content.to_string())),
                    ),
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
}
