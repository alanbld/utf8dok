//! Document extraction (docx → AsciiDoc)
//!
//! This module converts OOXML documents to AsciiDoc format,
//! preserving structure and generating style mappings.

use std::fmt::Write;
use std::path::Path;

use crate::archive::OoxmlArchive;
use crate::document::{Block, Document, Hyperlink, Paragraph, ParagraphChild, Run, Table};
use crate::error::Result;
use crate::relationships::Relationships;
use crate::styles::StyleSheet;

/// Result of extracting a document
#[derive(Debug)]
pub struct ExtractedDocument {
    /// The generated AsciiDoc content
    pub asciidoc: String,
    /// The detected style mappings (for utf8dok.toml)
    pub style_mappings: StyleMappings,
    /// Document metadata extracted from properties
    pub metadata: DocumentMetadata,
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
#[derive(Debug, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
}

/// Extracts OOXML documents to AsciiDoc
pub struct AsciiDocExtractor {
    /// Include document attributes header
    pub include_header: bool,
    /// Detect and convert tables
    pub extract_tables: bool,
    /// Preserve inline formatting (bold, italic)
    pub preserve_formatting: bool,
}

impl Default for AsciiDocExtractor {
    fn default() -> Self {
        Self {
            include_header: true,
            extract_tables: true,
            preserve_formatting: true,
        }
    }
}

impl AsciiDocExtractor {
    /// Create a new extractor with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract a document from a file path
    pub fn extract_file<P: AsRef<Path>>(&self, path: P) -> Result<ExtractedDocument> {
        let archive = OoxmlArchive::open(path)?;
        self.extract_archive(&archive)
    }

    /// Extract from an already-opened archive
    pub fn extract_archive(&self, archive: &OoxmlArchive) -> Result<ExtractedDocument> {
        let document = Document::parse(archive.document_xml()?)?;
        let styles = StyleSheet::parse(archive.styles_xml()?)?;

        // Load relationships for hyperlink resolution
        let relationships = archive
            .document_rels_xml()
            .and_then(|xml| Relationships::parse(xml).ok());

        let style_mappings = self.detect_style_mappings(&styles);
        let metadata = DocumentMetadata::default(); // TODO: parse docProps/core.xml

        let asciidoc = self.convert_to_asciidoc(&document, &styles, relationships.as_ref());

        Ok(ExtractedDocument {
            asciidoc,
            style_mappings,
            metadata,
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

    /// Convert document to AsciiDoc string
    fn convert_to_asciidoc(
        &self,
        document: &Document,
        styles: &StyleSheet,
        rels: Option<&Relationships>,
    ) -> String {
        let mut output = String::new();
        let mut first_heading_found = false;

        for block in &document.blocks {
            match block {
                Block::Paragraph(para) => {
                    if para.is_empty() {
                        continue;
                    }

                    let text = self.convert_paragraph_with_rels(para, rels);
                    
                    // Check if this is a heading
                    if let Some(ref style_id) = para.style_id {
                        if let Some(level) = styles.heading_level(style_id) {
                            // First heading might be document title
                            if !first_heading_found && level == 1 && self.include_header {
                                writeln!(output, "= {}", text.trim()).unwrap();
                                writeln!(output).unwrap();
                                first_heading_found = true;
                            } else {
                                let prefix = "=".repeat(level as usize + 1);
                                writeln!(output, "{} {}", prefix, text.trim()).unwrap();
                                writeln!(output).unwrap();
                            }
                            continue;
                        }
                    }

                    // Regular paragraph
                    if !text.trim().is_empty() {
                        writeln!(output, "{}", text.trim()).unwrap();
                        writeln!(output).unwrap();
                    }
                }
                Block::Table(table) if self.extract_tables => {
                    let table_text = self.convert_table(table);
                    writeln!(output, "{}", table_text).unwrap();
                    writeln!(output).unwrap();
                }
                Block::Table(_) => {
                    writeln!(output, "// [TABLE OMITTED]").unwrap();
                    writeln!(output).unwrap();
                }
                Block::SectionBreak => {
                    writeln!(output, "'''").unwrap();
                    writeln!(output).unwrap();
                }
            }
        }

        output
    }

    /// Convert a paragraph to AsciiDoc text
    fn convert_paragraph(&self, para: &Paragraph) -> String {
        self.convert_paragraph_with_rels(para, None)
    }

    /// Convert a paragraph to AsciiDoc text with relationship resolution
    fn convert_paragraph_with_rels(&self, para: &Paragraph, rels: Option<&Relationships>) -> String {
        let mut result = String::new();

        for child in &para.children {
            match child {
                ParagraphChild::Run(run) => {
                    result.push_str(&self.convert_run(run));
                }
                ParagraphChild::Hyperlink(hyperlink) => {
                    result.push_str(&self.convert_hyperlink(hyperlink, rels));
                }
            }
        }

        result
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

    /// Convert a hyperlink to AsciiDoc format
    fn convert_hyperlink(&self, hyperlink: &Hyperlink, rels: Option<&Relationships>) -> String {
        // Get the link text from the runs
        let text: String = hyperlink
            .runs
            .iter()
            .map(|r| self.convert_run(r))
            .collect();

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
        let col_count = table
            .rows
            .first()
            .map(|r| r.cells.len())
            .unwrap_or(0);

        if col_count == 0 {
            return output;
        }

        // Table header
        writeln!(output, "[cols=\"{}\", options=\"header\"]", vec!["1"; col_count].join(",")).unwrap();
        writeln!(output, "|===").unwrap();

        for (row_idx, row) in table.rows.iter().enumerate() {
            // First row as header
            if row_idx == 0 || row.is_header {
                for cell in &row.cells {
                    let text = cell
                        .paragraphs
                        .iter()
                        .map(|p| self.convert_paragraph(p))
                        .collect::<Vec<_>>()
                        .join(" ");
                    writeln!(output, "|{}", text.trim()).unwrap();
                }
            } else {
                for cell in &row.cells {
                    let text = cell
                        .paragraphs
                        .iter()
                        .map(|p| self.convert_paragraph(p))
                        .collect::<Vec<_>>()
                        .join(" ");
                    write!(output, "|{} ", text.trim()).unwrap();
                }
                writeln!(output).unwrap();
            }

            // Blank line after header row
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

        let extractor = AsciiDocExtractor::new();
        let asciidoc = extractor.convert_to_asciidoc(&doc, &styles, None);

        assert!(asciidoc.contains("Hello, world!"));
    }

    #[test]
    fn test_style_mappings_to_toml() {
        let mappings = StyleMappings {
            headings: vec![
                (1, "Heading1".to_string()),
                (2, "Heading2".to_string()),
            ],
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

        let extractor = AsciiDocExtractor::new();
        let asciidoc = extractor.convert_to_asciidoc(&doc, &styles, None);

        println!("Generated AsciiDoc:\n{}", asciidoc);
        // Should generate: <<_Toc123,Click me>>
        assert!(
            asciidoc.contains("<<_Toc123,Click me>>"),
            "Expected <<_Toc123,Click me>> but got: {}",
            asciidoc
        );
    }
}
