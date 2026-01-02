//! Document extraction (docx → AsciiDoc)
//!
//! This module converts OOXML documents to AsciiDoc format,
//! preserving structure and generating style mappings.
//!
//! When extracting from a self-contained utf8dok DOCX, the extractor
//! prioritizes the embedded `utf8dok/source.adoc` over parsing the
//! document content (unless `force_parse` is set).

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use crate::archive::OoxmlArchive;
use crate::document::{Block, Document, Hyperlink, Paragraph, ParagraphChild, Run, Table};
use crate::error::Result;
use crate::relationships::Relationships;
use crate::style_map::{
    classify_bookmark, normalize_heading_to_anchor, AnchorMapping, AnchorType, HyperlinkMapping,
    ParagraphStyleMapping, StyleContract,
};
use crate::styles::StyleSheet;

/// Parsed comments from word/comments.xml
#[derive(Debug, Default)]
pub struct Comments {
    /// Map of comment ID to comment text
    comments: HashMap<u32, String>,
}

impl Comments {
    /// Parse comments from XML
    pub fn parse(xml: &[u8]) -> Self {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut comments = HashMap::new();
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut current_id: Option<u32> = None;
        let mut current_text = String::new();
        let mut in_comment = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.local_name();
                    if name.as_ref() == b"comment" {
                        // Get comment ID
                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            if attr.key.as_ref() == b"w:id" || attr.key.as_ref() == b"id" {
                                if let Ok(val) = String::from_utf8(attr.value.to_vec()) {
                                    current_id = val.parse().ok();
                                    in_comment = true;
                                    current_text.clear();
                                }
                            }
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if in_comment {
                        if let Ok(text) = e.unescape() {
                            current_text.push_str(&text);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.local_name();
                    if name.as_ref() == b"comment" {
                        if let Some(id) = current_id.take() {
                            comments.insert(id, current_text.clone());
                        }
                        in_comment = false;
                        current_text.clear();
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        Comments { comments }
    }

    /// Get comment text by ID
    pub fn get(&self, id: u32) -> Option<&str> {
        self.comments.get(&id).map(|s| s.as_str())
    }

    /// Extract language from a comment if it matches "Language: XXX"
    pub fn get_language(&self, id: u32) -> Option<String> {
        self.get(id).and_then(|text| {
            let text = text.trim();
            if text.starts_with("Language:") {
                Some(text.trim_start_matches("Language:").trim().to_string())
            } else {
                None
            }
        })
    }
}

/// Parsed comment ranges from document.xml
///
/// Maps paragraph indices to comment IDs that wrap them
#[derive(Debug, Default)]
pub struct CommentRanges {
    /// Map of block index to comment IDs that contain it
    ranges: HashMap<usize, Vec<u32>>,
}

impl CommentRanges {
    /// Parse comment ranges from document XML
    ///
    /// This scans for commentRangeStart/End elements and tracks which
    /// blocks contain them. Note: commentRangeStart often appears INSIDE
    /// a paragraph element, not before it.
    pub fn parse(xml: &[u8]) -> Self {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut ranges = HashMap::new();
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut in_body = false;
        let mut in_paragraph = false;
        let mut block_index: usize = 0;
        let mut current_para_comments: Vec<u32> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"body" => in_body = true,
                        b"p" if in_body => {
                            in_paragraph = true;
                            current_para_comments.clear();
                        }
                        b"tbl" if in_body => {
                            // Tables are handled separately
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"body" => in_body = false,
                        b"p" if in_body => {
                            // Record any comments found within this paragraph
                            if !current_para_comments.is_empty() {
                                ranges.insert(block_index, current_para_comments.clone());
                            }
                            in_paragraph = false;
                            current_para_comments.clear();
                            block_index += 1;
                        }
                        b"tbl" if in_body => {
                            block_index += 1;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"commentRangeStart" if in_paragraph => {
                            // Get comment ID - this comment applies to current paragraph
                            for attr in e.attributes().filter_map(|a| a.ok()) {
                                if attr.key.as_ref() == b"w:id" || attr.key.as_ref() == b"id" {
                                    if let Ok(val) = String::from_utf8(attr.value.to_vec()) {
                                        if let Ok(id) = val.parse::<u32>() {
                                            current_para_comments.push(id);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        CommentRanges { ranges }
    }

    /// Get comment IDs for a block index
    pub fn get_comment_ids(&self, block_index: usize) -> Option<&Vec<u32>> {
        self.ranges.get(&block_index)
    }
}

/// Indicates the origin of the extracted AsciiDoc content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOrigin {
    /// Content was extracted from embedded `utf8dok/source.adoc`
    Embedded,
    /// Content was parsed/generated from document.xml
    Parsed,
}

/// Result of extracting a document
#[derive(Debug)]
pub struct ExtractedDocument {
    /// The generated AsciiDoc content
    pub asciidoc: String,
    /// The detected style mappings (for utf8dok.toml) - legacy format
    pub style_mappings: StyleMappings,
    /// Style contract for round-trip fidelity (ADR-007)
    pub style_contract: StyleContract,
    /// Document metadata extracted from properties
    pub metadata: DocumentMetadata,
    /// Indicates where the AsciiDoc content came from
    pub source_origin: SourceOrigin,
}

/// Style mappings detected from the document
#[derive(Debug, Default)]
pub struct StyleMappings {
    /// Heading style IDs mapped to levels
    pub headings: Vec<(u8, String)>,
    /// Normal/body text style
    pub paragraph: Option<String>,
    /// Table styles
    pub tables: Vec<String>,
    /// List styles
    pub lists: Vec<String>,
    /// Code/monospace styles
    pub code: Vec<String>,
}

/// Document metadata
#[derive(Debug, Default, Clone)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub revision: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
}

impl DocumentMetadata {
    /// Parse core properties from docProps/core.xml
    pub fn parse(xml: &[u8]) -> Self {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut metadata = DocumentMetadata::default();
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut current_element = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_element = name;
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        match current_element.as_str() {
                            "dc:title" => metadata.title = Some(text),
                            "dc:creator" => metadata.author = Some(text),
                            "dc:subject" => metadata.subject = Some(text),
                            "cp:keywords" => metadata.keywords = Some(text),
                            "cp:revision" => metadata.revision = Some(text),
                            "dcterms:created" => metadata.created = Some(text),
                            "dcterms:modified" => metadata.modified = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        metadata
    }

    /// Generate AsciiDoc document header attributes
    pub fn to_asciidoc_header(&self) -> String {
        let mut header = String::new();

        if let Some(ref author) = self.author {
            if !author.is_empty() {
                writeln!(header, ":author: {}", author).unwrap();
            }
        }

        // Use modified date as revdate if available
        if let Some(ref modified) = self.modified {
            // Extract just the date part (YYYY-MM-DD from ISO format)
            let date_part = modified.split('T').next().unwrap_or(modified);
            writeln!(header, ":revdate: {}", date_part).unwrap();
        }

        header
    }
}

/// Extracts OOXML documents to AsciiDoc
pub struct AsciiDocExtractor {
    /// Include document attributes header
    pub include_header: bool,
    /// Detect and convert tables
    pub extract_tables: bool,
    /// Preserve inline formatting (bold, italic)
    pub preserve_formatting: bool,
    /// Force parsing document.xml even if embedded source exists
    pub force_parse: bool,
}

impl Default for AsciiDocExtractor {
    fn default() -> Self {
        Self {
            include_header: true,
            extract_tables: true,
            preserve_formatting: true,
            force_parse: false,
        }
    }
}

impl AsciiDocExtractor {
    /// Create a new extractor with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to force parsing document.xml even if embedded source exists
    pub fn with_force_parse(mut self, force: bool) -> Self {
        self.force_parse = force;
        self
    }

    /// Extract a document from a file path
    pub fn extract_file<P: AsRef<Path>>(&self, path: P) -> Result<ExtractedDocument> {
        let source_file = path
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        let archive = OoxmlArchive::open(path)?;
        self.extract_archive_with_source(&archive, source_file)
    }

    /// Extract from an already-opened archive
    ///
    /// If the archive contains embedded utf8dok source (from a previous render),
    /// that source is returned directly unless `force_parse` is set.
    pub fn extract_archive(&self, archive: &OoxmlArchive) -> Result<ExtractedDocument> {
        self.extract_archive_with_source(archive, None)
    }

    /// Extract from an already-opened archive with optional source file name
    ///
    /// If the archive contains embedded utf8dok source (from a previous render),
    /// that source is returned directly unless `force_parse` is set.
    pub fn extract_archive_with_source(
        &self,
        archive: &OoxmlArchive,
        source_file: Option<String>,
    ) -> Result<ExtractedDocument> {
        // Check for embedded source first (unless force_parse is set)
        if !self.force_parse {
            if let Ok(Some(embedded_source)) = archive.read_utf8dok_string("source.adoc") {
                // Parse styles for style mappings even when using embedded source
                let styles = StyleSheet::parse(archive.styles_xml()?)?;
                let style_mappings = self.detect_style_mappings(&styles);
                let style_contract =
                    self.build_style_contract_from_stylesheet(&styles, source_file);
                let metadata = DocumentMetadata::default();

                return Ok(ExtractedDocument {
                    asciidoc: embedded_source,
                    style_mappings,
                    style_contract,
                    metadata,
                    source_origin: SourceOrigin::Embedded,
                });
            }
        }

        // Parse document.xml and generate AsciiDoc
        let doc_xml = archive.document_xml()?;
        let document = Document::parse(doc_xml)?;
        let styles = StyleSheet::parse(archive.styles_xml()?)?;

        // Load relationships for hyperlink resolution
        let relationships = archive
            .document_rels_xml()
            .and_then(|xml| Relationships::parse(xml).ok());

        let style_mappings = self.detect_style_mappings(&styles);

        // Parse document metadata from docProps/core.xml
        let metadata = archive
            .core_properties_xml()
            .map(DocumentMetadata::parse)
            .unwrap_or_default();

        // Parse comments for code block language preservation
        let comments = archive
            .comments_xml()
            .map(Comments::parse)
            .unwrap_or_default();

        // Parse comment ranges from document.xml
        let comment_ranges = CommentRanges::parse(doc_xml);

        // Build the style contract (ADR-007)
        let style_contract =
            self.build_style_contract(&document, &styles, relationships.as_ref(), source_file);

        let asciidoc = self.convert_to_asciidoc(
            &document,
            &styles,
            relationships.as_ref(),
            &metadata,
            &comments,
            &comment_ranges,
        );

        Ok(ExtractedDocument {
            asciidoc,
            style_mappings,
            style_contract,
            metadata,
            source_origin: SourceOrigin::Parsed,
        })
    }

    /// Detect style mappings from the stylesheet
    fn detect_style_mappings(&self, styles: &StyleSheet) -> StyleMappings {
        let mut mappings = StyleMappings::default();

        // Find heading styles
        for style in styles.heading_styles() {
            if let Some(level) = style.outline_level {
                mappings.headings.push((level + 1, style.id.clone()));
            }
        }
        mappings.headings.sort_by_key(|(level, _)| *level);

        // Find default paragraph style
        mappings.paragraph = styles.default_paragraph.clone();

        // Find table styles
        for style in styles.table_styles() {
            mappings.tables.push(style.id.clone());
        }

        mappings
    }

    /// Build a StyleContract from document, styles, and relationships (ADR-007)
    fn build_style_contract(
        &self,
        document: &Document,
        styles: &StyleSheet,
        rels: Option<&Relationships>,
        source_file: Option<String>,
    ) -> StyleContract {
        let mut contract = if let Some(ref src) = source_file {
            StyleContract::with_source(src)
        } else {
            StyleContract::new()
        };

        // Add paragraph style mappings from stylesheet
        self.add_paragraph_styles(&mut contract, styles);

        // Extract anchors/bookmarks from document
        self.extract_anchors(&mut contract, document, styles);

        // Extract hyperlink mappings
        self.extract_hyperlinks(&mut contract, document, rels);

        contract
    }

    /// Build a minimal StyleContract from stylesheet only (for embedded sources)
    fn build_style_contract_from_stylesheet(
        &self,
        styles: &StyleSheet,
        source_file: Option<String>,
    ) -> StyleContract {
        let mut contract = if let Some(ref src) = source_file {
            StyleContract::with_source(src)
        } else {
            StyleContract::new()
        };

        self.add_paragraph_styles(&mut contract, styles);
        contract
    }

    /// Add paragraph style mappings to the contract
    fn add_paragraph_styles(&self, contract: &mut StyleContract, styles: &StyleSheet) {
        // Map heading styles
        for style in styles.heading_styles() {
            if let Some(level) = style.outline_level {
                contract.add_paragraph_style(
                    &style.id,
                    ParagraphStyleMapping {
                        role: format!("h{}", level + 1),
                        heading_level: Some(level + 1),
                        is_list: false,
                        list_type: None,
                        based_on: style.based_on.clone(),
                    },
                );
            }
        }

        // Map default paragraph style
        if let Some(ref para_style) = styles.default_paragraph {
            contract.add_paragraph_style(
                para_style,
                ParagraphStyleMapping {
                    role: "body".into(),
                    heading_level: None,
                    is_list: false,
                    list_type: None,
                    based_on: None,
                },
            );
        }

        // Add common style mappings
        contract.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "body".into(),
                heading_level: None,
                is_list: false,
                list_type: None,
                based_on: None,
            },
        );
    }

    /// Extract anchor/bookmark mappings from the document
    fn extract_anchors(
        &self,
        contract: &mut StyleContract,
        document: &Document,
        styles: &StyleSheet,
    ) {
        // Track heading text for anchor semantic ID generation
        let mut current_heading_text: Option<String> = None;

        for block in &document.blocks {
            if let Block::Paragraph(para) = block {
                // Check if this is a heading - capture its text for anchor normalization
                if let Some(ref style_id) = para.style_id {
                    if styles.heading_level(style_id).is_some() {
                        current_heading_text = Some(self.get_raw_paragraph_text(para));
                    }
                }

                // Extract bookmarks from paragraph children
                for child in &para.children {
                    if let ParagraphChild::Bookmark(bookmark) = child {
                        let anchor_type = classify_bookmark(&bookmark.name);

                        // Generate semantic ID based on anchor type
                        let semantic_id = match anchor_type {
                            AnchorType::Toc | AnchorType::Heading => {
                                // Use current heading text if available
                                current_heading_text
                                    .as_ref()
                                    .map(|h| normalize_heading_to_anchor(h))
                                    .unwrap_or_else(|| bookmark.name.clone())
                            }
                            AnchorType::Reference => {
                                // Keep ref prefix for reference anchors
                                format!("ref-{}", bookmark.name.trim_start_matches("_Ref"))
                            }
                            AnchorType::Highlight => {
                                // Skip highlight anchors - they're not semantically meaningful
                                continue;
                            }
                            AnchorType::UserDefined => {
                                // User-defined bookmarks keep their name
                                normalize_heading_to_anchor(&bookmark.name)
                            }
                        };

                        contract.add_anchor(
                            &bookmark.name,
                            AnchorMapping {
                                semantic_id,
                                anchor_type,
                                target_heading: current_heading_text.clone(),
                                original_bookmark: Some(bookmark.name.clone()),
                            },
                        );
                    }
                }
            }
        }
    }

    /// Extract hyperlink mappings from the document
    fn extract_hyperlinks(
        &self,
        contract: &mut StyleContract,
        document: &Document,
        rels: Option<&Relationships>,
    ) {
        let mut link_counter = 0u32;

        for block in &document.blocks {
            if let Block::Paragraph(para) = block {
                for child in &para.children {
                    if let ParagraphChild::Hyperlink(hyperlink) = child {
                        link_counter += 1;
                        let link_id = format!("link{}", link_counter);

                        // Determine if external or internal
                        let is_external = hyperlink.id.is_some();
                        let url = if is_external {
                            hyperlink
                                .id
                                .as_ref()
                                .and_then(|id| rels.and_then(|r| r.get(id)))
                                .map(|s| s.to_string())
                        } else {
                            None
                        };
                        let anchor_target = hyperlink.anchor.clone();

                        contract.add_hyperlink(
                            &link_id,
                            HyperlinkMapping {
                                is_external,
                                url,
                                anchor_target,
                                original_rel_id: hyperlink.id.clone(),
                                original_anchor: hyperlink.anchor.clone(),
                            },
                        );
                    }
                }
            }

            // Also check table cells for hyperlinks
            if let Block::Table(table) = block {
                for row in &table.rows {
                    for cell in &row.cells {
                        for para in &cell.paragraphs {
                            for child in &para.children {
                                if let ParagraphChild::Hyperlink(hyperlink) = child {
                                    link_counter += 1;
                                    let link_id = format!("link{}", link_counter);

                                    let is_external = hyperlink.id.is_some();
                                    let url = if is_external {
                                        hyperlink
                                            .id
                                            .as_ref()
                                            .and_then(|id| rels.and_then(|r| r.get(id)))
                                            .map(|s| s.to_string())
                                    } else {
                                        None
                                    };
                                    let anchor_target = hyperlink.anchor.clone();

                                    contract.add_hyperlink(
                                        &link_id,
                                        HyperlinkMapping {
                                            is_external,
                                            url,
                                            anchor_target,
                                            original_rel_id: hyperlink.id.clone(),
                                            original_anchor: hyperlink.anchor.clone(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Convert document to AsciiDoc string
    fn convert_to_asciidoc(
        &self,
        document: &Document,
        styles: &StyleSheet,
        rels: Option<&Relationships>,
        metadata: &DocumentMetadata,
        comments: &Comments,
        comment_ranges: &CommentRanges,
    ) -> String {
        let mut output = String::new();
        let mut title_written = false;
        let mut last_was_list = false;
        let mut last_num_id: Option<u32> = None;
        let mut block_index: usize = 0;

        // If we have a title from docProps, use it as the document title
        if self.include_header {
            if let Some(ref title) = metadata.title {
                writeln!(output, "= {}", title).unwrap();
                // Add document metadata attributes after title
                let header_attrs = metadata.to_asciidoc_header();
                if !header_attrs.is_empty() {
                    output.push_str(&header_attrs);
                }
                writeln!(output).unwrap();
                title_written = true;
            }
        }

        for block in &document.blocks {
            match block {
                Block::Paragraph(para) => {
                    if para.is_empty() {
                        block_index += 1;
                        continue;
                    }

                    let text = self.convert_paragraph_with_rels(para, rels);

                    // Check if this is a heading
                    if let Some(ref style_id) = para.style_id {
                        if let Some(level) = styles.heading_level(style_id) {
                            // End any list before heading (add blank line)
                            if last_was_list {
                                writeln!(output).unwrap();
                                last_was_list = false;
                                last_num_id = None;
                            }
                            // If no title was written and this is level 1, use as title
                            if !title_written && level == 1 && self.include_header {
                                writeln!(output, "= {}", text.trim()).unwrap();
                                // Add document metadata attributes after title
                                let header_attrs = metadata.to_asciidoc_header();
                                if !header_attrs.is_empty() {
                                    output.push_str(&header_attrs);
                                }
                                writeln!(output).unwrap();
                                title_written = true;
                            } else {
                                let prefix = "=".repeat(level as usize + 1);
                                writeln!(output, "{} {}", prefix, text.trim()).unwrap();
                                writeln!(output).unwrap();
                            }
                            block_index += 1;
                            continue;
                        }

                        // Check for code block style
                        let style_lower = style_id.to_lowercase();
                        if style_lower.contains("code") || style_lower.contains("source") {
                            if last_was_list {
                                writeln!(output).unwrap();
                                last_was_list = false;
                                last_num_id = None;
                            }
                            // Check for language from comment
                            let lang = self.get_language_from_comment(
                                block_index,
                                comments,
                                comment_ranges,
                            );
                            if let Some(ref lang) = lang {
                                writeln!(output, "[source,{}]", lang).unwrap();
                            } else {
                                writeln!(output, "[source]").unwrap();
                            }
                            writeln!(output, "----").unwrap();
                            writeln!(output, "{}", text.trim()).unwrap();
                            writeln!(output, "----").unwrap();
                            writeln!(output).unwrap();
                            block_index += 1;
                            continue;
                        }
                    }

                    // Check if this is a multi-line monospace paragraph (code block)
                    // This catches code blocks that use a template-specific style
                    if self.is_code_block_paragraph(para) {
                        if last_was_list {
                            writeln!(output).unwrap();
                            last_was_list = false;
                            last_num_id = None;
                        }
                        // Get raw text without formatting marks for code blocks
                        let raw_text = self.get_raw_paragraph_text(para);
                        // Check for language from comment
                        let lang =
                            self.get_language_from_comment(block_index, comments, comment_ranges);
                        if let Some(ref lang) = lang {
                            writeln!(output, "[source,{}]", lang).unwrap();
                        } else {
                            writeln!(output, "[source]").unwrap();
                        }
                        writeln!(output, "----").unwrap();
                        writeln!(output, "{}", raw_text.trim()).unwrap();
                        writeln!(output, "----").unwrap();
                        writeln!(output).unwrap();
                        block_index += 1;
                        continue;
                    }

                    // Check if this is a list item
                    if let Some(ref numbering) = para.numbering {
                        let is_new_list = last_num_id != Some(numbering.num_id);

                        // Add blank line before new list
                        if is_new_list && !last_was_list {
                            // Already have blank line from previous paragraph
                        }

                        // Determine list marker based on style
                        // NumId 1-9 are typically bullet lists, 10+ are numbered
                        // Also check if the numbering ilvl > 0 for nested items
                        let indent = "*".repeat((numbering.ilvl + 1) as usize);

                        // Check if this looks like a numbered list (could improve with numbering.xml)
                        let marker = if self.is_numbered_list(numbering.num_id, styles) {
                            ".".repeat((numbering.ilvl + 1) as usize)
                        } else {
                            indent
                        };

                        writeln!(output, "{} {}", marker, text.trim()).unwrap();
                        last_was_list = true;
                        last_num_id = Some(numbering.num_id);
                        block_index += 1;
                        continue;
                    }

                    // End list if we hit a non-list paragraph
                    if last_was_list {
                        writeln!(output).unwrap();
                        last_was_list = false;
                        last_num_id = None;
                    }

                    // Regular paragraph
                    if !text.trim().is_empty() {
                        writeln!(output, "{}", text.trim()).unwrap();
                        writeln!(output).unwrap();
                    }
                    block_index += 1;
                }
                Block::Table(table) if self.extract_tables => {
                    if last_was_list {
                        writeln!(output).unwrap();
                        last_was_list = false;
                        last_num_id = None;
                    }
                    let table_text = self.convert_table(table);
                    // table_text already ends with newline from |===
                    output.push_str(&table_text);
                    writeln!(output).unwrap();
                    block_index += 1;
                }
                Block::Table(_) => {
                    writeln!(output, "// [TABLE OMITTED]").unwrap();
                    writeln!(output).unwrap();
                    block_index += 1;
                }
                Block::SectionBreak => {
                    if last_was_list {
                        writeln!(output).unwrap();
                        last_was_list = false;
                        last_num_id = None;
                    }
                    writeln!(output, "'''").unwrap();
                    writeln!(output).unwrap();
                    // SectionBreak doesn't increment block_index as it's not a block-level element
                }
            }
        }

        output
    }

    /// Get language from comment for a code block at a given index
    fn get_language_from_comment(
        &self,
        block_index: usize,
        comments: &Comments,
        comment_ranges: &CommentRanges,
    ) -> Option<String> {
        // Check if this block has any associated comments
        if let Some(comment_ids) = comment_ranges.get_comment_ids(block_index) {
            for &comment_id in comment_ids {
                if let Some(lang) = comments.get_language(comment_id) {
                    return Some(lang);
                }
            }
        }
        None
    }

    /// Check if a numbering ID represents a numbered list
    ///
    /// This uses a simple heuristic based on our writer's convention:
    /// - numId 1 = unordered (bullet) list
    /// - numId 2 = ordered (numbered) list
    ///
    /// A more complete implementation would parse numbering.xml to check
    /// the numFmt value (bullet vs decimal/lowerLetter/etc.)
    fn is_numbered_list(&self, num_id: u32, _styles: &StyleSheet) -> bool {
        // Our writer uses numId 2 for ordered lists
        num_id == 2
    }

    /// Get raw text from a paragraph without any formatting marks
    fn get_raw_paragraph_text(&self, para: &Paragraph) -> String {
        let mut result = String::new();

        for child in &para.children {
            match child {
                ParagraphChild::Run(run) => {
                    result.push_str(&run.text);
                }
                ParagraphChild::Hyperlink(hyperlink) => {
                    for run in &hyperlink.runs {
                        result.push_str(&run.text);
                    }
                }
                ParagraphChild::Image(img) => {
                    // Include alt text for images
                    if let Some(alt) = &img.alt {
                        result.push_str(alt);
                    }
                }
                ParagraphChild::Bookmark(_) => {
                    // Bookmarks have no text content
                }
            }
        }

        result
    }

    /// Check if a paragraph is a code block (multi-line monospace content)
    ///
    /// This detects code blocks that:
    /// 1. Consist entirely of monospace-formatted runs
    /// 2. Contain newlines (multi-line content)
    fn is_code_block_paragraph(&self, para: &Paragraph) -> bool {
        // Get all runs from the paragraph
        let mut has_monospace = false;
        let mut has_newline = false;
        let mut all_monospace = true;

        for child in &para.children {
            if let ParagraphChild::Run(run) = child {
                if !run.text.is_empty() {
                    if run.monospace {
                        has_monospace = true;
                        if run.text.contains('\n') {
                            has_newline = true;
                        }
                    } else {
                        // Non-monospace text present
                        all_monospace = false;
                    }
                }
            }
        }

        // It's a code block if all text is monospace and has newlines
        has_monospace && has_newline && all_monospace
    }

    /// Convert a paragraph to AsciiDoc text
    fn convert_paragraph(&self, para: &Paragraph) -> String {
        self.convert_paragraph_with_rels(para, None)
    }

    /// Convert a paragraph to AsciiDoc text with relationship resolution
    fn convert_paragraph_with_rels(
        &self,
        para: &Paragraph,
        rels: Option<&Relationships>,
    ) -> String {
        let mut result = String::new();

        // Collect and merge consecutive runs with the same formatting
        let mut merged_runs: Vec<Run> = Vec::new();

        for child in &para.children {
            match child {
                ParagraphChild::Run(run) => {
                    // Try to merge with previous run if formatting matches
                    if let Some(last) = merged_runs.last_mut() {
                        if last.bold == run.bold
                            && last.italic == run.italic
                            && last.monospace == run.monospace
                        {
                            // Same formatting - merge text
                            last.text.push_str(&run.text);
                        } else {
                            // Different formatting - convert previous runs and start new
                            for merged in merged_runs.drain(..) {
                                result.push_str(&self.convert_run(&merged));
                            }
                            merged_runs.push(run.clone());
                        }
                    } else {
                        merged_runs.push(run.clone());
                    }
                }
                ParagraphChild::Hyperlink(hyperlink) => {
                    // Flush any pending merged runs before hyperlink
                    for merged in merged_runs.drain(..) {
                        result.push_str(&self.convert_run(&merged));
                    }
                    result.push_str(&self.convert_hyperlink(hyperlink, rels));
                }
                ParagraphChild::Image(img) => {
                    // Flush any pending merged runs before image
                    for merged in merged_runs.drain(..) {
                        result.push_str(&self.convert_run(&merged));
                    }
                    result.push_str(&self.convert_image(img, rels));
                }
                ParagraphChild::Bookmark(bookmark) => {
                    // Flush any pending merged runs before bookmark
                    for merged in merged_runs.drain(..) {
                        result.push_str(&self.convert_run(&merged));
                    }
                    // Output AsciiDoc anchor
                    result.push_str(&format!("[[{}]]", bookmark.name));
                }
            }
        }

        // Flush any remaining merged runs
        for merged in merged_runs {
            result.push_str(&self.convert_run(&merged));
        }

        result
    }

    /// Convert an image to AsciiDoc image macro
    fn convert_image(&self, img: &crate::image::Image, rels: Option<&Relationships>) -> String {
        // Resolve target path from relationship ID
        let target = if let Some(rels) = rels {
            rels.get(&img.rel_id)
                .map(|t| format!("media/{}", t.rsplit('/').next().unwrap_or(t)))
                .unwrap_or_else(|| img.target.clone())
        } else if !img.target.is_empty() {
            img.target.clone()
        } else {
            format!("media/image{}.png", img.id)
        };

        // Build attributes
        let mut attrs = Vec::new();

        // Alt text first
        if let Some(alt) = &img.alt {
            attrs.push(alt.clone());
        }

        // Dimensions
        if let Some(width_emu) = img.width_emu {
            let width_px = crate::image::emu_to_pixels(width_emu);
            attrs.push(format!("width={}", width_px));
        }
        if let Some(height_emu) = img.height_emu {
            let height_px = crate::image::emu_to_pixels(height_emu);
            attrs.push(format!("height={}", height_px));
        }

        let attrs_str = if attrs.is_empty() {
            String::new()
        } else {
            format!("[{}]", attrs.join(","))
        };

        format!("image::{}{}\n", target, attrs_str)
    }

    /// Convert a run to AsciiDoc text
    fn convert_run(&self, run: &Run) -> String {
        let text = &run.text;

        if !self.preserve_formatting {
            return text.clone();
        }

        // Apply formatting
        if run.bold && run.italic {
            format!("*_{}*_", text)
        } else if run.bold {
            format!("*{}*", text)
        } else if run.italic {
            format!("_{}_", text)
        } else if run.monospace {
            format!("`{}`", text)
        } else {
            text.clone()
        }
    }

    /// Merge consecutive runs with the same formatting and convert to AsciiDoc
    fn merge_and_convert_runs(&self, runs: &[Run]) -> String {
        let mut result = String::new();
        let mut merged_runs: Vec<Run> = Vec::new();

        for run in runs {
            if let Some(last) = merged_runs.last_mut() {
                if last.bold == run.bold
                    && last.italic == run.italic
                    && last.monospace == run.monospace
                {
                    // Same formatting - merge text
                    last.text.push_str(&run.text);
                } else {
                    // Different formatting - convert previous runs and start new
                    for merged in merged_runs.drain(..) {
                        result.push_str(&self.convert_run(&merged));
                    }
                    merged_runs.push(run.clone());
                }
            } else {
                merged_runs.push(run.clone());
            }
        }

        // Flush any remaining merged runs
        for merged in merged_runs {
            result.push_str(&self.convert_run(&merged));
        }

        result
    }

    /// Convert a hyperlink to AsciiDoc format
    fn convert_hyperlink(&self, hyperlink: &Hyperlink, rels: Option<&Relationships>) -> String {
        // Get the link text from the runs, merging consecutive runs with same formatting
        let text = self.merge_and_convert_runs(&hyperlink.runs);

        // Resolve the target URL
        let target = if let Some(ref id) = hyperlink.id {
            // External link - look up in relationships
            rels.and_then(|r| r.get(id))
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("#{}", id))
        } else if let Some(ref anchor) = hyperlink.anchor {
            // Internal anchor link
            format!("#{}", anchor)
        } else {
            "#".to_string()
        };

        // Format as AsciiDoc link
        if target.starts_with('#') {
            // Internal anchor: <<anchor,text>>
            let anchor = target.trim_start_matches('#');
            format!("<<{},{}>>", anchor, text)
        } else {
            // External link: url[text]
            format!("{}[{}]", target, text)
        }
    }

    /// Convert a table to AsciiDoc format
    fn convert_table(&self, table: &Table) -> String {
        let mut output = String::new();

        // Determine column count
        let col_count = table.rows.first().map(|r| r.cells.len()).unwrap_or(0);

        if col_count == 0 {
            return output;
        }

        // Table header with proportional columns
        let col_spec = (0..col_count).map(|_| "1").collect::<Vec<_>>().join(",");
        writeln!(output, "[cols=\"{}\",options=\"header\"]", col_spec).unwrap();
        writeln!(output, "|===").unwrap();

        for (row_idx, row) in table.rows.iter().enumerate() {
            // Collect cell contents
            let cells: Vec<String> = row
                .cells
                .iter()
                .map(|cell| {
                    cell.paragraphs
                        .iter()
                        .map(|p| self.convert_paragraph(p))
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                })
                .collect();

            // Output all cells on one line (AsciiDoc compact table format)
            let row_text = cells.iter().map(|c| format!("|{}", c)).collect::<String>();
            writeln!(output, "{}", row_text).unwrap();

            // Blank line after header row for AsciiDoc table syntax
            if row_idx == 0 {
                writeln!(output).unwrap();
            }
        }

        writeln!(output, "|===").unwrap();

        output
    }
}

impl StyleMappings {
    /// Generate TOML configuration content
    pub fn to_toml(&self) -> String {
        let mut output = String::new();

        writeln!(output, "[styles]").unwrap();

        for (level, id) in &self.headings {
            writeln!(output, "heading{} = \"{}\"", level, id).unwrap();
        }

        if let Some(ref para) = self.paragraph {
            writeln!(output, "paragraph = \"{}\"", para).unwrap();
        }

        if !self.tables.is_empty() {
            writeln!(output, "table = \"{}\"", self.tables[0]).unwrap();
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_simple_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>Hello, world!</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let styles = StyleSheet::default();
        let metadata = DocumentMetadata::default();
        let comments = Comments::default();
        let comment_ranges = CommentRanges::default();

        let extractor = AsciiDocExtractor::new();
        let asciidoc = extractor.convert_to_asciidoc(
            &doc,
            &styles,
            None,
            &metadata,
            &comments,
            &comment_ranges,
        );

        assert!(asciidoc.contains("Hello, world!"));
    }

    #[test]
    fn test_style_mappings_to_toml() {
        let mappings = StyleMappings {
            headings: vec![(1, "Heading1".to_string()), (2, "Heading2".to_string())],
            paragraph: Some("Normal".to_string()),
            ..Default::default()
        };

        let toml = mappings.to_toml();
        assert!(toml.contains("heading1 = \"Heading1\""));
        assert!(toml.contains("heading2 = \"Heading2\""));
        assert!(toml.contains("paragraph = \"Normal\""));
    }

    #[test]
    fn test_extract_hyperlink_internal() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:hyperlink w:anchor="_Toc123">
                        <w:r><w:t>Click me</w:t></w:r>
                    </w:hyperlink>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let styles = StyleSheet::default();
        let metadata = DocumentMetadata::default();
        let comments = Comments::default();
        let comment_ranges = CommentRanges::default();

        let extractor = AsciiDocExtractor::new();
        let asciidoc = extractor.convert_to_asciidoc(
            &doc,
            &styles,
            None,
            &metadata,
            &comments,
            &comment_ranges,
        );

        println!("Generated AsciiDoc:\n{}", asciidoc);
        // Should generate: <<_Toc123,Click me>>
        assert!(
            asciidoc.contains("<<_Toc123,Click me>>"),
            "Expected <<_Toc123,Click me>> but got: {}",
            asciidoc
        );
    }

    #[test]
    fn test_comments_parse() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>Language: bash</w:t></w:r></w:p>
            </w:comment>
            <w:comment w:id="1" w:author="utf8dok">
                <w:p><w:r><w:t>Language: python</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);

        assert_eq!(comments.get(0), Some("Language: bash"));
        assert_eq!(comments.get(1), Some("Language: python"));
        assert_eq!(comments.get_language(0), Some("bash".to_string()));
        assert_eq!(comments.get_language(1), Some("python".to_string()));
        assert_eq!(comments.get(2), None);
    }

    #[test]
    fn test_comment_ranges_parse() {
        // Note: commentRangeStart appears INSIDE the paragraph in OOXML
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>First paragraph</w:t></w:r></w:p>
                <w:p>
                    <w:commentRangeStart w:id="0"/>
                    <w:r><w:t>Code block</w:t></w:r>
                    <w:commentRangeEnd w:id="0"/>
                </w:p>
                <w:p><w:r><w:t>Third paragraph</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let ranges = CommentRanges::parse(xml);

        // First paragraph (index 0) has no comment
        assert!(ranges.get_comment_ids(0).is_none());
        // Second paragraph (index 1) has comment 0
        assert_eq!(ranges.get_comment_ids(1), Some(&vec![0u32]));
        // Third paragraph (index 2) has no comment
        assert!(ranges.get_comment_ids(2).is_none());
    }

    // ==================== Sprint 5: Edge Case Tests ====================

    #[test]
    fn test_comments_empty_xml() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        </w:comments>"#;

        let comments = Comments::parse(xml);
        assert!(comments.get(0).is_none());
        assert!(comments.get_language(0).is_none());
    }

    #[test]
    fn test_comments_no_language_prefix() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0">
                <w:p><w:r><w:t>Just a regular comment</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        assert_eq!(comments.get(0), Some("Just a regular comment"));
        assert_eq!(comments.get_language(0), None); // No "Language:" prefix
    }

    #[test]
    fn test_comments_malformed_id() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="not_a_number">
                <w:p><w:r><w:t>Bad ID</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        // Malformed ID should be skipped gracefully
        assert!(comments.get(0).is_none());
    }

    #[test]
    fn test_comment_ranges_empty_document() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body></w:body>
        </w:document>"#;

        let ranges = CommentRanges::parse(xml);
        assert!(ranges.get_comment_ids(0).is_none());
    }

    #[test]
    fn test_comment_ranges_with_table() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Before table</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
                </w:tbl>
                <w:p><w:r><w:t>After table</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let ranges = CommentRanges::parse(xml);
        // Block indices: 0=para, 1=table, 2=para
        assert!(ranges.get_comment_ids(0).is_none());
        assert!(ranges.get_comment_ids(1).is_none());
        assert!(ranges.get_comment_ids(2).is_none());
    }

    #[test]
    fn test_metadata_parse_empty() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
        </cp:coreProperties>"#;

        let metadata = DocumentMetadata::parse(xml);
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.created.is_none());
    }

    #[test]
    fn test_metadata_parse_partial() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                          xmlns:dc="http://purl.org/dc/elements/1.1/"
                          xmlns:dcterms="http://purl.org/dc/terms/">
            <dc:title>Test Document</dc:title>
            <dc:creator>Test Author</dc:creator>
        </cp:coreProperties>"#;

        let metadata = DocumentMetadata::parse(xml);
        assert_eq!(metadata.title, Some("Test Document".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert!(metadata.subject.is_none());
        assert!(metadata.modified.is_none());
    }

    #[test]
    fn test_metadata_to_asciidoc_header_empty() {
        let metadata = DocumentMetadata::default();
        let header = metadata.to_asciidoc_header();
        assert!(header.is_empty());
    }

    #[test]
    fn test_metadata_to_asciidoc_header_with_author() {
        let metadata = DocumentMetadata {
            author: Some("John Doe".to_string()),
            modified: Some("2025-01-15T10:30:00Z".to_string()),
            ..Default::default()
        };
        let header = metadata.to_asciidoc_header();
        assert!(header.contains(":author: John Doe"));
        assert!(header.contains(":revdate: 2025-01-15"));
    }

    #[test]
    fn test_metadata_to_asciidoc_header_empty_author() {
        let metadata = DocumentMetadata {
            author: Some("".to_string()), // Empty string
            ..Default::default()
        };
        let header = metadata.to_asciidoc_header();
        // Empty author should not produce :author: line
        assert!(!header.contains(":author:"));
    }

    #[test]
    fn test_extractor_with_force_parse() {
        let extractor = AsciiDocExtractor::new().with_force_parse(true);
        assert!(extractor.force_parse);

        let extractor2 = AsciiDocExtractor::new().with_force_parse(false);
        assert!(!extractor2.force_parse);
    }

    #[test]
    fn test_style_mappings_to_toml_empty() {
        let mappings = StyleMappings::default();
        let toml = mappings.to_toml();
        assert!(toml.contains("[styles]"));
        // Should not have any style mappings
        assert!(!toml.contains("heading"));
        assert!(!toml.contains("paragraph"));
    }

    #[test]
    fn test_style_mappings_to_toml_with_tables() {
        let mappings = StyleMappings {
            tables: vec!["TableGrid".to_string(), "TableSimple".to_string()],
            ..Default::default()
        };
        let toml = mappings.to_toml();
        // Only first table should be included
        assert!(toml.contains("table = \"TableGrid\""));
    }

    #[test]
    fn test_convert_empty_table() {
        let extractor = AsciiDocExtractor::new();
        let table = Table {
            rows: vec![],
            style_id: None,
        };

        let result = extractor.convert_table(&table);
        // Empty table should produce empty output
        assert!(result.is_empty());
    }

    #[test]
    fn test_is_numbered_list_heuristic() {
        let extractor = AsciiDocExtractor::new();
        let styles = StyleSheet::default();

        // numId 2 is ordered list by convention
        assert!(extractor.is_numbered_list(2, &styles));
        // numId 1 is unordered list
        assert!(!extractor.is_numbered_list(1, &styles));
        // Other numIds default to unordered
        assert!(!extractor.is_numbered_list(0, &styles));
        assert!(!extractor.is_numbered_list(3, &styles));
    }

    #[test]
    fn test_is_code_block_paragraph_mixed_formatting() {
        let extractor = AsciiDocExtractor::new();

        // Mixed monospace and non-monospace - NOT a code block
        let para = Paragraph {
            style_id: None,
            numbering: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "regular ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Run(Run {
                    text: "code\nmore".to_string(),
                    bold: false,
                    italic: false,
                    monospace: true,
                }),
            ],
        };

        assert!(!extractor.is_code_block_paragraph(&para));
    }

    #[test]
    fn test_is_code_block_paragraph_all_monospace_with_newline() {
        let extractor = AsciiDocExtractor::new();

        // All monospace with newline - IS a code block
        let para = Paragraph {
            style_id: None,
            numbering: None,
            children: vec![ParagraphChild::Run(Run {
                text: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
                bold: false,
                italic: false,
                monospace: true,
            })],
        };

        assert!(extractor.is_code_block_paragraph(&para));
    }

    #[test]
    fn test_is_code_block_paragraph_monospace_no_newline() {
        let extractor = AsciiDocExtractor::new();

        // Monospace but no newline - NOT a code block (inline code)
        let para = Paragraph {
            style_id: None,
            numbering: None,
            children: vec![ParagraphChild::Run(Run {
                text: "inline_code".to_string(),
                bold: false,
                italic: false,
                monospace: true,
            })],
        };

        assert!(!extractor.is_code_block_paragraph(&para));
    }

    #[test]
    fn test_get_raw_paragraph_text_with_hyperlink() {
        let extractor = AsciiDocExtractor::new();

        let para = Paragraph {
            style_id: None,
            numbering: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "Click ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Hyperlink(Hyperlink {
                    id: None,
                    anchor: Some("target".to_string()),
                    runs: vec![Run {
                        text: "here".to_string(),
                        bold: false,
                        italic: false,
                        monospace: false,
                    }],
                }),
                ParagraphChild::Run(Run {
                    text: " for more.".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
            ],
        };

        let text = extractor.get_raw_paragraph_text(&para);
        assert_eq!(text, "Click here for more.");
    }

    #[test]
    fn test_get_raw_paragraph_text_with_image() {
        let extractor = AsciiDocExtractor::new();

        let para = Paragraph {
            style_id: None,
            numbering: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "See ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Image(crate::image::Image {
                    id: 1,
                    rel_id: "rId1".to_string(),
                    target: "image.png".to_string(),
                    alt: Some("diagram".to_string()),
                    name: None,
                    width_emu: None,
                    height_emu: None,
                    position: crate::image::ImagePosition::Inline,
                }),
            ],
        };

        let text = extractor.get_raw_paragraph_text(&para);
        assert_eq!(text, "See diagram");
    }

    #[test]
    fn test_convert_hyperlink_no_target() {
        let extractor = AsciiDocExtractor::new();

        // Hyperlink with no id and no anchor
        let hyperlink = Hyperlink {
            id: None,
            anchor: None,
            runs: vec![Run {
                text: "orphan link".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            }],
        };

        let result = extractor.convert_hyperlink(&hyperlink, None);
        // Should produce <<,orphan link>> with empty anchor
        assert!(result.contains("orphan link"));
    }

    #[test]
    fn test_source_origin_variants() {
        assert_eq!(SourceOrigin::Embedded, SourceOrigin::Embedded);
        assert_eq!(SourceOrigin::Parsed, SourceOrigin::Parsed);
        assert_ne!(SourceOrigin::Embedded, SourceOrigin::Parsed);
    }

    #[test]
    fn test_convert_run_preserve_formatting_disabled() {
        let mut extractor = AsciiDocExtractor::new();
        extractor.preserve_formatting = false;

        let run = Run {
            text: "bold text".to_string(),
            bold: true,
            italic: false,
            monospace: false,
        };

        let result = extractor.convert_run(&run);
        // Should NOT have formatting markers
        assert_eq!(result, "bold text");
        assert!(!result.contains('*'));
    }

    #[test]
    fn test_convert_run_bold_italic_combined() {
        let extractor = AsciiDocExtractor::new();

        let run = Run {
            text: "emphasis".to_string(),
            bold: true,
            italic: true,
            monospace: false,
        };

        let result = extractor.convert_run(&run);
        // Should have both bold and italic markers
        assert!(result.contains('*'));
        assert!(result.contains('_'));
    }

    // ==================== Sprint 13: Comments Language Edge Cases ====================

    #[test]
    fn test_comments_language_with_whitespace() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>Language:   rust  </w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        // Should trim whitespace around the language
        assert_eq!(comments.get_language(0), Some("rust".to_string()));
    }

    #[test]
    fn test_comments_language_case_sensitive_prefix() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>language: python</w:t></w:r></w:p>
            </w:comment>
            <w:comment w:id="1" w:author="utf8dok">
                <w:p><w:r><w:t>LANGUAGE: java</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        // Current implementation is case-sensitive, lowercase won't match
        assert_eq!(comments.get_language(0), None);
        assert_eq!(comments.get_language(1), None);
    }

    #[test]
    fn test_comments_language_with_special_chars() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>Language: c++</w:t></w:r></w:p>
            </w:comment>
            <w:comment w:id="1" w:author="utf8dok">
                <w:p><w:r><w:t>Language: c#</w:t></w:r></w:p>
            </w:comment>
            <w:comment w:id="2" w:author="utf8dok">
                <w:p><w:r><w:t>Language: objective-c</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        assert_eq!(comments.get_language(0), Some("c++".to_string()));
        assert_eq!(comments.get_language(1), Some("c#".to_string()));
        assert_eq!(comments.get_language(2), Some("objective-c".to_string()));
    }

