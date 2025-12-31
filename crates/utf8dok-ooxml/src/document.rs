//! Document content parsing (word/document.xml)
//!
//! This module parses the main document content and extracts
//! paragraphs, tables, and other block-level elements.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{OoxmlError, Result};
use crate::image::{Image, ImagePosition, WrapType};

/// A parsed Word document
#[derive(Debug, Clone)]
pub struct Document {
    /// Document body blocks
    pub blocks: Vec<Block>,
}

/// Block-level elements
#[derive(Debug, Clone)]
pub enum Block {
    /// A paragraph
    Paragraph(Paragraph),
    /// A table
    Table(Table),
    /// A section break
    SectionBreak,
}

/// A paragraph with its content and style
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// Style ID (references styles.xml)
    pub style_id: Option<String>,
    /// Children (runs and hyperlinks)
    pub children: Vec<ParagraphChild>,
    /// Numbering info (for lists/headings)
    pub numbering: Option<NumberingRef>,
}

/// Child elements of a paragraph
#[derive(Debug, Clone)]
pub enum ParagraphChild {
    /// A text run
    Run(Run),
    /// A hyperlink
    Hyperlink(Hyperlink),
    /// An embedded image
    Image(Image),
    /// A bookmark anchor
    Bookmark(Bookmark),
}

/// A bookmark (anchor point for internal links)
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Bookmark name (used as anchor ID)
    pub name: String,
}

/// A hyperlink with its target and content
#[derive(Debug, Clone)]
pub struct Hyperlink {
    /// Relationship ID for external URLs (r:id)
    pub id: Option<String>,
    /// Internal anchor name (w:anchor)
    pub anchor: Option<String>,
    /// Child runs inside the hyperlink
    pub runs: Vec<Run>,
}

/// A text run with formatting
#[derive(Debug, Clone)]
pub struct Run {
    /// The text content
    pub text: String,
    /// Whether the text is bold
    pub bold: bool,
    /// Whether the text is italic
    pub italic: bool,
    /// Whether the text is monospace/code
    pub monospace: bool,
}

/// Reference to numbering definition
#[derive(Debug, Clone)]
pub struct NumberingRef {
    /// Numbering ID
    pub num_id: u32,
    /// Indent level (0-based)
    pub ilvl: u32,
}

/// A table
#[derive(Debug, Clone)]
pub struct Table {
    /// Table style ID
    pub style_id: Option<String>,
    /// Table rows
    pub rows: Vec<TableRow>,
}

/// A table row
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Cells in this row
    pub cells: Vec<TableCell>,
    /// Whether this is a header row
    pub is_header: bool,
}

/// A table cell
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Paragraphs in this cell
    pub paragraphs: Vec<Paragraph>,
}

impl Document {
    /// Parse a document from XML bytes
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let mut reader = Reader::from_reader(xml);
        // Don't trim text - preserve whitespace in runs
        reader.config_mut().trim_text(false);

        let mut blocks = Vec::new();
        let mut buf = Vec::new();

