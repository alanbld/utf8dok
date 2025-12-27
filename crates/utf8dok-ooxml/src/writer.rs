//! DOCX Writer
//!
//! This module writes `utf8dok_ast::Document` to DOCX format using a template.
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ooxml::writer::DocxWriter;
//! use utf8dok_ast::Document;
//!
//! let doc = Document::new();
//! let template = std::fs::read("template.dotx")?;
//! let output = DocxWriter::generate(&doc, &template)?;
//! std::fs::write("output.docx", output)?;
//! ```

use std::io::Cursor;

use utf8dok_ast::{
    Block, Document, FormatType, Heading, Inline, List, ListItem, ListType, Paragraph, Table,
};

use crate::archive::OoxmlArchive;
use crate::error::Result;

/// Default paragraph style when none is specified
const DEFAULT_PARAGRAPH_STYLE: &str = "Normal";

/// DOCX Writer for generating DOCX files from AST
pub struct DocxWriter {
    /// XML output buffer
    output: String,
}

impl DocxWriter {
    /// Create a new DocxWriter
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    /// Generate a DOCX file from an AST Document using a template
    ///
    /// # Arguments
    ///
    /// * `doc` - The AST document to convert
    /// * `template` - The template DOCX file as bytes
    ///
    /// # Returns
    ///
    /// The generated DOCX file as bytes
    pub fn generate(doc: &Document, template: &[u8]) -> Result<Vec<u8>> {
        // Load the template archive
        let cursor = Cursor::new(template);
        let mut archive = OoxmlArchive::from_reader(cursor)?;

        // Generate the document XML
        let mut writer = DocxWriter::new();
        let document_xml = writer.generate_document_xml(doc);

        // Replace word/document.xml in the archive
        archive.set_string("word/document.xml", document_xml);

        // Write to output buffer
        let mut output = Cursor::new(Vec::new());
        archive.write_to(&mut output)?;

        Ok(output.into_inner())
    }

    /// Generate the complete document.xml content
    fn generate_document_xml(&mut self, doc: &Document) -> String {
        self.output.clear();

        // XML declaration and document root
        self.output.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        self.output.push('\n');
        self.output.push_str(r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" "#);
        self.output.push_str(r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
        self.output.push('\n');
        self.output.push_str("<w:body>\n");

        // Generate blocks
        for block in &doc.blocks {
            self.generate_block(block);
        }

        // Close body and document
        self.output.push_str("</w:body>\n");
        self.output.push_str("</w:document>");

        self.output.clone()
    }

    /// Generate XML for a single block
    fn generate_block(&mut self, block: &Block) {
        match block {
            Block::Paragraph(para) => self.generate_paragraph(para),
            Block::Heading(heading) => self.generate_heading(heading),
            Block::List(list) => self.generate_list(list),
            Block::Table(table) => self.generate_table(table),
            Block::Break(break_type) => self.generate_break(break_type),
            Block::Literal(literal) => self.generate_literal(literal),
            Block::Admonition(admon) => self.generate_admonition(admon),
        }
    }

    /// Generate XML for a paragraph
    fn generate_paragraph(&mut self, para: &Paragraph) {
        self.output.push_str("<w:p>\n");

        // Paragraph properties (style)
        let style = para
            .style_id
            .as_deref()
            .unwrap_or(DEFAULT_PARAGRAPH_STYLE);
        self.output.push_str("<w:pPr>\n");
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));
        self.output.push_str("</w:pPr>\n");

        // Generate runs for inline content
        for inline in &para.inlines {
            self.generate_inline(inline);
        }

        self.output.push_str("</w:p>\n");
    }