    #[test]
    fn test_comments_language_empty_after_prefix() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>Language:</w:t></w:r></w:p>
            </w:comment>
            <w:comment w:id="1" w:author="utf8dok">
                <w:p><w:r><w:t>Language:   </w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        // Empty language after trimming
        assert_eq!(comments.get_language(0), Some("".to_string()));
        assert_eq!(comments.get_language(1), Some("".to_string()));
    }

    #[test]
    fn test_comments_get_nonexistent_id() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="5" w:author="utf8dok">
                <w:p><w:r><w:t>Language: go</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        assert_eq!(comments.get(0), None);
        assert_eq!(comments.get(1), None);
        assert_eq!(comments.get(5), Some("Language: go"));
        assert_eq!(comments.get_language(5), Some("go".to_string()));
    }

    #[test]
    fn test_comments_multiple_paragraphs_in_comment() {
        // Only first paragraph text should be captured
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:comment w:id="0" w:author="utf8dok">
                <w:p><w:r><w:t>Language: typescript</w:t></w:r></w:p>
                <w:p><w:r><w:t>Additional info ignored</w:t></w:r></w:p>
            </w:comment>
        </w:comments>"#;

        let comments = Comments::parse(xml);
        // Current parser concatenates all text in a comment
        let text = comments.get(0).unwrap();
        assert!(text.contains("Language: typescript"));
    }

    #[test]
    fn test_comment_ranges_empty_block_list() {
        let ranges = CommentRanges::default();
        assert!(ranges.get_comment_ids(0).is_none());
        assert!(ranges.get_comment_ids(100).is_none());
    }

    #[test]
    fn test_comment_ranges_multiple_comments_on_same_block() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:commentRangeStart w:id="0"/>
                    <w:commentRangeStart w:id="1"/>
                    <w:r><w:t>Doubly commented</w:t></w:r>
                    <w:commentRangeEnd w:id="0"/>
                    <w:commentRangeEnd w:id="1"/>
                </w:p>
            </w:body>
        </w:document>"#;

        let ranges = CommentRanges::parse(xml);
        let ids = ranges.get_comment_ids(0);
        assert!(ids.is_some());
        let ids = ids.unwrap();
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
    }
}