        // State for current paragraph
        let mut in_body = false;
        let mut in_textbox_content = false; // Track if inside <w:txbxContent>
        let mut in_drawingml_shape = 0u32; // Depth counter for DrawingML shapes (wsp, sp, etc.)
        let mut current_para: Option<ParagraphBuilder> = None;
        let mut current_run: Option<RunBuilder> = None;
        let mut current_table: Option<TableBuilder> = None;
        let mut current_hyperlink: Option<HyperlinkBuilder> = None;
        // Track if we're inside a <w:t> or <a:t> element (actual text vs instrText)
        let mut in_text_element = false;
        // Image parsing state
        let mut current_image: Option<ImageBuilder> = None;
        let mut image_id_counter: u32 = 1;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"body" => in_body = true,
                        b"txbxContent" => {
                            // Text box content - treat like body for paragraph parsing
                            in_textbox_content = true;
                        }
                        // Track DrawingML shapes for a:t text extraction
                        b"wsp" | b"sp" | b"cxnSp" => {
                            // WordprocessingML shape, DrawingML shape, or connector shape
                            in_drawingml_shape += 1;
                        }
                        b"p" if (in_body || in_textbox_content) && current_table.is_none() => {
                            current_para = Some(ParagraphBuilder::new());
                        }
                        b"p" if current_table.is_some() => {
                            // Paragraph inside table cell
                            current_para = Some(ParagraphBuilder::new());
                        }
                        b"p" if in_drawingml_shape > 0 && !in_textbox_content => {
                            // DrawingML paragraph (a:p) inside a shape
                            // Only create if not already inside txbxContent which uses w:p
                            current_para = Some(ParagraphBuilder::new());
                        }
                        b"pStyle" if current_para.is_some() => {
                            if let Some(style) = get_attr(e, b"w:val") {
                                current_para.as_mut().unwrap().style_id = Some(style);
                            }
                        }
                        b"numPr" => {}
                        b"numId" if current_para.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(num_id) = val.parse() {
                                    let para = current_para.as_mut().unwrap();
                                    if para.numbering.is_none() {
                                        para.numbering = Some(NumberingRef { num_id, ilvl: 0 });
                                    } else {
                                        para.numbering.as_mut().unwrap().num_id = num_id;
                                    }
                                }
                            }
                        }
                        b"ilvl" if current_para.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(ilvl) = val.parse() {
                                    let para = current_para.as_mut().unwrap();
                                    if para.numbering.is_none() {
                                        para.numbering = Some(NumberingRef { num_id: 0, ilvl });
                                    } else {
                                        para.numbering.as_mut().unwrap().ilvl = ilvl;
                                    }
                                }
                            }
                        }
                        b"r" if current_para.is_some() => {
                            // WordprocessingML run (w:r) or DrawingML run (a:r)
                            current_run = Some(RunBuilder::new());
                        }
                        b"b" if current_run.is_some() => {
                            // Check for w:val="0" which means NOT bold
                            let is_off = get_attr(e, b"w:val")
                                .map(|v| v == "0" || v == "false")
                                .unwrap_or(false);
                            if !is_off {
                                current_run.as_mut().unwrap().bold = true;
                            }
                        }
                        b"i" if current_run.is_some() => {
                            let is_off = get_attr(e, b"w:val")
                                .map(|v| v == "0" || v == "false")
                                .unwrap_or(false);
                            if !is_off {
                                current_run.as_mut().unwrap().italic = true;
                            }
                        }
                        b"rFonts" if current_run.is_some() => {
                            // Check for monospace fonts
                            if let Some(font) = get_attr(e, b"w:ascii") {
                                if is_monospace_font(&font) {
                                    current_run.as_mut().unwrap().monospace = true;
                                }
                            }
                        }
                        b"t" if current_run.is_some() => {
                            // Start of actual text element (w:t or a:t) - capture text from here
                            in_text_element = true;
                        }
                        b"drawing" if current_para.is_some() => {
                            // Start of drawing element - begin image parsing
                            current_image = Some(ImageBuilder::new(image_id_counter));
                            image_id_counter += 1;
                        }
                        b"inline" if current_image.is_some() => {
                            // Inline image positioning
                            current_image.as_mut().unwrap().position = ImagePosition::Inline;
                        }
                        b"anchor" if current_image.is_some() => {
                            // Anchored image positioning
                            let horizontal = get_attr(e, b"distL")
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(0);
                            let vertical = get_attr(e, b"distT")
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(0);
                            current_image.as_mut().unwrap().position = ImagePosition::Anchor {
                                horizontal,
                                vertical,
                                wrap: WrapType::None,
                            };
                        }
                        b"extent" if current_image.is_some() => {
                            // Image dimensions in EMUs
                            if let Some(cx) = get_attr(e, b"cx") {
                                if let Ok(width) = cx.parse::<i64>() {
                                    current_image.as_mut().unwrap().width_emu = Some(width);
                                }
                            }
                            if let Some(cy) = get_attr(e, b"cy") {
                                if let Ok(height) = cy.parse::<i64>() {
                                    current_image.as_mut().unwrap().height_emu = Some(height);
                                }
                            }
                        }
                        b"docPr" if current_image.is_some() => {
                            // Document properties (alt text, name, id)
                            if let Some(descr) = get_attr(e, b"descr") {
                                current_image.as_mut().unwrap().alt = Some(descr);
                            }
                            if let Some(name) = get_attr(e, b"name") {
                                current_image.as_mut().unwrap().name = Some(name);
                            }
                            if let Some(id) = get_attr(e, b"id") {
                                if let Ok(id_num) = id.parse::<u32>() {
                                    current_image.as_mut().unwrap().doc_id = Some(id_num);
                                }
                            }
                        }
                        b"blip" if current_image.is_some() => {
                            // Image reference via relationship ID
                            if let Some(rel_id) = get_attr_with_ns(e, b"r:embed") {
                                current_image.as_mut().unwrap().rel_id = Some(rel_id);
                            }
                        }
                        b"wrapSquare" | b"wrapTight" | b"wrapThrough" | b"wrapTopAndBottom"
                        | b"wrapNone"
                            if current_image.is_some() =>
                        {
                            // Update wrap type for anchored images
                            let wrap_type = WrapType::from_element_name(
                                std::str::from_utf8(name.as_ref()).unwrap_or("wrapNone"),
                            );
                            if let Some(ref mut img) = current_image {
                                if let ImagePosition::Anchor { ref mut wrap, .. } = img.position {
                                    *wrap = wrap_type;
                                }
                            }
                        }
                        b"posOffset" if current_image.is_some() => {
                            // Position offset will be captured as text
                        }
                        b"hyperlink" if current_para.is_some() => {
                            // Start of hyperlink
                            let mut builder = HyperlinkBuilder::new();
                            // Get r:id attribute for external links
                            if let Some(id) = get_attr_with_ns(e, b"r:id") {
                                builder.id = Some(id);
                            }
                            // Get w:anchor attribute for internal links
                            if let Some(anchor) = get_attr_with_ns(e, b"w:anchor") {
                                builder.anchor = Some(anchor);
                            }
                            current_hyperlink = Some(builder);
                        }
                        b"tbl" if in_body => {
                            current_table = Some(TableBuilder::new());
                        }
                        b"tblStyle" if current_table.is_some() => {
                            if let Some(style) = get_attr(e, b"w:val") {
                                current_table.as_mut().unwrap().style_id = Some(style);
                            }
                        }
                        b"tr" if current_table.is_some() => {
                            current_table.as_mut().unwrap().current_row =
                                Some(TableRowBuilder::new());
                        }
                        b"tc" if current_table.is_some() => {
                            if let Some(ref mut table) = current_table {
                                if let Some(ref mut row) = table.current_row {
                                    row.current_cell = Some(TableCellBuilder::new());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"body" => in_body = false,
                        b"txbxContent" => in_textbox_content = false,
                        b"wsp" | b"sp" | b"cxnSp" => {
                            // End of DrawingML shape
                            in_drawingml_shape = in_drawingml_shape.saturating_sub(1);
                        }
                        b"p" if current_para.is_some() => {
                            let para = current_para.take().unwrap().build();

                            if let Some(ref mut table) = current_table {
                                // Add to current table cell
                                if let Some(ref mut row) = table.current_row {
                                    if let Some(ref mut cell) = row.current_cell {
                                        cell.paragraphs.push(para);
                                    }
                                }
                            } else {
                                blocks.push(Block::Paragraph(para));
                            }
                        }
                        b"t" => {
                            // End of text element
                            in_text_element = false;
                        }
                        b"r" if current_run.is_some() => {
                            let run = current_run.take().unwrap().build();
                            if !run.text.is_empty() {
                                // If inside a hyperlink, add to hyperlink
                                if let Some(ref mut hyperlink) = current_hyperlink {
                                    hyperlink.runs.push(run);
                                } else if let Some(ref mut para) = current_para {
                                    // Otherwise add directly to paragraph
                                    para.children.push(ParagraphChild::Run(run));
                                }
                            }
                        }
                        b"hyperlink" if current_hyperlink.is_some() => {
                            // End of hyperlink - add to paragraph
                            let hyperlink = current_hyperlink.take().unwrap().build();
                            if let Some(ref mut para) = current_para {
                                para.children.push(ParagraphChild::Hyperlink(hyperlink));
                            }
                        }
                        b"drawing" if current_image.is_some() => {
                            // End of drawing - add image to paragraph
                            if let Some(image_builder) = current_image.take() {
                                if let Some(image) = image_builder.build() {
                                    if let Some(ref mut para) = current_para {
                                        para.children.push(ParagraphChild::Image(image));
                                    }
                                }
                            }
                        }
                        b"tc" if current_table.is_some() => {
                            if let Some(ref mut table) = current_table {
                                if let Some(ref mut row) = table.current_row {
                                    if let Some(cell) = row.current_cell.take() {
                                        row.cells.push(cell.build());
                                    }
                                }
                            }
                        }
                        b"tr" if current_table.is_some() => {
                            if let Some(ref mut table) = current_table {
                                if let Some(row) = table.current_row.take() {
                                    table.rows.push(row.build());
                                }
                            }
                        }
                        b"tbl" if current_table.is_some() => {
                            let table = current_table.take().unwrap().build();
                            blocks.push(Block::Table(table));
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    // Handle self-closing elements like <w:pStyle w:val="Heading1"/>
                    let name = e.local_name();
                    match name.as_ref() {
                        b"pStyle" if current_para.is_some() => {
                            if let Some(style) = get_attr(e, b"w:val") {
                                current_para.as_mut().unwrap().style_id = Some(style);
                            }
                        }
                        b"numId" if current_para.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(num_id) = val.parse() {
                                    let para = current_para.as_mut().unwrap();
                                    if para.numbering.is_none() {
                                        para.numbering = Some(NumberingRef { num_id, ilvl: 0 });
                                    } else {
                                        para.numbering.as_mut().unwrap().num_id = num_id;
                                    }
                                }
                            }
                        }
                        b"ilvl" if current_para.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(ilvl) = val.parse() {
                                    let para = current_para.as_mut().unwrap();
                                    if para.numbering.is_none() {
                                        para.numbering = Some(NumberingRef { num_id: 0, ilvl });
                                    } else {
                                        para.numbering.as_mut().unwrap().ilvl = ilvl;
                                    }
                                }
                            }
                        }
                        b"b" if current_run.is_some() => {
                            current_run.as_mut().unwrap().bold = true;
                        }
                        b"i" if current_run.is_some() => {
                            current_run.as_mut().unwrap().italic = true;
                        }
                        b"rFonts" if current_run.is_some() => {
                            // Check for monospace fonts (self-closing element)
                            if let Some(font) = get_attr(e, b"w:ascii") {
                                if is_monospace_font(&font) {
                                    current_run.as_mut().unwrap().monospace = true;
                                }
                            }
                        }
                        b"bookmarkStart" if current_para.is_some() => {
                            // Parse bookmark anchor
                            if let Some(name) = get_attr(e, b"w:name") {
                                // Skip internal Word bookmarks (starting with _)
                                if !name.starts_with('_') {
                                    let para = current_para.as_mut().unwrap();
                                    para.children
                                        .push(ParagraphChild::Bookmark(Bookmark { name }));
                                }
                            }
                        }
                        b"extent" if current_image.is_some() => {
                            // Image dimensions in EMUs (self-closing)
                            if let Some(cx) = get_attr(e, b"cx") {
                                if let Ok(width) = cx.parse::<i64>() {
                                    current_image.as_mut().unwrap().width_emu = Some(width);
                                }
                            }
                            if let Some(cy) = get_attr(e, b"cy") {
                                if let Ok(height) = cy.parse::<i64>() {
                                    current_image.as_mut().unwrap().height_emu = Some(height);
                                }
                            }
                        }
                        b"docPr" if current_image.is_some() => {
                            // Document properties (self-closing)
                            if let Some(descr) = get_attr(e, b"descr") {
                                current_image.as_mut().unwrap().alt = Some(descr);
                            }
                            if let Some(name) = get_attr(e, b"name") {
                                current_image.as_mut().unwrap().name = Some(name);
                            }
                            if let Some(id) = get_attr(e, b"id") {
                                if let Ok(id_num) = id.parse::<u32>() {
                                    current_image.as_mut().unwrap().doc_id = Some(id_num);
                                }
                            }
                        }
                        b"blip" if current_image.is_some() => {
                            // Image reference via relationship ID (self-closing)
                            if let Some(rel_id) = get_attr_with_ns(e, b"r:embed") {
                                current_image.as_mut().unwrap().rel_id = Some(rel_id);
                            }
                        }
                        b"wrapSquare" | b"wrapTight" | b"wrapThrough" | b"wrapTopAndBottom"
                        | b"wrapNone"
                            if current_image.is_some() =>
                        {
                            // Wrap type for anchored images (self-closing)
                            let wrap_type = WrapType::from_element_name(
                                std::str::from_utf8(name.as_ref()).unwrap_or("wrapNone"),
                            );
                            if let Some(ref mut img) = current_image {
                                if let ImagePosition::Anchor { ref mut wrap, .. } = img.position {
                                    *wrap = wrap_type;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    // Only capture text inside <w:t> elements, not <w:instrText>
                    if in_text_element {
                        if let Some(ref mut run) = current_run {
                            let text = e.unescape().unwrap_or_default();
                            run.text.push_str(&text);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(OoxmlError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(Document { blocks })
    }

    /// Get all paragraphs (flattening tables)
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        self.blocks.iter().flat_map(|block| match block {
            Block::Paragraph(p) => vec![p].into_iter(),
            Block::Table(t) => t
                .rows
                .iter()
                .flat_map(|r| r.cells.iter())
                .flat_map(|c| c.paragraphs.iter())
                .collect::<Vec<_>>()
                .into_iter(),
            Block::SectionBreak => vec![].into_iter(),
        })
    }

    /// Get plain text content
    pub fn plain_text(&self) -> String {
        self.paragraphs()
            .map(|p| p.plain_text())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl Paragraph {
    /// Get plain text of this paragraph
    pub fn plain_text(&self) -> String {
        self.children
            .iter()
            .map(|child| match child {
                ParagraphChild::Run(run) => run.text.clone(),
                ParagraphChild::Hyperlink(hyperlink) => {
                    // Collect text from all runs in the hyperlink
                    hyperlink
                        .runs
                        .iter()
                        .map(|r| r.text.as_str())
                        .collect::<String>()
                }
                ParagraphChild::Image(img) => {
                    // Use alt text as placeholder if available
                    img.alt.clone().unwrap_or_default()
                }
                ParagraphChild::Bookmark(_) => String::new(), // Bookmarks have no text
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this paragraph is empty
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
            || self.children.iter().all(|child| match child {
                ParagraphChild::Run(run) => run.text.trim().is_empty(),
                ParagraphChild::Hyperlink(hyperlink) => {
                    hyperlink.runs.iter().all(|r| r.text.trim().is_empty())
                }
                ParagraphChild::Image(_) => false, // Images are never "empty"
                ParagraphChild::Bookmark(_) => true, // Bookmarks are "empty" (no visible content)
            })
    }

    /// Get all runs (flattening hyperlinks)
    pub fn runs(&self) -> impl Iterator<Item = &Run> {
        self.children.iter().flat_map(|child| match child {
            ParagraphChild::Run(run) => vec![run].into_iter(),
            ParagraphChild::Hyperlink(hyperlink) => {
                hyperlink.runs.iter().collect::<Vec<_>>().into_iter()
            }
            ParagraphChild::Image(_) => vec![].into_iter(),
            ParagraphChild::Bookmark(_) => vec![].into_iter(), // Bookmarks have no runs
        })
    }

    /// Get all images in this paragraph
    pub fn images(&self) -> impl Iterator<Item = &Image> {
        self.children.iter().filter_map(|child| match child {
            ParagraphChild::Image(img) => Some(img),
            _ => None,
        })
    }
}

// Builder types for constructing elements during parsing

#[derive(Default)]
struct ParagraphBuilder {
    style_id: Option<String>,
    children: Vec<ParagraphChild>,
    numbering: Option<NumberingRef>,
}

impl ParagraphBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Paragraph {
        Paragraph {
            style_id: self.style_id,
            children: self.children,
            numbering: self.numbering,
        }
    }
}

#[derive(Default)]
struct HyperlinkBuilder {
    id: Option<String>,
    anchor: Option<String>,
    runs: Vec<Run>,
}

impl HyperlinkBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Hyperlink {
        Hyperlink {
            id: self.id,
            anchor: self.anchor,
            runs: self.runs,
        }
    }
}

#[derive(Default)]
struct RunBuilder {
    text: String,
    bold: bool,
    italic: bool,
    monospace: bool,
}

impl RunBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Run {
        Run {
            text: self.text,
            bold: self.bold,
            italic: self.italic,
            monospace: self.monospace,
        }
    }
}

#[derive(Default)]
struct TableBuilder {
    style_id: Option<String>,
    rows: Vec<TableRow>,
    current_row: Option<TableRowBuilder>,
}

impl TableBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Table {
        Table {
            style_id: self.style_id,
            rows: self.rows,
        }
    }
}

#[derive(Default)]
struct TableRowBuilder {
    cells: Vec<TableCell>,
    current_cell: Option<TableCellBuilder>,
    is_header: bool,
}

impl TableRowBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> TableRow {
        TableRow {
            cells: self.cells,
            is_header: self.is_header,
        }
    }
}

#[derive(Default)]
struct TableCellBuilder {
    paragraphs: Vec<Paragraph>,
}

impl TableCellBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> TableCell {
        TableCell {
            paragraphs: self.paragraphs,
        }
    }
}

/// Builder for Image elements during parsing
struct ImageBuilder {
    id: u32,
    rel_id: Option<String>,
    alt: Option<String>,
    name: Option<String>,
    doc_id: Option<u32>,
    width_emu: Option<i64>,
    height_emu: Option<i64>,
    position: ImagePosition,
}

impl ImageBuilder {
    fn new(id: u32) -> Self {
        Self {
            id,
            rel_id: None,
            alt: None,
            name: None,
            doc_id: None,
            width_emu: None,
            height_emu: None,
            position: ImagePosition::Inline,
        }
    }

    /// Build the Image if we have the required relationship ID
    fn build(self) -> Option<Image> {
        // rel_id is required to reference the actual image file
        let rel_id = self.rel_id?;

        Some(Image {
            id: self.doc_id.unwrap_or(self.id),
            rel_id,
            // Target will be resolved later from relationships
            target: String::new(),
            alt: self.alt,
            name: self.name,
            width_emu: self.width_emu,
            height_emu: self.height_emu,
            position: self.position,
        })
    }
}

// Helper functions

fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

/// Get attribute with namespace prefix (e.g., "r:id", "w:anchor")
fn get_attr_with_ns(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| {
            let key = a.key.as_ref();
            // Match exact name or local name after colon
            key == name
                || key.ends_with(
                    &name[name
                        .iter()
                        .position(|&b| b == b':')
                        .map(|i| i + 1)
                        .unwrap_or(0)..],
                )
        })
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

fn is_monospace_font(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("mono")
        || lower.contains("courier")
        || lower.contains("consolas")
        || lower.contains("menlo")
        || lower.contains("source code")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_field_codes() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                    <w:r><w:instrText>TOC \o "1-3"</w:instrText></w:r>
                    <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                    <w:r><w:t>Table of Contents</w:t></w:r>
                    <w:r><w:fldChar w:fldCharType="end"/></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let text = doc.plain_text();

        // Should only contain the visible text, not the field instruction
        assert_eq!(text, "Table of Contents");
        assert!(
            !text.contains("TOC"),
            "Field code TOC should not appear in text"
        );
        assert!(
            !text.contains("\\o"),
            "Field code parameters should not appear"
        );
    }

    #[test]
    fn test_parse_simple_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>Hello, world!</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.plain_text(), "Hello, world!");
    }

    #[test]
    fn test_parse_styled_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:pPr>
                        <w:pStyle w:val="Heading1"/>
                    </w:pPr>
                    <w:r>
                        <w:t>Section Title</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        if let Block::Paragraph(p) = &doc.blocks[0] {
            assert_eq!(p.style_id, Some("Heading1".to_string()));
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_hyperlink_with_anchor() {
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
        assert_eq!(doc.blocks.len(), 1);

        if let Block::Paragraph(p) = &doc.blocks[0] {
            assert_eq!(p.children.len(), 1);
            if let ParagraphChild::Hyperlink(h) = &p.children[0] {
                assert_eq!(h.anchor, Some("_Toc123".to_string()));
                assert_eq!(h.id, None);
                assert_eq!(h.runs.len(), 1);
                assert_eq!(h.runs[0].text, "Click me");
            } else {
                panic!("Expected Hyperlink, got {:?}", p.children[0]);
            }
        } else {
            panic!("Expected paragraph");
        }
    }
}
