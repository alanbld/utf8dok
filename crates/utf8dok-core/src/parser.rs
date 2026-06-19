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
    Block, BreakType, Document, DocumentMeta, FormatType, Heading, Image, Inline, Link, List,
    ListItem, ListType, LiteralBlock, Paragraph, Table, TableCell, TableRow,
};

use crate::include::{resolve_data_include, IncludeDirective};

/// Configuration for the parser
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    /// Base path for resolving relative include paths
    pub base_path: Option<String>,
    /// Whether to resolve data includes (Excel, CSV) to tables
    pub resolve_data_includes: bool,
    /// Whether to emit warnings for unresolved includes
    pub warn_unresolved: bool,
}

impl ParserConfig {
    /// Create a new parser config with data includes enabled
    pub fn with_data_includes(base_path: impl Into<String>) -> Self {
        Self {
            base_path: Some(base_path.into()),
            resolve_data_includes: true,
            warn_unresolved: true,
        }
    }
}

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
        /// Lines being accumulated for an `a|` (AsciiDoc-in-cell) cell
        asciidoc_cell_lines: Option<Vec<String>>,
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
    /// Whether we've seen the document title (= Title)
    title_seen: bool,
    /// Whether we've seen the author line (immediately after title)
    author_seen: bool,
    /// Pending block attributes (e.g., [source,rust], [mermaid])
    pending_attributes: Vec<String>,
    /// Parser configuration
    config: ParserConfig,
    /// Warnings accumulated during parsing
    warnings: Vec<String>,
    /// Depth counter for skipping false ifdef/ifndef blocks
    skip_depth: u32,
}

impl Parser {
    fn new() -> Self {
        Self::with_config(ParserConfig::default())
    }