    /// Generate XML for a heading
    fn generate_heading(&mut self, heading: &Heading) {
        self.output.push_str("<w:p>\n");

        // Heading style based on level or explicit style_id
        let style = heading.style_id.as_deref().unwrap_or(match heading.level {
            1 => "Heading1",
            2 => "Heading2",
            3 => "Heading3",
            4 => "Heading4",
            5 => "Heading5",
            6 => "Heading6",
            _ => "Heading1",
        });

        self.output.push_str("<w:pPr>\n");
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));
        self.output.push_str("</w:pPr>\n");

        // Generate runs for heading text
        for inline in &heading.text {
            self.generate_inline(inline);
        }

        self.output.push_str("</w:p>\n");
    }

    /// Generate XML for a list
    fn generate_list(&mut self, list: &List) {
        for item in &list.items {
            self.generate_list_item(item, &list.list_type, list.style_id.as_deref());
        }
    }

    /// Generate XML for a list item
    fn generate_list_item(
        &mut self,
        item: &ListItem,
        list_type: &ListType,
        style_id: Option<&str>,
    ) {
        // Each list item becomes a paragraph with list properties
        for block in &item.content {
            if let Block::Paragraph(para) = block {
                self.output.push_str("<w:p>\n");
                self.output.push_str("<w:pPr>\n");

                // Use style_id if provided, otherwise default list style
                let style = style_id.unwrap_or(match list_type {
                    ListType::Unordered => "ListBullet",
                    ListType::Ordered => "ListNumber",
                    ListType::Description => "ListParagraph",
                });
                self.output
                    .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));

                // List numbering properties
                self.output.push_str("<w:numPr>\n");
                self.output.push_str(&format!(
                    "<w:ilvl w:val=\"{}\"/>\n",
                    item.level
                ));
                // numId would need to reference numbering.xml; use 1 as default
                let num_id = match list_type {
                    ListType::Unordered => 1,
                    ListType::Ordered => 2,
                    ListType::Description => 3,
                };
                self.output
                    .push_str(&format!("<w:numId w:val=\"{}\"/>\n", num_id));
                self.output.push_str("</w:numPr>\n");

                self.output.push_str("</w:pPr>\n");

                // Generate runs for content
                for inline in &para.inlines {
                    self.generate_inline(inline);
                }

                self.output.push_str("</w:p>\n");
            } else {
                // For non-paragraph content, generate as normal block
                self.generate_block(block);
            }
        }
    }

    /// Generate XML for a table
    fn generate_table(&mut self, table: &Table) {
        self.output.push_str("<w:tbl>\n");

        // Table properties
        self.output.push_str("<w:tblPr>\n");
        if let Some(style) = &table.style_id {
            self.output
                .push_str(&format!("<w:tblStyle w:val=\"{}\"/>\n", escape_xml(style)));
        }
        self.output.push_str("<w:tblW w:w=\"5000\" w:type=\"pct\"/>\n");
        self.output.push_str("</w:tblPr>\n");

        // Table grid (column definitions)
        if !table.columns.is_empty() {
            self.output.push_str("<w:tblGrid>\n");
            for _col in &table.columns {
                self.output.push_str("<w:gridCol w:w=\"2000\"/>\n");
            }
            self.output.push_str("</w:tblGrid>\n");
        }

        // Table rows
        for row in &table.rows {
            self.output.push_str("<w:tr>\n");

            // Row properties for header rows
            if row.is_header {
                self.output.push_str("<w:trPr>\n");
                self.output.push_str("<w:tblHeader/>\n");
                self.output.push_str("</w:trPr>\n");
            }

            // Cells
            for cell in &row.cells {
                self.output.push_str("<w:tc>\n");

                // Cell properties
                self.output.push_str("<w:tcPr>\n");
                if cell.colspan > 1 {
                    self.output
                        .push_str(&format!("<w:gridSpan w:val=\"{}\"/>\n", cell.colspan));
                }
                if cell.rowspan > 1 {
                    self.output.push_str("<w:vMerge w:val=\"restart\"/>\n");
                }
                self.output.push_str("</w:tcPr>\n");

                // Cell content (blocks)
                for block in &cell.content {
                    self.generate_block(block);
                }

                // Ensure at least one paragraph in cell
                if cell.content.is_empty() {
                    self.output.push_str("<w:p/>\n");
                }

                self.output.push_str("</w:tc>\n");
            }

            self.output.push_str("</w:tr>\n");
        }

        self.output.push_str("</w:tbl>\n");
    }

    /// Generate XML for a break
    fn generate_break(&mut self, break_type: &utf8dok_ast::BreakType) {
        self.output.push_str("<w:p>\n");
        self.output.push_str("<w:r>\n");
        match break_type {
            utf8dok_ast::BreakType::Page => {
                self.output.push_str("<w:br w:type=\"page\"/>\n");
            }
            utf8dok_ast::BreakType::Section => {
                // Section break requires sectPr
                self.output.push_str("</w:r>\n");
                self.output.push_str("<w:pPr>\n");
                self.output.push_str("<w:sectPr>\n");
                self.output.push_str("<w:type w:val=\"nextPage\"/>\n");
                self.output.push_str("</w:sectPr>\n");
                self.output.push_str("</w:pPr>\n");
                self.output.push_str("</w:p>\n");
                return;
            }
        }
        self.output.push_str("</w:r>\n");
        self.output.push_str("</w:p>\n");
    }

    /// Generate XML for a literal/code block
    fn generate_literal(&mut self, literal: &utf8dok_ast::LiteralBlock) {
        // Code blocks become paragraphs with monospace style
        self.output.push_str("<w:p>\n");
        self.output.push_str("<w:pPr>\n");
        let style = literal.style_id.as_deref().unwrap_or("CodeBlock");
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));
        self.output.push_str("</w:pPr>\n");

        // Generate the content as a run with preserved whitespace
        self.output.push_str("<w:r>\n");
        self.output.push_str("<w:rPr>\n");
        self.output.push_str("<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>\n");
        self.output.push_str("</w:rPr>\n");
        self.output.push_str(&format!(
            "<w:t xml:space=\"preserve\">{}</w:t>\n",
            escape_xml(&literal.content)
        ));
        self.output.push_str("</w:r>\n");

        self.output.push_str("</w:p>\n");
    }

    /// Generate XML for an admonition
    fn generate_admonition(&mut self, admonition: &utf8dok_ast::Admonition) {
        // Admonitions become paragraphs with a special style
        let type_name = match admonition.admonition_type {
            utf8dok_ast::AdmonitionType::Note => "Note",
            utf8dok_ast::AdmonitionType::Tip => "Tip",
            utf8dok_ast::AdmonitionType::Important => "Important",
            utf8dok_ast::AdmonitionType::Warning => "Warning",
            utf8dok_ast::AdmonitionType::Caution => "Caution",
        };

        // Title paragraph if present
        if let Some(title) = &admonition.title {
            self.output.push_str("<w:p>\n");
            self.output.push_str("<w:pPr>\n");
            self.output
                .push_str(&format!("<w:pStyle w:val=\"{}Title\"/>\n", type_name));
            self.output.push_str("</w:pPr>\n");
            for inline in title {
                self.generate_inline(inline);
            }
            self.output.push_str("</w:p>\n");
        }

        // Content blocks
        for block in &admonition.content {
            self.generate_block(block);
        }
    }

    /// Generate XML for inline content
    fn generate_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text(text) => {
                self.output.push_str("<w:r>\n");
                self.output
                    .push_str(&format!("<w:t>{}</w:t>\n", escape_xml(text)));
                self.output.push_str("</w:r>\n");
            }
            Inline::Format(format_type, inner) => {
                self.generate_formatted_inline(format_type, inner);
            }
            Inline::Span(inlines) => {
                for inline in inlines {
                    self.generate_inline(inline);
                }
            }
            Inline::Link(link) => {
                // Links require relationship handling; for now, output as styled text
                self.output.push_str("<w:r>\n");
                self.output.push_str("<w:rPr>\n");
                self.output.push_str("<w:color w:val=\"0000FF\"/>\n");
                self.output.push_str("<w:u w:val=\"single\"/>\n");
                self.output.push_str("</w:rPr>\n");
                for text_inline in &link.text {
                    if let Inline::Text(text) = text_inline {
                        self.output
                            .push_str(&format!("<w:t>{}</w:t>\n", escape_xml(text)));
                    }
                }
                self.output.push_str("</w:r>\n");
            }
            Inline::Image(_image) => {
                // Images require relationship and drawing handling; placeholder for now
                self.output.push_str("<w:r>\n");
                self.output.push_str("<w:t>[Image]</w:t>\n");
                self.output.push_str("</w:r>\n");
            }
            Inline::Break => {
                self.output.push_str("<w:r>\n");
                self.output.push_str("<w:br/>\n");
                self.output.push_str("</w:r>\n");
            }
        }
    }

    /// Generate XML for formatted inline content
    fn generate_formatted_inline(&mut self, format_type: &FormatType, inner: &Inline) {
        self.output.push_str("<w:r>\n");
        self.output.push_str("<w:rPr>\n");

        // Apply formatting
        match format_type {
            FormatType::Bold => {
                self.output.push_str("<w:b/>\n");
            }
            FormatType::Italic => {
                self.output.push_str("<w:i/>\n");
            }
            FormatType::Monospace => {
                self.output.push_str("<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>\n");
            }
            FormatType::Highlight => {
                self.output.push_str("<w:highlight w:val=\"yellow\"/>\n");
            }
            FormatType::Superscript => {
                self.output.push_str("<w:vertAlign w:val=\"superscript\"/>\n");
            }
            FormatType::Subscript => {
                self.output.push_str("<w:vertAlign w:val=\"subscript\"/>\n");
            }
        }

        self.output.push_str("</w:rPr>\n");

        // Extract text from inner inline
        let text = extract_text(inner);
        self.output
            .push_str(&format!("<w:t>{}</w:t>\n", escape_xml(&text)));

        self.output.push_str("</w:r>\n");
    }
}

