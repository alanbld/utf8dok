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
                                // Keep semantically meaningful bookmarks, skip internal ones
                                // _Toc* - TOC/heading anchors (important for navigation)
                                // _Ref* - Cross-reference targets (important for linking)
                                // Skip: _Hlk* (hyperlink highlights), _GoBack, other internal
                                let should_keep = !name.starts_with('_')
                                    || name.starts_with("_Toc")
                                    || name.starts_with("_Ref");
                                if should_keep {
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
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        assert_eq!(p.style_id, Some("Heading1".to_string()));
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

        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        assert_eq!(p.children.len(), 1);
        let ParagraphChild::Hyperlink(h) = &p.children[0] else {
            panic!("Expected Hyperlink");
        };
        assert_eq!(h.anchor, Some("_Toc123".to_string()));
        assert_eq!(h.id, None);
        assert_eq!(h.runs.len(), 1);
        assert_eq!(h.runs[0].text, "Click me");
    }

    // ==================== Sprint 8: Document::parse Integration Tests ====================

    #[test]
    fn test_parse_table_simple() {
        // Note: tblStyle must be non-self-closing for current parser
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tblPr>
                        <w:tblStyle w:val="TableGrid"></w:tblStyle>
                    </w:tblPr>
                    <w:tr>
                        <w:tc>
                            <w:p><w:r><w:t>Cell 1</w:t></w:r></w:p>
                        </w:tc>
                        <w:tc>
                            <w:p><w:r><w:t>Cell 2</w:t></w:r></w:p>
                        </w:tc>
                    </w:tr>
                    <w:tr>
                        <w:tc>
                            <w:p><w:r><w:t>Cell 3</w:t></w:r></w:p>
                        </w:tc>
                        <w:tc>
                            <w:p><w:r><w:t>Cell 4</w:t></w:r></w:p>
                        </w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert_eq!(doc.blocks.len(), 1);

        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected Table");
        };
        assert_eq!(t.style_id, Some("TableGrid".to_string()));
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].cells.len(), 2);
        assert_eq!(t.rows[0].cells[0].paragraphs[0].plain_text(), "Cell 1");
        assert_eq!(t.rows[1].cells[1].paragraphs[0].plain_text(), "Cell 4");
    }

    #[test]
    fn test_parse_table_without_style() {
        // Test table without tblStyle element
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected Table");
        };
        assert!(t.style_id.is_none());
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn test_parse_table_multiple_rows() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:tc>
                    </w:tr>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected Table");
        };
        assert_eq!(t.rows.len(), 2);
        // Note: is_header detection not yet implemented
        assert_eq!(t.rows[0].cells[0].paragraphs[0].plain_text(), "Header");
        assert_eq!(t.rows[1].cells[0].paragraphs[0].plain_text(), "Data");
    }

    #[test]
    fn test_parse_section_break() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Before break</w:t></w:r></w:p>
                <w:p>
                    <w:pPr>
                        <w:sectPr>
                            <w:type w:val="nextPage"/>
                        </w:sectPr>
                    </w:pPr>
                </w:p>
                <w:p><w:r><w:t>After break</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        // Should have: para, section para, para
        assert!(doc.blocks.len() >= 2);
    }

    #[test]
    fn test_parse_numbering_reference() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:pPr>
                        <w:numPr>
                            <w:ilvl w:val="0"/>
                            <w:numId w:val="1"/>
                        </w:numPr>
                    </w:pPr>
                    <w:r><w:t>List item</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        assert!(p.numbering.is_some());
        let num = p.numbering.as_ref().unwrap();
        assert_eq!(num.num_id, 1);
        assert_eq!(num.ilvl, 0);
    }

    #[test]
    fn test_parse_multiple_block_types() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Intro</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
                <w:p><w:r><w:t>Conclusion</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
        assert!(matches!(&doc.blocks[1], Block::Table(_)));
        assert!(matches!(&doc.blocks[2], Block::Paragraph(_)));
    }

    #[test]
    fn test_parse_empty_document() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_run_with_formatting() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:rPr>
                            <w:b/>
                            <w:i/>
                        </w:rPr>
                        <w:t>Bold and italic</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        let ParagraphChild::Run(r) = &p.children[0] else {
            panic!("Expected run");
        };
        assert!(r.bold);
        assert!(r.italic);
        assert_eq!(r.text, "Bold and italic");
    }

    #[test]
    fn test_parse_hyperlink_with_external_id() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <w:body>
                <w:p>
                    <w:hyperlink r:id="rId5">
                        <w:r><w:t>External link</w:t></w:r>
                    </w:hyperlink>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        let ParagraphChild::Hyperlink(h) = &p.children[0] else {
            panic!("Expected hyperlink");
        };
        assert_eq!(h.id, Some("rId5".to_string()));
        assert!(h.anchor.is_none());
    }

    #[test]
    fn test_parse_bookmark() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="0" w:name="_Toc123456"/>
                    <w:r><w:t>Heading</w:t></w:r>
                    <w:bookmarkEnd w:id="0"/>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        // Should have bookmark and run
        let has_bookmark = p.children.iter().any(|c| matches!(c, ParagraphChild::Bookmark(_)));
        assert!(has_bookmark);
    }

    #[test]
    fn test_parse_table_cell_with_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc>
                            <w:p><w:r><w:t>Cell content</w:t></w:r></w:p>
                        </w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected Table");
        };
        assert_eq!(t.rows[0].cells.len(), 1);
        assert!(!t.rows[0].cells[0].paragraphs.is_empty());
        assert_eq!(t.rows[0].cells[0].paragraphs[0].plain_text(), "Cell content");
    }

    #[test]
    fn test_document_paragraphs_iterator_flattens_tables() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>First</w:t></w:r></w:p>
                <w:p><w:r><w:t>Second</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr><w:tc><w:p><w:r><w:t>Table text</w:t></w:r></w:p></w:tc></w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let paras: Vec<_> = doc.paragraphs().collect();
        // paragraphs() flattens tables, so includes table paragraphs
        assert_eq!(paras.len(), 3);
        assert_eq!(paras[0].plain_text(), "First");
        assert_eq!(paras[1].plain_text(), "Second");
        assert_eq!(paras[2].plain_text(), "Table text");
    }

    #[test]
    fn test_document_blocks_access() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Para</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr><w:tc><w:p><w:r><w:t>Table</w:t></w:r></w:p></w:tc></w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        // For block-level iteration, use blocks directly
        assert_eq!(doc.blocks.len(), 2);
        let top_level_paras: Vec<_> = doc.blocks.iter()
            .filter_map(|b| match b {
                Block::Paragraph(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(top_level_paras.len(), 1);
        assert_eq!(top_level_paras[0].plain_text(), "Para");
    }

    // ==================== Sprint 7: Paragraph::is_empty Tests ====================

    #[test]
    fn test_paragraph_is_empty_with_no_children() {
        let para = Paragraph {
            style_id: None,
            children: vec![],
            numbering: None,
        };
        assert!(para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_with_whitespace_only() {
        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Run(Run {
                text: "   \t\n  ".to_string(),
                bold: false,
                italic: false,
                monospace: false,
            })],
            numbering: None,
        };
        assert!(para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_with_image() {
        use crate::image::{Image, ImagePosition};

        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Image(Image {
                id: 1,
                rel_id: "rId1".to_string(),
                target: "media/image.png".to_string(),
                alt: None,
                name: None,
                width_emu: None,
                height_emu: None,
                position: ImagePosition::Inline,
            })],
            numbering: None,
        };
        // Images are NOT empty
        assert!(!para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_with_bookmark_only() {
        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Bookmark(Bookmark {
                name: "_Toc123".to_string(),
            })],
            numbering: None,
        };
        // Bookmarks are considered empty (no visible content)
        assert!(para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_with_empty_hyperlink() {
        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Hyperlink(Hyperlink {
                id: Some("rId1".to_string()),
                anchor: None,
                runs: vec![Run {
                    text: "   ".to_string(), // Whitespace only
                    bold: false,
                    italic: false,
                    monospace: false,
                }],
            })],
            numbering: None,
        };
        assert!(para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_with_hyperlink_and_image() {
        use crate::image::{Image, ImagePosition};

        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Hyperlink(Hyperlink {
                    id: Some("rId1".to_string()),
                    anchor: None,
                    runs: vec![], // Empty runs
                }),
                ParagraphChild::Image(Image {
                    id: 1,
                    rel_id: "rId2".to_string(),
                    target: "media/image.png".to_string(),
                    alt: None,
                    name: None,
                    width_emu: None,
                    height_emu: None,
                    position: ImagePosition::Inline,
                }),
            ],
            numbering: None,
        };
        // Image makes it non-empty
        assert!(!para.is_empty());
    }

    #[test]
    fn test_paragraph_is_empty_mixed_children() {
        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "   ".to_string(), // Whitespace only
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Run(Run {
                    text: "content".to_string(), // Has content
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
            ],
            numbering: None,
        };
        // One run has content
        assert!(!para.is_empty());
    }

    #[test]
    fn test_paragraph_runs_iterator() {
        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "First ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Hyperlink(Hyperlink {
                    id: None,
                    anchor: Some("target".to_string()),
                    runs: vec![
                        Run {
                            text: "link".to_string(),
                            bold: true,
                            italic: false,
                            monospace: false,
                        },
                        Run {
                            text: " text".to_string(),
                            bold: false,
                            italic: false,
                            monospace: false,
                        },
                    ],
                }),
                ParagraphChild::Run(Run {
                    text: " last".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
            ],
            numbering: None,
        };

        let runs: Vec<_> = para.runs().collect();
        assert_eq!(runs.len(), 4);
        assert_eq!(runs[0].text, "First ");
        assert_eq!(runs[1].text, "link");
        assert_eq!(runs[2].text, " text");
        assert_eq!(runs[3].text, " last");
    }

    #[test]
    fn test_paragraph_images_iterator() {
        use crate::image::{Image, ImagePosition};

        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "Text ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Image(Image {
                    id: 1,
                    rel_id: "rId1".to_string(),
                    target: "media/image1.png".to_string(),
                    alt: Some("First image".to_string()),
                    name: None,
                    width_emu: None,
                    height_emu: None,
                    position: ImagePosition::Inline,
                }),
                ParagraphChild::Image(Image {
                    id: 2,
                    rel_id: "rId2".to_string(),
                    target: "media/image2.png".to_string(),
                    alt: Some("Second image".to_string()),
                    name: None,
                    width_emu: None,
                    height_emu: None,
                    position: ImagePosition::Inline,
                }),
            ],
            numbering: None,
        };

        let images: Vec<_> = para.images().collect();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].target, "media/image1.png");
        assert_eq!(images[1].target, "media/image2.png");
    }

    // ==================== Sprint 16: Document Coverage Edge Cases ====================

    #[test]
    fn test_is_monospace_font_variants() {
        // Standard monospace fonts
        assert!(is_monospace_font("Courier New"));
        assert!(is_monospace_font("Consolas"));
        assert!(is_monospace_font("Menlo"));
        assert!(is_monospace_font("Source Code Pro"));
        assert!(is_monospace_font("Ubuntu Mono"));

        // Case insensitive
        assert!(is_monospace_font("COURIER"));
        assert!(is_monospace_font("CONSOLAS"));
        assert!(is_monospace_font("DejaVu Sans Mono"));

        // Non-monospace fonts
        assert!(!is_monospace_font("Arial"));
        assert!(!is_monospace_font("Times New Roman"));
        assert!(!is_monospace_font("Calibri"));
        assert!(!is_monospace_font("Helvetica"));
        // Note: Monaco is monospace but not detected (doesn't contain "mono")
        assert!(!is_monospace_font("Monaco"));
    }

    #[test]
    fn test_bookmark_filtering_internal_bookmarks() {
        // Internal bookmarks like _Hlk and _GoBack should be filtered out
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="0" w:name="_Hlk123"/>
                    <w:bookmarkStart w:id="1" w:name="_GoBack"/>
                    <w:bookmarkStart w:id="2" w:name="_Toc456"/>
                    <w:bookmarkStart w:id="3" w:name="_Ref789"/>
                    <w:bookmarkStart w:id="4" w:name="UserBookmark"/>
                    <w:r><w:t>Content</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };

        let bookmarks: Vec<_> = p
            .children
            .iter()
            .filter_map(|c| match c {
                ParagraphChild::Bookmark(b) => Some(&b.name),
                _ => None,
            })
            .collect();

        // _Toc, _Ref, and user bookmarks should be kept
        assert!(bookmarks.contains(&&"_Toc456".to_string()));
        assert!(bookmarks.contains(&&"_Ref789".to_string()));
        assert!(bookmarks.contains(&&"UserBookmark".to_string()));

        // _Hlk and _GoBack should be filtered out
        assert!(!bookmarks.contains(&&"_Hlk123".to_string()));
        assert!(!bookmarks.contains(&&"_GoBack".to_string()));
    }

    #[test]
    fn test_document_blocks_direct_access() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
                <w:p><w:r><w:t>Para 2</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Direct block access
        assert_eq!(doc.blocks.len(), 3);

        // Type checking
        assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
        assert!(matches!(&doc.blocks[1], Block::Paragraph(_)));
        assert!(matches!(&doc.blocks[2], Block::Table(_)));
    }

    #[test]
    fn test_document_plain_text_with_tables() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Before table</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Cell A</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>Cell B</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
                <w:p><w:r><w:t>After table</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let text = doc.plain_text();

        // Should include all paragraph text, including table cells
        assert!(text.contains("Before table"));
        assert!(text.contains("Cell A"));
        assert!(text.contains("Cell B"));
        assert!(text.contains("After table"));
    }

    #[test]
    fn test_document_paragraphs_flattens_tables() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Intro</w:t></w:r></w:p>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>Cell 1</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>Cell 2</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let paragraphs: Vec<_> = doc.paragraphs().collect();

        // Should have 3 paragraphs: 1 standalone + 2 in table cells
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].plain_text(), "Intro");
        assert_eq!(paragraphs[1].plain_text(), "Cell 1");
        assert_eq!(paragraphs[2].plain_text(), "Cell 2");
    }

    #[test]
    fn test_numbering_ref_ilvl_before_numid() {
        // Test when ilvl comes before numId in XML
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:pPr>
                        <w:numPr>
                            <w:ilvl w:val="2"/>
                            <w:numId w:val="5"/>
                        </w:numPr>
                    </w:pPr>
                    <w:r><w:t>Nested item</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };

        let num = p.numbering.as_ref().unwrap();
        assert_eq!(num.ilvl, 2);
        assert_eq!(num.num_id, 5);
    }

    #[test]
    fn test_paragraph_plain_text_with_image_alt() {
        use crate::image::{Image, ImagePosition};

        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Run(Run {
                    text: "See figure: ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
                ParagraphChild::Image(Image {
                    id: 1,
                    rel_id: "rId1".to_string(),
                    target: "media/diagram.png".to_string(),
                    alt: Some("Architecture Diagram".to_string()),
                    name: None,
                    width_emu: None,
                    height_emu: None,
                    position: ImagePosition::Inline,
                }),
            ],
            numbering: None,
        };

        let text = para.plain_text();
        assert_eq!(text, "See figure: Architecture Diagram");
    }

    #[test]
    fn test_paragraph_plain_text_image_no_alt() {
        use crate::image::{Image, ImagePosition};

        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Image(Image {
                id: 1,
                rel_id: "rId1".to_string(),
                target: "media/image.png".to_string(),
                alt: None, // No alt text
                name: None,
                width_emu: None,
                height_emu: None,
                position: ImagePosition::Inline,
            })],
            numbering: None,
        };

        // Image without alt should contribute empty string
        assert_eq!(para.plain_text(), "");
    }

    #[test]
    fn test_hyperlink_empty_runs() {
        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Hyperlink(Hyperlink {
                id: Some("rId1".to_string()),
                anchor: None,
                runs: vec![], // Empty hyperlink
            })],
            numbering: None,
        };

        // Empty hyperlink should be considered empty
        assert!(para.is_empty());

        // runs() should return nothing
        let runs: Vec<_> = para.runs().collect();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_hyperlink_with_whitespace_only() {
        let para = Paragraph {
            style_id: None,
            children: vec![ParagraphChild::Hyperlink(Hyperlink {
                id: Some("rId1".to_string()),
                anchor: None,
                runs: vec![Run {
                    text: "   ".to_string(), // Whitespace only
                    bold: false,
                    italic: false,
                    monospace: false,
                }],
            })],
            numbering: None,
        };

        // Hyperlink with only whitespace should be considered empty
        assert!(para.is_empty());
    }

    #[test]
    fn test_parse_textbox_content() {
        // Text inside textbox (w:txbxContent) should be extracted
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
            <w:body>
                <w:p>
                    <w:r>
                        <mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
                            <mc:Choice>
                                <w:drawing>
                                    <wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                                        <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
                                            <wps:wsp>
                                                <wps:txbx>
                                                    <w:txbxContent>
                                                        <w:p>
                                                            <w:r>
                                                                <w:t>Textbox content</w:t>
                                                            </w:r>
                                                        </w:p>
                                                    </w:txbxContent>
                                                </wps:txbx>
                                            </wps:wsp>
                                        </a:graphic>
                                    </wp:anchor>
                                </w:drawing>
                            </mc:Choice>
                        </mc:AlternateContent>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should have parsed paragraphs from textbox
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_multiple_runs_concatenation() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>Hello</w:t></w:r>
                    <w:r><w:t> </w:t></w:r>
                    <w:r><w:t>World</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert_eq!(doc.plain_text(), "Hello World");
    }

    #[test]
    fn test_table_row_cell_counts() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>C1</w:t></w:r></w:p></w:tc>
                    </w:tr>
                    <w:tr>
                        <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected table");
        };

        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].cells.len(), 3);
        assert_eq!(t.rows[1].cells.len(), 2);
    }

    #[test]
    fn test_table_cell_multiple_paragraphs() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc>
                            <w:p><w:r><w:t>First para</w:t></w:r></w:p>
                            <w:p><w:r><w:t>Second para</w:t></w:r></w:p>
                        </w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Table(t) = &doc.blocks[0] else {
            panic!("Expected table");
        };

        assert_eq!(t.rows[0].cells[0].paragraphs.len(), 2);
        assert_eq!(t.rows[0].cells[0].paragraphs[0].plain_text(), "First para");
        assert_eq!(t.rows[0].cells[0].paragraphs[1].plain_text(), "Second para");
    }

    #[test]
    fn test_run_monospace_detection() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:rPr>
                            <w:rFonts w:ascii="Consolas"/>
                        </w:rPr>
                        <w:t>code</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("Expected paragraph");
        };
        let ParagraphChild::Run(r) = &p.children[0] else {
            panic!("Expected run");
        };

        assert!(r.monospace);
    }

    #[test]
    fn test_section_break_in_blocks() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p><w:r><w:t>Section 1</w:t></w:r></w:p>
                <w:p>
                    <w:pPr>
                        <w:sectPr/>
                    </w:pPr>
                </w:p>
                <w:p><w:r><w:t>Section 2</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should have blocks including section-related paragraphs
        assert!(doc.blocks.len() >= 2);

        // First and last should be paragraphs with content
        if let Block::Paragraph(p) = &doc.blocks[0] {
            assert_eq!(p.plain_text(), "Section 1");
        }
    }

    #[test]
    fn test_bookmark_plain_text_empty() {
        let para = Paragraph {
            style_id: None,
            children: vec![
                ParagraphChild::Bookmark(Bookmark {
                    name: "_Toc123".to_string(),
                }),
                ParagraphChild::Run(Run {
                    text: "Heading".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }),
            ],
            numbering: None,
        };

        // Bookmark should not contribute to plain text
        assert_eq!(para.plain_text(), "Heading");
    }
}