    fn with_config(config: ParserConfig) -> Self {
        Self {
            metadata: DocumentMeta::default(),
            blocks: Vec::new(),
            state: ParserState::Root,
            header_done: false,
            title_seen: false,
            author_seen: false,
            pending_attributes: Vec::new(),
            config,
            warnings: Vec::new(),
            skip_depth: 0,
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
        // Preprocessor: ifdef/ifndef/endif handling (must be checked first)
        if let Some(rest) = line.strip_prefix("ifdef::") {
            if let Some(attr) = rest.strip_suffix("[]") {
                let defined = self.metadata.attributes.contains_key(attr);
                if !defined {
                    self.skip_depth += 1;
                }
                return;
            }
        }
        if let Some(rest) = line.strip_prefix("ifndef::") {
            if let Some(attr) = rest.strip_suffix("[]") {
                let defined = self.metadata.attributes.contains_key(attr);
                if defined {
                    self.skip_depth += 1;
                }
                return;
            }
        }
        if line.starts_with("endif::") {
            if self.skip_depth > 0 {
                self.skip_depth -= 1;
            }
            return;
        }
        // Skip all content inside a false conditional
        if self.skip_depth > 0 {
            return;
        }

        // Check for document title (level 0 heading)
        if !self.header_done && line.starts_with("= ") && !line.starts_with("== ") {
            self.flush_state();
            let title = line[2..].trim().to_string();
            self.metadata.title = Some(title);
            self.title_seen = true;
            return;
        }

        // Check for document attributes (only in header)
        if !self.header_done && line.starts_with(':') {
            // Boolean attribute like :sectnums: (starts and ends with :)
            if line.ends_with(':') && line.len() > 2 {
                let key = line[1..line.len() - 1].trim().to_string();
                if !key.is_empty() {
                    self.metadata.attributes.insert(key, String::new());
                    return;
                }
            }
            // Key-value attribute like :doctype: book
            if line.contains(": ") {
                if let Some((key, value)) = self.parse_attribute(line) {
                    self.metadata.attributes.insert(key, value);
                    return;
                }
            }
        }

        // In header: parse author line (immediately after title, not starting with : or =)
        // Must be non-empty and appear before any blank line or attribute.
        // If we hit a blank line or attribute first, there is no author line and the
        // condition simply falls through to be handled by the subsequent checks.
        if !self.header_done
            && self.title_seen
            && !self.author_seen
            && !line.trim().is_empty()
            && !line.starts_with(':')
            && !line.starts_with('=')
        {
            // Author line: "Name <email>" or "Name; Name2 <email2>"
            self.author_seen = true;
            let authors = Self::parse_author_line(line);
            if !authors.is_empty() {
                self.metadata.authors = authors;
            }
            return;
        }

        // In header: parse revision line (immediately after author, not starting with : or =)
        if !self.header_done
            && self.author_seen
            && self.metadata.revision.is_none()
            && !line.trim().is_empty()
            && !line.starts_with(':')
            && !line.starts_with('=')
        {
            // Revision line: "v1.0, 2025-02-09" or similar
            self.metadata.revision = Some(line.trim().to_string());
            return;
        }

        // Skip block-level attribute lines (appear after headings, not content)
        // These are metadata like :slide-layout:, :slide-bullets:, etc.
        if self.header_done && line.starts_with(':') && line.ends_with(':') {
            // Boolean attribute like :toc: outside header — skip silently
            return;
        }
        if self.header_done && line.starts_with(':') && line.contains(": ") {
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
            if let ParserState::Table {
                rows,
                current_row,
                col_count: _,
                asciidoc_cell_lines,
            } = &mut self.state
            {
                // Flush any pending a| cell before treating blank line as row separator
                if let Some(lines) = asciidoc_cell_lines.take() {
                    let cell = Self::parse_asciidoc_cell(&lines);
                    current_row.push(cell);
                }
                if !current_row.is_empty() {
                    // Push current row to rows and start a new row
                    rows.push(std::mem::take(current_row));
                }
                return;
            }
            self.flush_state();
            // Don't set header_done on blank lines if we haven't seen any body content yet
            // (blank lines within the header are separators, e.g., after ifdef blocks)
            if self.title_seen {
                self.header_done = true;
            }
            return;
        }

        // Skip single-line comments (// ...) — valid anywhere in AsciiDoc
        if line.starts_with("//") && !line.starts_with("///") {
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
                        asciidoc_cell_lines: None,
                    };
                }
            }
            return;
        }

        // If we're in a table, handle table cell lines
        if let ParserState::Table {
            rows,
            current_row,
            col_count,
            asciidoc_cell_lines,
        } = &mut self.state
        {
            // Check for a| (AsciiDoc-in-cell) start
            if line == "a|" || line.starts_with("a| ") {
                // Flush any previous a| cell
                if let Some(lines) = asciidoc_cell_lines.take() {
                    let cell = Self::parse_asciidoc_cell(&lines);
                    current_row.push(cell);
                }
                // Start new a| accumulation
                let initial = if line == "a|" {
                    Vec::new()
                } else {
                    vec![line[2..].trim().to_string()]
                };
                *asciidoc_cell_lines = Some(initial);
                return;
            }

            // If we're accumulating a| content, check for terminators
            if asciidoc_cell_lines.is_some() {
                // A new | cell or a| starts → flush the accumulated cell
                if line.starts_with('|') || line == "a|" || line.starts_with("a| ") {
                    let lines = asciidoc_cell_lines.take().unwrap();
                    let cell = Self::parse_asciidoc_cell(&lines);
                    current_row.push(cell);
                    // Fall through to handle the current line as a normal cell
                } else {
                    // Accumulate content for the a| cell
                    asciidoc_cell_lines.as_mut().unwrap().push(line.to_string());
                    return;
                }
            }

            if let Some(cell_content) = line.strip_prefix('|') {
                // Split by | to handle multiple cells on one line: | A | B | C
                let cell_parts: Vec<&str> = cell_content.split('|').collect();

                // Collect cells from this line
                let mut line_cells = Vec::new();
                let is_multicell_line = cell_parts.len() > 1;

                if cell_parts.len() == 1 {
                    // Single cell - content may be empty (for empty cells like "| ")
                    let content = cell_content.trim();
                    let inlines =
                        parse_inlines_with_attrs(content, Some(&self.metadata.attributes));
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
                        let inlines =
                            parse_inlines_with_attrs(trimmed, Some(&self.metadata.attributes));
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

        // Check for page break (<<<)
        if line.trim() == "<<<" {
            self.flush_state();
            self.blocks.push(Block::Break(BreakType::Page));
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

        // Check for image macro (image::path[alt, attrs])
        if let Some(para) = self.try_parse_image(line) {
            self.flush_state();
            self.blocks.push(Block::Paragraph(para));
            return;
        }

        // Check for include directive (include::path[attrs])
        if let Some(block) = self.try_parse_include(line) {
            self.flush_state();
            self.blocks.push(block);
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

    /// Parse an AsciiDoc author line into a list of author names.
    /// Supports: "Name <email>", "Name; Name2 <email2>", or plain "Name"
    fn parse_author_line(line: &str) -> Vec<String> {
        let mut authors = Vec::new();
        for part in line.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            // Strip email if present: "Name <email>" → "Name"
            let name = if let Some(angle_pos) = part.find('<') {
                part[..angle_pos].trim()
            } else {
                part.trim()
            };
            if !name.is_empty() {
                authors.push(name.to_string());
            }
        }
        authors
    }

    /// Parse accumulated lines as AsciiDoc content for an `a|` table cell.
    /// Uses a fresh Parser instance to recursively parse the cell content.
    fn parse_asciidoc_cell(lines: &[String]) -> TableCell {
        let cell_source = lines.join("\n");
        // Parse the cell content as a mini-document
        let parser = Parser::new();
        let doc = parser.parse(&cell_source).unwrap_or_else(|_| Document {
            metadata: DocumentMeta::default(),
            blocks: vec![],
            intent: None,
        });
        let content = if doc.blocks.is_empty() {
            // Ensure at least one paragraph (OOXML requires it)
            vec![Block::Paragraph(Paragraph {
                inlines: vec![],
                style_id: None,
                attributes: HashMap::new(),
            })]
        } else {
            doc.blocks
        };
        TableCell {
            content,
            colspan: 1,
            rowspan: 1,
            align: None,
        }
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
            let anchor = generate_heading_anchor(&text);
            return Some(Heading {
                level: (level - 1) as u8, // == is level 1, === is level 2, etc.
                text: vec![Inline::Text(text)],
                style_id: None,
                anchor: Some(anchor),
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

    /// Try to parse an image macro line: image::path[alt, attrs]
    fn try_parse_image(&self, line: &str) -> Option<Paragraph> {
        // Match image::path[attributes] pattern
        if !line.starts_with("image::") {
            return None;
        }

        // Find the bracket containing attributes
        let rest = &line[7..]; // Skip "image::"
        let bracket_start = rest.find('[')?;
        let bracket_end = rest.rfind(']')?;

        if bracket_start >= bracket_end {
            return None;
        }

        let path = rest[..bracket_start].to_string();
        let attrs_str = &rest[bracket_start + 1..bracket_end];

        // Parse attributes: first attribute is alt text, then key=value pairs
        let mut alt = None;
        for (i, attr) in attrs_str.split(',').enumerate() {
            let attr = attr.trim();
            if i == 0 && !attr.contains('=') && !attr.is_empty() {
                // First non-key=value is alt text
                alt = Some(attr.to_string());
            }
            // We could parse width/height here if needed, but DOCX writer uses defaults
        }

        // Create paragraph with inline image
        Some(Paragraph {
            inlines: vec![Inline::Image(Image { src: path, alt })],
            style_id: None,
            attributes: HashMap::new(),
        })
    }

    /// Try to parse an include directive: include::path[attrs]
    ///
    /// For data files (xlsx, csv, tsv), resolves to a Table block.
    /// For other files, returns a placeholder paragraph (or could be extended).
    fn try_parse_include(&mut self, line: &str) -> Option<Block> {
        // Parse the include directive
        let directive = IncludeDirective::parse(line)?;

        // Only handle data file includes
        if !directive.is_data_file() {
            // For non-data includes, we could expand this later
            // For now, emit a warning and skip
            if self.config.warn_unresolved {
                self.warnings
                    .push(format!("Non-data include not resolved: {}", directive.path));
            }
            return None;
        }

        // Check if we should resolve includes
        if !self.config.resolve_data_includes {
            if self.config.warn_unresolved {
                self.warnings.push(format!(
                    "Data include not resolved (disabled): {}",
                    directive.path
                ));
            }
            // Return a placeholder paragraph
            return Some(Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text(format!("[Include: {}]", directive.path))],
                style_id: None,
                attributes: HashMap::new(),
            }));
        }

        // Get base path
        let base_path = self.config.base_path.as_deref().unwrap_or(".");

        // Resolve the include to a table
        match resolve_data_include(&directive, base_path) {
            Ok(table) => Some(Block::Table(table)),
            Err(err) => {
                self.warnings.push(format!(
                    "Failed to resolve include '{}': {}",
                    directive.path, err
                ));
                // Return error placeholder
                Some(Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text(format!(
                        "[Include error: {} - {}]",
                        directive.path, err
                    ))],
                    style_id: None,
                    attributes: HashMap::new(),
                }))
            }
        }
    }

    /// Handle a list item
    fn handle_list_item(&mut self, list_type: ListType, level: usize, content: String) {
        let inlines = parse_inlines_with_attrs(&content, Some(&self.metadata.attributes));
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
                    let inlines = parse_inlines_with_attrs(&text, Some(&self.metadata.attributes));
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
                mut current_row,
                col_count: _,
                asciidoc_cell_lines,
            } => {
                // Flush any pending a| cell
                if let Some(lines) = asciidoc_cell_lines {
                    let cell = Self::parse_asciidoc_cell(&lines);
                    current_row.push(cell);
                }
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

/// Generate a URL-friendly anchor from heading text
///
/// Converts to lowercase, replaces non-alphanumeric characters with hyphens,
/// collapses consecutive hyphens, and trims leading/trailing hyphens.
fn generate_heading_anchor(text: &str) -> String {
    let raw: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse consecutive hyphens and trim
    let mut result = String::new();
    let mut prev_hyphen = false;
    for c in raw.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }
    result
}

/// Parse inline formatting in text (without attribute substitution, used in tests)
#[cfg(test)]
fn parse_inlines(text: &str) -> Vec<Inline> {
    parse_inlines_with_attrs(text, None)
}

/// Parse inline formatting in text, with optional attribute substitution
fn parse_inlines_with_attrs(text: &str, attrs: Option<&HashMap<String, String>>) -> Vec<Inline> {
    // Regex patterns for inline formatting
    // Order matters: we process left-to-right
    let bold_re = Regex::new(r"\*([^*]+)\*").unwrap();
    let italic_re = Regex::new(r"_([^_]+)_").unwrap();
    let mono_re = Regex::new(r"`([^`]+)`").unwrap();
    // Cross-reference: <<anchor,text>> or <<anchor>>
    let xref_re = Regex::new(r"<<([^,>]+),([^>]+)>>|<<([^>]+)>>").unwrap();
    // Inline anchor: [[name]]
    let anchor_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    // URL with text: https://url[text] or http://url[text]
    let url_with_text_re = Regex::new(r"(https?://[^\s\[]+)\[([^\]]+)\]").unwrap();
    // Attribute reference: {attribute-name}
    let attr_re = Regex::new(r"\{([a-zA-Z0-9_-]+)\}").unwrap();
    // Link macro: link:url[text]
    let link_macro_re = Regex::new(r"link:([^\[]+)\[([^\]]*)\]").unwrap();
    // Bare URL: https://url or http://url (ends at whitespace or end of string)
    // Note: url_with_text should be checked first due to earliest match logic
    let bare_url_re = Regex::new(r"https?://[^\s\[\]<>]+").unwrap();

    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest match of any formatting
        let bold_match = bold_re.find(remaining);
        let italic_match = italic_re.find(remaining);
        let mono_match = mono_re.find(remaining);
        let xref_match = xref_re.find(remaining);
        let anchor_match = anchor_re.find(remaining);
        let url_with_text_match = url_with_text_re.find(remaining);
        let link_macro_match = link_macro_re.find(remaining);
        let bare_url_match = bare_url_re.find(remaining);
        // Only search for attribute refs if we have an attributes map
        let attr_match = if attrs.is_some() {
            attr_re.find(remaining)
        } else {
            None
        };

        // Determine which match comes first
        let earliest = [
            bold_match.map(|m| (m.start(), m.end(), "bold")),
            italic_match.map(|m| (m.start(), m.end(), "italic")),
            mono_match.map(|m| (m.start(), m.end(), "mono")),
            xref_match.map(|m| (m.start(), m.end(), "xref")),
            anchor_match.map(|m| (m.start(), m.end(), "anchor")),
            url_with_text_match.map(|m| (m.start(), m.end(), "url_text")),
            link_macro_match.map(|m| (m.start(), m.end(), "link_macro")),
            bare_url_match.map(|m| (m.start(), m.end(), "bare_url")),
            attr_match.map(|m| (m.start(), m.end(), "attr")),
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
                        let inner = parse_inlines_with_attrs(content, attrs);
                        if inner.len() == 1 {
                            Inline::Format(
                                FormatType::Bold,
                                Box::new(inner.into_iter().next().unwrap()),
                            )
                        } else {
                            Inline::Format(
                                FormatType::Bold,
                                Box::new(Inline::Text(content.to_string())),
                            )
                        }
                    }
                    "italic" => {
                        let content = &matched[1..matched.len() - 1]; // Remove _ markers
                        let inner = parse_inlines_with_attrs(content, attrs);
                        if inner.len() == 1 {
                            Inline::Format(
                                FormatType::Italic,
                                Box::new(inner.into_iter().next().unwrap()),
                            )
                        } else {
                            Inline::Format(
                                FormatType::Italic,
                                Box::new(Inline::Text(content.to_string())),
                            )
                        }
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
                    "anchor" => {
                        // Parse anchor: [[name]]
                        if let Some(caps) = anchor_re.captures(matched) {
                            if let Some(name) = caps.get(1) {
                                Inline::Anchor(name.as_str().to_string())
                            } else {
                                Inline::Text(matched.to_string())
                            }
                        } else {
                            Inline::Text(matched.to_string())
                        }
                    }
                    "url_text" => {
                        // Parse URL with text: https://url[text]
                        if let Some(caps) = url_with_text_re.captures(matched) {
                            let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                            let text = caps.get(2).map(|m| m.as_str()).unwrap_or(url);
                            Inline::Link(Link {
                                url: url.to_string(),
                                text: vec![Inline::Text(text.to_string())],
                            })
                        } else {
                            Inline::Text(matched.to_string())
                        }
                    }
                    "link_macro" => {
                        // Parse link macro: link:url[text]
                        if let Some(caps) = link_macro_re.captures(matched) {
                            let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                            let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                            let display_text = if text.is_empty() { url } else { text };
                            Inline::Link(Link {
                                url: url.to_string(),
                                text: vec![Inline::Text(display_text.to_string())],
                            })
                        } else {
                            Inline::Text(matched.to_string())
                        }
                    }
                    "bare_url" => {
                        // Bare URL becomes a link with URL as text
                        Inline::Link(Link {
                            url: matched.to_string(),
                            text: vec![Inline::Text(matched.to_string())],
                        })
                    }
                    "attr" => {
                        // Attribute reference: {name} → resolved value or literal
                        if let Some(caps) = attr_re.captures(matched) {
                            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                            if let Some(value) = attrs.and_then(|a| a.get(name)) {
                                Inline::Text(value.clone())
                            } else {
                                // Unresolved attribute — emit literal
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

/// Parse AsciiDoc text with configuration options
///
/// # Arguments
///
/// * `text` - The AsciiDoc source text
/// * `config` - Parser configuration (includes, base path, etc.)
///
/// # Returns
///
/// * `Ok(Document)` - The parsed document AST
/// * `Err(anyhow::Error)` - If parsing fails
///
/// # Example
///
/// ```ignore
/// use utf8dok_core::{parse_with_config, ParserConfig};
///
/// let config = ParserConfig::with_data_includes("./data");
/// let doc = parse_with_config(input, config)?;
/// ```
pub fn parse_with_config(text: &str, config: ParserConfig) -> Result<Document> {
    let parser = Parser::with_config(config);
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
    fn test_parse_inlines_url_with_text() {
        let inlines = parse_inlines("Visit https://example.com[Example Site] for more");
        assert_eq!(inlines.len(), 3);
        assert_eq!(inlines[0], Inline::Text("Visit ".to_string()));

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "https://example.com");
            assert_eq!(link.text.len(), 1);
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "Example Site");
            } else {
                panic!("Expected Text inline in link");
            }
        } else {
            panic!("Expected Link inline");
        }

        assert_eq!(inlines[2], Inline::Text(" for more".to_string()));
    }

    #[test]
    fn test_parse_inlines_link_macro() {
        let inlines = parse_inlines("See link:https://docs.rs[Rust Docs] here");
        assert_eq!(inlines.len(), 3);

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "https://docs.rs");
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "Rust Docs");
            }
        } else {
            panic!("Expected Link inline");
        }
    }

    #[test]
    fn test_parse_inlines_link_macro_empty_text() {
        let inlines = parse_inlines("Go to link:https://example.com[]");

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "https://example.com");
            // Empty text should use URL as display text
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "https://example.com");
            }
        } else {
            panic!("Expected Link inline");
        }
    }

    #[test]
    fn test_parse_inlines_bare_url() {
        let inlines = parse_inlines("Check out https://github.com for code");
        assert_eq!(inlines.len(), 3);

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "https://github.com");
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "https://github.com");
            }
        } else {
            panic!("Expected Link inline");
        }
    }

    #[test]
    fn test_parse_inlines_http_url() {
        let inlines = parse_inlines("Visit http://legacy.example.com[Legacy]");

        if let Inline::Link(link) = &inlines[1] {
            assert_eq!(link.url, "http://legacy.example.com");
            if let Inline::Text(text) = &link.text[0] {
                assert_eq!(text, "Legacy");
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
            assert_eq!(h.anchor, Some("level-1-heading".to_string()));
        } else {
            panic!("Expected Heading block");
        }

        // === should parse as level 2
        let doc = parse("=== Level 2 Heading").unwrap();
        if let Block::Heading(h) = &doc.blocks[0] {
            assert_eq!(h.level, 2);
            assert_eq!(h.anchor, Some("level-2-heading".to_string()));
        } else {
            panic!("Expected Heading block");
        }

        // ==== should parse as level 3
        let doc = parse("==== Level 3 Heading").unwrap();
        if let Block::Heading(h) = &doc.blocks[0] {
            assert_eq!(h.level, 3);
            assert_eq!(h.anchor, Some("level-3-heading".to_string()));
        } else {
            panic!("Expected Heading block");
        }
    }

    #[test]
    fn test_generate_heading_anchor_simple() {
        assert_eq!(generate_heading_anchor("Introduction"), "introduction");
        assert_eq!(
            generate_heading_anchor("Getting Started"),
            "getting-started"
        );
        assert_eq!(
            generate_heading_anchor("Chapter 1: Overview"),
            "chapter-1-overview"
        );
    }

    #[test]
    fn test_generate_heading_anchor_special_chars() {
        assert_eq!(generate_heading_anchor("What's New?"), "what-s-new");
        assert_eq!(
            generate_heading_anchor("  Leading & Trailing  "),
            "leading-trailing"
        );
        assert_eq!(generate_heading_anchor("API (v2.0)"), "api-v2-0");
    }

    #[test]
    fn test_parse_page_break() {
        let doc = parse("Before\n\n<<<\n\nAfter").unwrap();
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], Block::Paragraph(_)));
        assert!(matches!(doc.blocks[1], Block::Break(BreakType::Page)));
        assert!(matches!(doc.blocks[2], Block::Paragraph(_)));
    }

    #[test]
    fn test_parse_page_break_multiple() {
        let doc = parse("Page 1\n\n<<<\n\nPage 2\n\n<<<\n\nPage 3").unwrap();
        assert_eq!(doc.blocks.len(), 5);
        assert!(matches!(doc.blocks[1], Block::Break(BreakType::Page)));
        assert!(matches!(doc.blocks[3], Block::Break(BreakType::Page)));
    }

    #[test]
    fn test_parse_page_break_at_start() {
        let doc = parse("<<<\n\nContent").unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Break(BreakType::Page)));
    }

    #[test]
    fn test_parse_document_header_full() {
        let input = "= My Document\nAlan Baldassarre <alan@example.com>\nv1.0, 2025-02-09\n:doctype: book\n:toc: left\n:sectnums:\n:doc-title: Custom Title\n\n== Introduction\n\nSome text.";
        let doc = parse(input).unwrap();

        // Title parsed
        assert_eq!(doc.metadata.title, Some("My Document".to_string()));
        // Author parsed
        assert_eq!(doc.metadata.authors, vec!["Alan Baldassarre".to_string()]);
        // Revision parsed
        assert_eq!(doc.metadata.revision, Some("v1.0, 2025-02-09".to_string()));
        // Key-value attributes parsed
        assert_eq!(
            doc.metadata.attributes.get("doctype"),
            Some(&"book".to_string())
        );
        assert_eq!(
            doc.metadata.attributes.get("toc"),
            Some(&"left".to_string())
        );
        assert_eq!(
            doc.metadata.attributes.get("doc-title"),
            Some(&"Custom Title".to_string())
        );
        // Boolean attribute parsed
        assert_eq!(
            doc.metadata.attributes.get("sectnums"),
            Some(&String::new())
        );
        // No header content leaked into blocks
        assert_eq!(doc.blocks.len(), 2); // Heading + Paragraph
        assert!(matches!(doc.blocks[0], Block::Heading(_)));
    }

    #[test]
    fn test_parse_header_no_author() {
        // Title followed directly by blank line (no author/revision)
        let input = "= Report\n\nSome content.";
        let doc = parse(input).unwrap();
        assert_eq!(doc.metadata.title, Some("Report".to_string()));
        assert!(doc.metadata.authors.is_empty());
        assert!(doc.metadata.revision.is_none());
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_header_author_no_revision() {
        let input = "= Report\nJohn Doe <john@example.com>\n:doctype: article\n\nContent.";
        let doc = parse(input).unwrap();
        assert_eq!(doc.metadata.title, Some("Report".to_string()));
        assert_eq!(doc.metadata.authors, vec!["John Doe".to_string()]);
        assert!(doc.metadata.revision.is_none());
        assert_eq!(
            doc.metadata.attributes.get("doctype"),
            Some(&"article".to_string())
        );
        assert_eq!(doc.blocks.len(), 1); // just the paragraph
    }

    #[test]
    fn test_parse_header_multiple_authors() {
        let input = "= Report\nAlice <a@x.com>; Bob <b@x.com>\nv2.0\n\nContent.";
        let doc = parse(input).unwrap();
        assert_eq!(doc.metadata.authors.len(), 2);
        assert_eq!(doc.metadata.authors[0], "Alice");
        assert_eq!(doc.metadata.authors[1], "Bob");
        assert_eq!(doc.metadata.revision, Some("v2.0".to_string()));
    }

    #[test]
    fn test_parse_header_boolean_attributes() {
        let input = "= Doc\n:sectnums:\n:icons:\n:experimental:\n\nContent.";
        let doc = parse(input).unwrap();
        assert!(doc.metadata.attributes.contains_key("sectnums"));
        assert!(doc.metadata.attributes.contains_key("icons"));
        assert!(doc.metadata.attributes.contains_key("experimental"));
        // Boolean attrs should have empty string values
        assert_eq!(
            doc.metadata.attributes.get("sectnums"),
            Some(&String::new())
        );
    }

    #[test]
    fn test_ifdef_false_skips_content() {
        let input =
            "= Doc\n\nVisible.\n\nifdef::backend-pdf[]\nHidden text.\nendif::[]\n\nAlso visible.";
        let doc = parse(input).unwrap();
        // "Hidden text." should not appear in any block
        let all_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph(p) = b {
                    Some(
                        p.inlines
                            .iter()
                            .filter_map(|i| {
                                if let Inline::Text(t) = i {
                                    Some(t.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            })
            .collect();
        assert!(all_text.contains("Visible."));
        assert!(all_text.contains("Also visible."));
        assert!(!all_text.contains("Hidden"));
    }

    #[test]
    fn test_ifdef_true_includes_content() {
        let input = "= Doc\n:my-attr:\n\nifdef::my-attr[]\nIncluded text.\nendif::[]\n\nOther.";
        let doc = parse(input).unwrap();
        let all_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph(p) = b {
                    Some(
                        p.inlines
                            .iter()
                            .filter_map(|i| {
                                if let Inline::Text(t) = i {
                                    Some(t.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            })
            .collect();
        assert!(all_text.contains("Included text."));
        assert!(all_text.contains("Other."));
    }

    #[test]
    fn test_ifndef_false_includes_content() {
        // ifndef with a DEFINED attr → skip content
        let input = "= Doc\n:my-attr:\n\nifndef::my-attr[]\nSkipped.\nendif::[]\n\nVisible.";
        let doc = parse(input).unwrap();
        let all_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph(p) = b {
                    Some(
                        p.inlines
                            .iter()
                            .filter_map(|i| {
                                if let Inline::Text(t) = i {
                                    Some(t.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            })
            .collect();
        assert!(!all_text.contains("Skipped"));
        assert!(all_text.contains("Visible."));
    }

    #[test]
    fn test_comment_lines_skipped() {
        let input = "= Doc\n:doc-title: My Title\n// This is a comment\n:doc-status: DRAFT\n\nStatus is {doc-status}, title is {doc-title}.";
        let doc = parse(input).unwrap();
        // Both attributes should be captured (comment shouldn't break header)
        assert_eq!(
            doc.metadata.attributes.get("doc-title").unwrap(),
            "My Title"
        );
        assert_eq!(doc.metadata.attributes.get("doc-status").unwrap(), "DRAFT");
        // Comment should not appear in blocks
        let all_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph(p) = b {
                    Some(
                        p.inlines
                            .iter()
                            .filter_map(|i| {
                                if let Inline::Text(t) = i {
                                    Some(t.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            })
            .collect();
        assert!(!all_text.contains("comment"));
        assert!(all_text.contains("DRAFT"));
        assert!(all_text.contains("My Title"));
    }

    #[test]
    fn test_attribute_substitution_in_paragraph() {
        let input = "= Doc\n:doc-title: My Custom Title\n\nThe title is {doc-title}.";
        let doc = parse(input).unwrap();
        if let Block::Paragraph(p) = &doc.blocks[0] {
            let text: String = p
                .inlines
                .iter()
                .filter_map(|i| {
                    if let Inline::Text(t) = i {
                        Some(t.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            assert!(text.contains("My Custom Title"), "Got: {}", text);
            assert!(
                !text.contains("{doc-title}"),
                "Literal {{doc-title}} should be resolved"
            );
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_attribute_substitution_inside_bold() {
        let input = "= Doc\n:doc-status: DRAFT\n\n| Status | *{doc-status}*";
        let doc = parse(input).unwrap();
        // Find all text recursively in the document
        fn collect_text(inline: &Inline) -> String {
            match inline {
                Inline::Text(t) => t.clone(),
                Inline::Format(_, inner) => collect_text(inner),
                _ => String::new(),
            }
        }
        let all_text: String = doc
            .blocks
            .iter()
            .flat_map(|b| match b {
                Block::Table(t) => t
                    .rows
                    .iter()
                    .flat_map(|r| {
                        r.cells.iter().flat_map(|c| {
                            c.content.iter().filter_map(|b| {
                                if let Block::Paragraph(p) = b {
                                    Some(p.inlines.iter().map(collect_text).collect::<String>())
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .collect::<Vec<_>>(),
                Block::Paragraph(p) => vec![p.inlines.iter().map(collect_text).collect()],
                _ => vec![],
            })
            .collect();
        assert!(all_text.contains("DRAFT"), "Got: {}", all_text);
        assert!(
            !all_text.contains("{doc-status}"),
            "Attribute inside bold not resolved, got: {}",
            all_text
        );
    }

    #[test]
    fn test_attribute_substitution_undefined() {
        let input = "= Doc\n\nValue is {undefined-attr}.";
        let doc = parse(input).unwrap();
        if let Block::Paragraph(p) = &doc.blocks[0] {
            let text: String = p
                .inlines
                .iter()
                .filter_map(|i| {
                    if let Inline::Text(t) = i {
                        Some(t.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            assert!(
                text.contains("{undefined-attr}"),
                "Undefined attrs should be literal"
            );
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_attribute_substitution_in_table() {
        let input = "= Doc\n:org: Engineering S.p.A.\n\n|===\n| Company | {org}\n|===";
        let doc = parse(input).unwrap();
        if let Block::Table(t) = &doc.blocks[0] {
            let cell = &t.rows[0].cells[1];
            if let Block::Paragraph(p) = &cell.content[0] {
                let text: String = p
                    .inlines
                    .iter()
                    .filter_map(|i| {
                        if let Inline::Text(t) = i {
                            Some(t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(text.contains("Engineering S.p.A."), "Got: {}", text);
            }
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_asciidoc_cell_bullet_list() {
        let input = "|===\n| *Label*\na|\n* Item 1\n* Item 2\n* Item 3\n\n| *Other*\n|===";
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Table(t) = &doc.blocks[0] {
            // Should have 1 row with 2 cells
            assert!(!t.rows.is_empty(), "Table should have rows");
            let row = &t.rows[0];
            assert_eq!(row.cells.len(), 2, "Row should have 2 cells");

            // Second cell (a| cell) should contain a List block
            let a_cell = &row.cells[1];
            let has_list = a_cell.content.iter().any(|b| matches!(b, Block::List(_)));
            assert!(
                has_list,
                "a| cell should contain a List block, got: {:?}",
                a_cell
                    .content
                    .iter()
                    .map(|b| std::mem::discriminant(b))
                    .collect::<Vec<_>>()
            );
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_asciidoc_cell_inline_content() {
        // a| with content on the same line
        let input = "|===\n| Field\na| Some *bold* text\n|===";
        let doc = parse(input).unwrap();
        if let Block::Table(t) = &doc.blocks[0] {
            let row = &t.rows[0];
            assert_eq!(row.cells.len(), 2);
            // The a| cell should have parsed content
            assert!(!row.cells[1].content.is_empty());
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_asciidoc_cell_at_table_end() {
        // a| cell terminated by |=== (end of table)
        let input = "|===\n| Label\na|\n* Item A\n* Item B\n|===";
        let doc = parse(input).unwrap();
        if let Block::Table(t) = &doc.blocks[0] {
            let row = &t.rows[0];
            assert_eq!(row.cells.len(), 2);
            let a_cell = &row.cells[1];
            let has_list = a_cell.content.iter().any(|b| matches!(b, Block::List(_)));
            assert!(has_list, "a| cell terminated by |=== should contain list");
        } else {
            panic!("Expected table");
        }
    }
}