/// Extract plain text from an inline element
fn extract_text(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => text.clone(),
        Inline::Format(_, inner) => extract_text(inner),
        Inline::Span(inlines) => inlines.iter().map(extract_text).collect(),
        Inline::Link(link) => link.text.iter().map(extract_text).collect(),
        Inline::Image(image) => image.alt.clone().unwrap_or_default(),
        Inline::Break => String::new(),
    }
}

/// Escape special XML characters
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

    /// Create a minimal valid DOCX template for testing
    fn create_minimal_template() -> Vec<u8> {
        use zip::write::SimpleFileOptions;
        use zip::CompressionMethod;
        use zip::ZipWriter;

        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();

        // _rels/.rels
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).unwrap();

        // word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#).unwrap();

        // word/document.xml (placeholder, will be replaced)
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Template</w:t></w:r></w:p>
  </w:body>
</w:document>"#).unwrap();

        zip.finish().unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_write_basic_doc() {
        let template = create_minimal_template();

        // Create a simple document
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Hello World".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![
                        Inline::Text("This is a ".to_string()),
                        Inline::Format(FormatType::Bold, Box::new(Inline::Text("test".to_string()))),
                        Inline::Text(" document.".to_string()),
                    ],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
            ],
        };

        // Generate DOCX
        let result = DocxWriter::generate(&doc, &template);
        assert!(result.is_ok(), "Failed to generate DOCX: {:?}", result.err());

        let output = result.unwrap();

        // Verify it's a valid ZIP
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor);
        assert!(archive.is_ok(), "Output is not a valid ZIP");

        let archive = archive.unwrap();

        // Verify word/document.xml exists and contains our content
        let doc_xml = archive.get_string("word/document.xml").unwrap();
        assert!(doc_xml.is_some(), "word/document.xml not found");

        let doc_xml = doc_xml.unwrap();
        assert!(doc_xml.contains("Hello World"), "Heading text not found");
        assert!(doc_xml.contains("<w:b/>"), "Bold formatting not found");
        assert!(doc_xml.contains("test"), "Bold text not found");
        assert!(doc_xml.contains("document"), "Paragraph text not found");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello & World"), "Hello &amp; World");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_write_list() {
        let template = create_minimal_template();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![Block::List(List {
                list_type: ListType::Unordered,
                items: vec![
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("First item".to_string())],
                            style_id: None,
                            attributes: HashMap::new(),
                        })],
                        level: 0,
                        term: None,
                    },
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Second item".to_string())],
                            style_id: None,
                            attributes: HashMap::new(),
                        })],
                        level: 0,
                        term: None,
                    },
                ],
                style_id: None,
            })],
        };

        let result = DocxWriter::generate(&doc, &template);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        assert!(doc_xml.contains("First item"));
        assert!(doc_xml.contains("Second item"));
        assert!(doc_xml.contains("w:numPr"));
    }

    #[test]
    fn test_write_table() {
        let template = create_minimal_template();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![Block::Table(Table {
                rows: vec![
                    utf8dok_ast::TableRow {
                        cells: vec![
                            utf8dok_ast::TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Header 1".to_string())],
                                    style_id: None,
                                    attributes: HashMap::new(),
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                            utf8dok_ast::TableCell {
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
                    utf8dok_ast::TableRow {
                        cells: vec![
                            utf8dok_ast::TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Cell 1".to_string())],
                                    style_id: None,
                                    attributes: HashMap::new(),
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                            utf8dok_ast::TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Cell 2".to_string())],
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
                style_id: Some("TableGrid".to_string()),
                caption: None,
                columns: vec![],
            })],
        };

        let result = DocxWriter::generate(&doc, &template);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        assert!(doc_xml.contains("<w:tbl>"));
        assert!(doc_xml.contains("Header 1"));
        assert!(doc_xml.contains("Cell 2"));
        assert!(doc_xml.contains("<w:tblHeader/>"));
    }

    #[test]
    fn test_write_formatted_text() {
        let template = create_minimal_template();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![
                    Inline::Format(FormatType::Bold, Box::new(Inline::Text("bold".to_string()))),
                    Inline::Text(" ".to_string()),
                    Inline::Format(FormatType::Italic, Box::new(Inline::Text("italic".to_string()))),
                    Inline::Text(" ".to_string()),
                    Inline::Format(FormatType::Monospace, Box::new(Inline::Text("mono".to_string()))),
                ],
                style_id: None,
                attributes: HashMap::new(),
            })],
        };

        let result = DocxWriter::generate(&doc, &template);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        assert!(doc_xml.contains("<w:b/>"));
        assert!(doc_xml.contains("<w:i/>"));
        assert!(doc_xml.contains("Courier New"));
    }
}
