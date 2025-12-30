//! DOCX Writer
//!
//! This module writes `utf8dok_ast::Document` to DOCX format using a template.
//! It supports rendering diagrams (Mermaid, PlantUML, etc.) as embedded images.
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ooxml::writer::DocxWriter;
//! use utf8dok_ooxml::Template;
//! use utf8dok_ast::Document;
//!
//! let doc = Document::new();
//! let template = Template::load("template.dotx")?;
//! let output = DocxWriter::generate_from_template(&doc, template)?;
//! std::fs::write("output.docx", output)?;
//! ```

use std::io::Cursor;

use sha2::{Digest, Sha256};
use utf8dok_ast::{
    Block, Document, FormatType, Heading, Inline, List, ListItem, ListType, Paragraph, Table,
};
use utf8dok_diagrams::{DiagramEngine, DiagramType};

use crate::archive::OoxmlArchive;
use crate::error::Result;
use crate::manifest::{ElementMeta, Manifest};
use crate::relationships::Relationships;
use crate::styles::StyleMap;
use crate::template::Template;

/// Known diagram style IDs that should be rendered as images
const DIAGRAM_STYLES: &[&str] = &[
    "mermaid",
    "plantuml",
    "graphviz",
    "dot",
    "d2",
    "ditaa",
    "blockdiag",
    "seqdiag",
    "actdiag",
    "nwdiag",
    "c4plantuml",
    "erd",
    "nomnoml",
    "pikchr",
    "structurizr",
    "vega",
    "vegalite",
    "wavedrom",
    "svgbob", // Native rendering support
];

/// A comment to be added to the document
#[derive(Debug, Clone)]
struct Comment {
    /// Comment ID
    id: usize,
    /// Comment text
    text: String,
    /// Author name
    author: String,
}

/// DOCX Writer for generating DOCX files from AST
pub struct DocxWriter {
    /// XML output buffer
    output: String,
    /// Document relationships (word/_rels/document.xml.rels)
    relationships: Relationships,
    /// Media files to embed (path, bytes)
    media_files: Vec<(String, Vec<u8>)>,
    /// Diagram source files to embed (path, content)
    diagram_sources: Vec<(String, String)>,
    /// Document manifest
    manifest: Manifest,
    /// Next image ID for unique naming
    next_image_id: usize,
    /// Next drawing ID for docPr
    next_drawing_id: usize,
    /// Diagram engine for rendering (uses native + Kroki fallback)
    diagram_engine: Option<DiagramEngine>,
    /// Style mapping for template injection
    style_map: StyleMap,
    /// Original AsciiDoc source (for self-contained DOCX)
    source_text: Option<String>,
    /// Configuration TOML (for self-contained DOCX)
    config_text: Option<String>,
    /// Comments to be added to comments.xml
    comments: Vec<Comment>,
    /// Next comment ID
    next_comment_id: usize,
}

impl Default for DocxWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DocxWriter {
    /// Create a new DocxWriter
    pub fn new() -> Self {
        Self {
            output: String::new(),
            relationships: Relationships::new(),
            media_files: Vec::new(),
            diagram_sources: Vec::new(),
            manifest: Manifest::new(),
            next_image_id: 1,
            next_drawing_id: 1,
            diagram_engine: None,
            style_map: StyleMap::default(),
            source_text: None,
            config_text: None,
            comments: Vec::new(),
            next_comment_id: 1,
        }
    }

    /// Create a new DocxWriter with a custom style map
    fn with_style_map(style_map: StyleMap) -> Self {
        Self {
            output: String::new(),
            relationships: Relationships::new(),
            media_files: Vec::new(),
            diagram_sources: Vec::new(),
            manifest: Manifest::new(),
            next_image_id: 1,
            next_drawing_id: 1,
            diagram_engine: None,
            style_map,
            source_text: None,
            config_text: None,
            comments: Vec::new(),
            next_comment_id: 1,
        }
    }

    /// Set the original AsciiDoc source text to embed in the DOCX
    ///
    /// This enables round-trip editing - the source can be extracted later.
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source_text = Some(source.into());
    }

    /// Set the configuration TOML to embed in the DOCX
    pub fn set_config(&mut self, config: impl Into<String>) {
        self.config_text = Some(config.into());
    }

    /// Set both source and config at once
    pub fn set_embedded_content(&mut self, source: impl Into<String>, config: impl Into<String>) {
        self.source_text = Some(source.into());
        self.config_text = Some(config.into());
    }

    /// Initialize the writer from a template archive
    fn init_from_template(&mut self, archive: &OoxmlArchive) -> Result<()> {
        // Parse existing relationships from template
        if let Some(rels_xml) = archive.get("word/_rels/document.xml.rels") {
            self.relationships = Relationships::parse(rels_xml)?;
        }

        // Parse existing manifest if present
        if let Some(manifest_bytes) = archive.read_utf8dok_file("manifest.json") {
            self.manifest = Manifest::from_json_bytes(manifest_bytes)?;
        }

        Ok(())
    }

    /// Write embedded content (source, config) to the archive
    ///
    /// This makes the DOCX self-contained for round-trip editing.
    fn write_embedded_content(&mut self, archive: &mut OoxmlArchive) -> Result<()> {
        // Write source file if present
        if let Some(ref source) = self.source_text {
            let source_path = "utf8dok/source.adoc";
            archive.set_string(source_path, source.clone());

            // Compute hash for manifest
            let mut hasher = Sha256::new();
            hasher.update(source.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            // Add to manifest
            self.manifest.add_element(
                "source".to_string(),
                ElementMeta::new("source")
                    .with_source(source_path.to_string())
                    .with_hash(hash)
                    .with_description("Original AsciiDoc source".to_string()),
            );
        }

        // Write config file if present
        if let Some(ref config) = self.config_text {
            let config_path = "utf8dok/utf8dok.toml";
            archive.set_string(config_path, config.clone());

            // Compute hash for manifest
            let mut hasher = Sha256::new();
            hasher.update(config.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            // Add to manifest
            self.manifest.add_element(
                "config".to_string(),
                ElementMeta::new("config")
                    .with_source(config_path.to_string())
                    .with_hash(hash)
                    .with_description("utf8dok configuration".to_string()),
            );
        }

        Ok(())
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
        Self::generate_with_options(doc, template, true)
    }

    /// Generate a DOCX file using instance settings (source, config, style_map)
    ///
    /// This method allows setting source/config before generation for self-contained DOCX.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use utf8dok_ooxml::{DocxWriter, Template};
    ///
    /// let mut writer = DocxWriter::new();
    /// writer.set_source(&adoc_content);
    /// writer.set_config(&config_toml);
    ///
    /// let template = Template::load("template.dotx")?;
    /// let output = writer.generate_with_template(&doc, template)?;
    /// ```
    pub fn generate_with_template(self, doc: &Document, template: Template) -> Result<Vec<u8>> {
        self.generate_with_template_options(doc, template, true, None)
    }

    /// Generate a DOCX file with full options using instance settings
    pub fn generate_with_template_options(
        mut self,
        doc: &Document,
        mut template: Template,
        render_diagrams: bool,
        custom_style_map: Option<StyleMap>,
    ) -> Result<Vec<u8>> {
        // Get styles from template and create style map
        let stylesheet = template.get_styles()?;
        let style_map = custom_style_map.unwrap_or_else(|| StyleMap::from_stylesheet(stylesheet));
        self.style_map = style_map;

        // Get the underlying archive (consume template)
        let mut archive = template.into_archive();

        // Initialize from template
        self.init_from_template(&archive)?;

        // Initialize diagram engine if rendering diagrams
        if render_diagrams {
            self.diagram_engine = Some(DiagramEngine::new());
        }

        // Generate the document XML
        let document_xml = self.generate_document_xml(doc);

        // Write word/document.xml
        archive.set_string("word/document.xml", document_xml);

        // Write word/_rels/document.xml.rels
        archive.set_string("word/_rels/document.xml.rels", self.relationships.to_xml());

        // Write media files
        for (path, data) in &self.media_files {
            archive.set(path.clone(), data.clone());
        }

        // Write diagram source files
        for (path, content) in &self.diagram_sources {
            archive.set(path.clone(), content.as_bytes().to_vec());
        }

        // Write embedded content (source, config) for self-contained DOCX
        self.write_embedded_content(&mut archive)?;

        // Write manifest if we have tracked elements
        if !self.manifest.is_empty() {
            let manifest_json = self.manifest.to_json()?;
            archive.set_string("utf8dok/manifest.json", manifest_json);
        }

        // Update [Content_Types].xml to include PNG if we have images
        if !self.media_files.is_empty() {
            self.update_content_types(&mut archive)?;
        }

        // Write comments.xml if we have any language annotations
        self.write_comments(&mut archive)?;

        // Update docProps/core.xml with document metadata (title, author)
        self.update_core_properties(&mut archive, doc)?;

        // Write to output buffer
        let mut output = Cursor::new(Vec::new());
        archive.write_to(&mut output)?;

        Ok(output.into_inner())
    }

    /// Generate a DOCX file with options
    ///
    /// # Arguments
    ///
    /// * `doc` - The AST document to convert
    /// * `template` - The template DOCX file as bytes
    /// * `render_diagrams` - Whether to render diagrams via Kroki
    pub fn generate_with_options(
        doc: &Document,
        template: &[u8],
        render_diagrams: bool,
    ) -> Result<Vec<u8>> {
        // Load the template archive
        let cursor = Cursor::new(template);
        let mut archive = OoxmlArchive::from_reader(cursor)?;

        // Initialize writer from template
        let mut writer = DocxWriter::new();
        writer.init_from_template(&archive)?;

        // Initialize diagram engine if rendering diagrams
        if render_diagrams {
            writer.diagram_engine = Some(DiagramEngine::new());
        }

        // Generate the document XML
        let document_xml = writer.generate_document_xml(doc);

        // Write word/document.xml
        archive.set_string("word/document.xml", document_xml);

        // Write word/_rels/document.xml.rels
        archive.set_string(
            "word/_rels/document.xml.rels",
            writer.relationships.to_xml(),
        );

        // Write media files
        for (path, data) in &writer.media_files {
            archive.set(path.clone(), data.clone());
        }

        // Write diagram source files
        for (path, content) in &writer.diagram_sources {
            archive.set(path.clone(), content.as_bytes().to_vec());
        }

        // Write embedded content (source, config) for self-contained DOCX
        writer.write_embedded_content(&mut archive)?;

        // Write manifest if we have tracked elements
        if !writer.manifest.is_empty() {
            let manifest_json = writer.manifest.to_json()?;
            archive.set_string("utf8dok/manifest.json", manifest_json);
        }

        // Update [Content_Types].xml to include PNG if we have images
        if !writer.media_files.is_empty() {
            writer.update_content_types(&mut archive)?;
        }

        // Write comments.xml if we have any language annotations
        writer.write_comments(&mut archive)?;

        // Update docProps/core.xml with document metadata (title, author)
        writer.update_core_properties(&mut archive, doc)?;

        // Write to output buffer
        let mut output = Cursor::new(Vec::new());
        archive.write_to(&mut output)?;

        Ok(output.into_inner())
    }

    /// Generate a DOCX file from an AST Document using a Template object
    ///
    /// This is the preferred method for template-aware document generation.
    /// It automatically detects styles from the template and uses them.
    ///
    /// # Arguments
    ///
    /// * `doc` - The AST document to convert
    /// * `template` - The loaded Template object (consumed)
    ///
    /// # Returns
    ///
    /// The generated DOCX file as bytes
    pub fn generate_from_template(doc: &Document, template: Template) -> Result<Vec<u8>> {
        Self::generate_from_template_with_options(doc, template, true, None)
    }

    /// Generate a DOCX file from an AST Document with custom options
    ///
    /// # Arguments
    ///
    /// * `doc` - The AST document to convert
    /// * `template` - The loaded Template object (consumed)
    /// * `render_diagrams` - Whether to render diagrams
    /// * `custom_style_map` - Optional custom style mapping (uses auto-detected if None)
    pub fn generate_from_template_with_options(
        doc: &Document,
        mut template: Template,
        render_diagrams: bool,
        custom_style_map: Option<StyleMap>,
    ) -> Result<Vec<u8>> {
        // Get styles from template and create style map
        let stylesheet = template.get_styles()?;
        let style_map = custom_style_map.unwrap_or_else(|| StyleMap::from_stylesheet(stylesheet));

        // Get the underlying archive (consume template)
        let mut archive = template.into_archive();

        // Initialize writer with style map
        let mut writer = DocxWriter::with_style_map(style_map);
        writer.init_from_template(&archive)?;

        // Initialize diagram engine if rendering diagrams
        if render_diagrams {
            writer.diagram_engine = Some(DiagramEngine::new());
        }

        // Generate the document XML
        let document_xml = writer.generate_document_xml(doc);

        // Write word/document.xml
        archive.set_string("word/document.xml", document_xml);

        // Write word/_rels/document.xml.rels
        archive.set_string(
            "word/_rels/document.xml.rels",
            writer.relationships.to_xml(),
        );

        // Write media files
        for (path, data) in &writer.media_files {
            archive.set(path.clone(), data.clone());
        }

        // Write diagram source files
        for (path, content) in &writer.diagram_sources {
            archive.set(path.clone(), content.as_bytes().to_vec());
        }

        // Write embedded content (source, config) for self-contained DOCX
        writer.write_embedded_content(&mut archive)?;

        // Write manifest if we have tracked elements
        if !writer.manifest.is_empty() {
            let manifest_json = writer.manifest.to_json()?;
            archive.set_string("utf8dok/manifest.json", manifest_json);
        }

        // Update [Content_Types].xml to include PNG if we have images
        if !writer.media_files.is_empty() {
            writer.update_content_types(&mut archive)?;
        }

        // Write comments.xml if we have any language annotations
        writer.write_comments(&mut archive)?;

        // Update docProps/core.xml with document metadata (title, author)
        writer.update_core_properties(&mut archive, doc)?;

        // Write to output buffer
        let mut output = Cursor::new(Vec::new());
        archive.write_to(&mut output)?;

        Ok(output.into_inner())
    }

    /// Update docProps/core.xml with document metadata
    fn update_core_properties(&self, archive: &mut OoxmlArchive, doc: &Document) -> Result<()> {
        // Get the document title and author from AST metadata
        let title = doc.metadata.title.as_deref();

        // Check both authors Vec and attributes for author
        let author = doc.metadata.authors.first().map(|s| s.as_str())
            .or_else(|| doc.metadata.attributes.get("author").map(|s| s.as_str()));

        // Check for revdate attribute
        let revdate = doc.metadata.revision.as_deref()
            .or_else(|| doc.metadata.attributes.get("revdate").map(|s| s.as_str()));

        // Only update if we have metadata to write
        if title.is_none() && author.is_none() && revdate.is_none() {
            return Ok(());
        }

        // Check if docProps/core.xml exists
        if let Some(core_xml) = archive.get_string("docProps/core.xml")? {
            // Update existing core.xml
            let mut updated = core_xml;

            if let Some(new_title) = title {
                // Replace existing title or insert one
                if updated.contains("<dc:title>") {
                    updated = updated
                        .split("<dc:title>")
                        .enumerate()
                        .map(|(i, part)| {
                            if i == 0 {
                                part.to_string()
                            } else if let Some((_, rest)) = part.split_once("</dc:title>") {
                                format!("<dc:title>{}</dc:title>{}", escape_xml(new_title), rest)
                            } else {
                                part.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                } else if updated.contains("<cp:coreProperties") {
                    // Insert title after opening tag
                    updated = updated.replace(
                        "</cp:coreProperties>",
                        &format!("<dc:title>{}</dc:title></cp:coreProperties>", escape_xml(new_title)),
                    );
                }
            }

            if let Some(new_author) = author {
                // Replace existing creator or insert one
                if updated.contains("<dc:creator>") {
                    updated = updated
                        .split("<dc:creator>")
                        .enumerate()
                        .map(|(i, part)| {
                            if i == 0 {
                                part.to_string()
                            } else if let Some((_, rest)) = part.split_once("</dc:creator>") {
                                format!("<dc:creator>{}</dc:creator>{}", escape_xml(new_author), rest)
                            } else {
                                part.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                } else if updated.contains("<cp:coreProperties") {
                    // Insert creator after opening tag
                    updated = updated.replace(
                        "</cp:coreProperties>",
                        &format!("<dc:creator>{}</dc:creator></cp:coreProperties>", escape_xml(new_author)),
                    );
                }
            }

            if let Some(new_revdate) = revdate {
                // Convert revdate to ISO format (add T00:00:00Z if just date)
                let iso_date = if new_revdate.contains('T') {
                    new_revdate.to_string()
                } else {
                    format!("{}T00:00:00Z", new_revdate)
                };

                // Replace existing modified date or insert one
                if updated.contains("<dcterms:modified") {
                    // Use regex-like replacement for the modified element (has xsi:type attribute)
                    if let Some(start) = updated.find("<dcterms:modified") {
                        if let Some(end) = updated[start..].find("</dcterms:modified>") {
                            let end_pos = start + end + "</dcterms:modified>".len();
                            let replacement = format!(
                                "<dcterms:modified xsi:type=\"dcterms:W3CDTF\">{}</dcterms:modified>",
                                iso_date
                            );
                            updated = format!("{}{}{}", &updated[..start], replacement, &updated[end_pos..]);
                        }
                    }
                } else if updated.contains("<cp:coreProperties") {
                    // Insert modified date before closing tag
                    updated = updated.replace(
                        "</cp:coreProperties>",
                        &format!(
                            "<dcterms:modified xsi:type=\"dcterms:W3CDTF\">{}</dcterms:modified></cp:coreProperties>",
                            iso_date
                        ),
                    );
                }
            }

            archive.set_string("docProps/core.xml", updated);
        } else {
            // Create new core.xml
            let mut core_xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            );

            if let Some(t) = title {
                core_xml.push_str(&format!("<dc:title>{}</dc:title>", escape_xml(t)));
            }
            if let Some(a) = author {
                core_xml.push_str(&format!("<dc:creator>{}</dc:creator>", escape_xml(a)));
            }
            if let Some(r) = revdate {
                let iso_date = if r.contains('T') {
                    r.to_string()
                } else {
                    format!("{}T00:00:00Z", r)
                };
                core_xml.push_str(&format!(
                    "<dcterms:modified xsi:type=\"dcterms:W3CDTF\">{}</dcterms:modified>",
                    iso_date
                ));
            }

            core_xml.push_str("</cp:coreProperties>");
            archive.set_string("docProps/core.xml", core_xml);
        }

        Ok(())
    }

    /// Generate comments.xml if there are any comments
    fn generate_comments_xml(&self) -> Option<String> {
        if self.comments.is_empty() {
            return None;
        }

        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        );

        for comment in &self.comments {
            xml.push_str(&format!(
                r#"
<w:comment w:id="{}" w:author="{}" w:date="2024-01-01T00:00:00Z">
<w:p><w:r><w:t>{}</w:t></w:r></w:p>
</w:comment>"#,
                comment.id,
                escape_xml(&comment.author),
                escape_xml(&comment.text)
            ));
        }

        xml.push_str("\n</w:comments>");
        Some(xml)
    }

    /// Write comments.xml and update relationships/content types
    fn write_comments(&self, archive: &mut OoxmlArchive) -> Result<()> {
        if let Some(comments_xml) = self.generate_comments_xml() {
            // Write comments.xml
            archive.set_string("word/comments.xml", comments_xml);

            // Update document relationships to include comments
            if let Some(rels) = archive.get_string("word/_rels/document.xml.rels")? {
                if !rels.contains("comments.xml") {
                    // Find the next rId
                    let next_rid = rels.matches("Id=\"rId").count() + 1;
                    let new_rels = rels.replace(
                        "</Relationships>",
                        &format!(
                            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>
</Relationships>"#,
                            next_rid
                        ),
                    );
                    archive.set_string("word/_rels/document.xml.rels", new_rels);
                }
            }

            // Update [Content_Types].xml to include comments
            if let Some(content_types) = archive.get_string("[Content_Types].xml")? {
                if !content_types.contains("comments.xml") {
                    let new_content_types = content_types.replace(
                        "</Types>",
                        r#"<Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>
</Types>"#,
                    );
                    archive.set_string("[Content_Types].xml", new_content_types);
                }
            }
        }
        Ok(())
    }

    /// Update [Content_Types].xml to include PNG extension
    fn update_content_types(&self, archive: &mut OoxmlArchive) -> Result<()> {
        if let Some(content_types) = archive.get_string("[Content_Types].xml")? {
            // Check if PNG is already defined
            if !content_types.contains("Extension=\"png\"") {
                // Add PNG content type before closing </Types>
                let new_content_types = content_types.replace(
                    "</Types>",
                    "  <Default Extension=\"png\" ContentType=\"image/png\"/>\n</Types>",
                );
                archive.set_string("[Content_Types].xml", new_content_types);
            }
        }
        Ok(())
    }

    /// Generate the complete document.xml content
    fn generate_document_xml(&mut self, doc: &Document) -> String {
        self.output.clear();

        // XML declaration and document root with all required namespaces
        self.output
            .push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        self.output.push('\n');
        self.output.push_str(r#"<w:document "#);
        self.output
            .push_str(r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" "#);
        self.output.push_str(
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" "#,
        );
        self.output.push_str(
            r#"xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" "#,
        );
        self.output
            .push_str(r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" "#);
        self.output
            .push_str(r#"xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#);
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

        // Paragraph properties (style) - use explicit style_id or mapped style
        let style = para
            .style_id
            .as_deref()
            .unwrap_or_else(|| self.style_map.paragraph());
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

        // Heading style based on level or explicit style_id - use style_map
        let style = heading
            .style_id
            .as_deref()
            .unwrap_or_else(|| self.style_map.heading(heading.level));

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

                // Use style_id if provided, otherwise use style_map
                let style = style_id.unwrap_or_else(|| match list_type {
                    ListType::Unordered => self.style_map.list(false),
                    ListType::Ordered => self.style_map.list(true),
                    ListType::Description => self
                        .style_map
                        .get(crate::styles::ElementType::ListDescription),
                });
                self.output
                    .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));

                // List numbering properties
                self.output.push_str("<w:numPr>\n");
                self.output
                    .push_str(&format!("<w:ilvl w:val=\"{}\"/>\n", item.level));
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

        // Table properties - use style_id or mapped table style
        self.output.push_str("<w:tblPr>\n");
        let style = table
            .style_id
            .as_deref()
            .unwrap_or_else(|| self.style_map.table());
        self.output
            .push_str(&format!("<w:tblStyle w:val=\"{}\"/>\n", escape_xml(style)));
        self.output
            .push_str("<w:tblW w:w=\"5000\" w:type=\"pct\"/>\n");
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
        // Check if this is a diagram block
        if let Some(style) = &literal.style_id {
            let style_lower = style.to_lowercase();
            if DIAGRAM_STYLES.contains(&style_lower.as_str()) && self.diagram_engine.is_some() {
                // Attempt to render as diagram
                if self.generate_diagram(literal, &style_lower) {
                    return; // Successfully rendered as diagram
                }
                // Fall through to code block if rendering fails
            }
        }

        // Regular code block rendering - use style_map
        self.output.push_str("<w:p>\n");
        self.output.push_str("<w:pPr>\n");
        let style = literal
            .style_id
            .as_deref()
            .unwrap_or_else(|| self.style_map.code_block());
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(style)));
        self.output.push_str("</w:pPr>\n");

        // If there's a language, add a comment to preserve it
        let comment_id = if let Some(ref lang) = literal.language {
            let id = self.next_comment_id;
            self.next_comment_id += 1;
            self.comments.push(Comment {
                id,
                text: format!("Language: {}", lang),
                author: "utf8dok".to_string(),
            });
            // Add comment range start
            self.output
                .push_str(&format!("<w:commentRangeStart w:id=\"{}\"/>\n", id));
            Some(id)
        } else {
            None
        };

        // Generate the content as a run with preserved whitespace
        self.output.push_str("<w:r>\n");
        self.output.push_str("<w:rPr>\n");
        self.output
            .push_str("<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>\n");
        self.output.push_str("</w:rPr>\n");
        self.output.push_str(&format!(
            "<w:t xml:space=\"preserve\">{}</w:t>\n",
            escape_xml(&literal.content)
        ));
        self.output.push_str("</w:r>\n");

        // Close comment range if we added one
        if let Some(id) = comment_id {
            self.output
                .push_str(&format!("<w:commentRangeEnd w:id=\"{}\"/>\n", id));
            self.output.push_str("<w:r>\n");
            self.output
                .push_str(&format!("<w:commentReference w:id=\"{}\"/>\n", id));
            self.output.push_str("</w:r>\n");
        }

        self.output.push_str("</w:p>\n");
    }

    /// Generate a diagram as an embedded image
    ///
    /// Returns true if successful, false if rendering failed
    fn generate_diagram(&mut self, literal: &utf8dok_ast::LiteralBlock, style: &str) -> bool {
        let engine = match &self.diagram_engine {
            Some(e) => e,
            None => return false,
        };

        // Map style to DiagramType
        let diagram_type = match style {
            "mermaid" => DiagramType::Mermaid,
            "plantuml" => DiagramType::PlantUml,
            "graphviz" | "dot" => DiagramType::GraphViz,
            "d2" => DiagramType::D2,
            "ditaa" => DiagramType::Ditaa,
            "blockdiag" => DiagramType::BlockDiag,
            "seqdiag" => DiagramType::SeqDiag,
            "actdiag" => DiagramType::ActDiag,
            "nwdiag" => DiagramType::NwDiag,
            "c4plantuml" => DiagramType::C4PlantUml,
            "erd" => DiagramType::Erd,
            "nomnoml" => DiagramType::Nomnoml,
            "pikchr" => DiagramType::Pikchr,
            "structurizr" => DiagramType::Structurizr,
            "vega" => DiagramType::Vega,
            "vegalite" => DiagramType::VegaLite,
            "wavedrom" => DiagramType::WaveDrom,
            "svgbob" => DiagramType::Svgbob,
            _ => return false,
        };

        // Render the diagram to PNG using the engine (native or Kroki fallback)
        let png_data = match engine.render_png(&literal.content, diagram_type) {
            Ok(data) => data,
            Err(_) => return false, // Silently fall back to code block
        };

        // Generate unique IDs
        let image_id = self.next_image_id;
        self.next_image_id += 1;
        let drawing_id = self.next_drawing_id;
        self.next_drawing_id += 1;

        // Determine file extension for diagram source
        let source_ext = match style {
            "mermaid" => "mmd",
            "plantuml" | "c4plantuml" => "puml",
            "graphviz" | "dot" => "dot",
            "d2" => "d2",
            _ => "txt",
        };

        // Store media file
        let media_path = format!("word/media/image{}.png", image_id);
        self.media_files.push((media_path.clone(), png_data));

        // Store diagram source
        let source_path = format!("utf8dok/diagrams/fig{}.{}", image_id, source_ext);
        self.diagram_sources
            .push((source_path.clone(), literal.content.clone()));

        // Add relationship
        let rel_id = self.relationships.add(
            format!("media/image{}.png", image_id),
            Relationships::TYPE_IMAGE.to_string(),
        );

        // Compute content hash for manifest
        let mut hasher = Sha256::new();
        hasher.update(literal.content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        // Add to manifest
        self.manifest.add_element(
            format!("fig{}", image_id),
            ElementMeta::new("figure")
                .with_source(source_path)
                .with_hash(hash)
                .with_description(format!("{} diagram", style)),
        );

        // Generate the drawing XML
        self.generate_drawing_xml(drawing_id, &rel_id);

        true
    }

    /// Generate the <w:drawing> XML for an embedded image
    fn generate_drawing_xml(&mut self, drawing_id: usize, rel_id: &str) {
        // Approximate 6x4 inches in EMUs (914400 EMUs per inch)
        let cx = 5715000; // ~6.25 inches
        let cy = 3810000; // ~4.17 inches

        self.output.push_str("<w:p>\n");
        self.output.push_str("  <w:r>\n");
        self.output.push_str("    <w:drawing>\n");
        self.output.push_str(&format!(
            r#"      <wp:inline distT="0" distB="0" distL="0" distR="0">
        <wp:extent cx="{}" cy="{}"/>
        <wp:docPr id="{}" name="Diagram {}"/>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <pic:pic>
              <pic:nvPicPr>
                <pic:cNvPr id="{}" name="Diagram"/>
                <pic:cNvPicPr/>
              </pic:nvPicPr>
              <pic:blipFill>
                <a:blip r:embed="{}"/>
                <a:stretch><a:fillRect/></a:stretch>
              </pic:blipFill>
              <pic:spPr>
                <a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm>
                <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
              </pic:spPr>
            </pic:pic>
          </a:graphicData>
        </a:graphic>
      </wp:inline>
"#,
            cx, cy, drawing_id, drawing_id, drawing_id, rel_id, cx, cy
        ));
        self.output.push_str("    </w:drawing>\n");
        self.output.push_str("  </w:r>\n");
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
                if link.url.starts_with('#') {
                    // Internal link (cross-reference): use w:hyperlink with w:anchor
                    let anchor = &link.url[1..]; // Strip the leading #
                    self.output.push_str(&format!(
                        "<w:hyperlink w:anchor=\"{}\">\n",
                        escape_xml(anchor)
                    ));
                    self.output.push_str("<w:r>\n");
                    self.output.push_str("<w:rPr>\n");
                    self.output.push_str("<w:rStyle w:val=\"Hyperlink\"/>\n");
                    self.output.push_str("</w:rPr>\n");
                    for text_inline in &link.text {
                        if let Inline::Text(text) = text_inline {
                            self.output
                                .push_str(&format!("<w:t>{}</w:t>\n", escape_xml(text)));
                        }
                    }
                    self.output.push_str("</w:r>\n");
                    self.output.push_str("</w:hyperlink>\n");
                } else {
                    // External link: add relationship and use r:id
                    let rel_id = self
                        .relationships
                        .add(link.url.clone(), Relationships::TYPE_HYPERLINK.to_string());
                    self.output
                        .push_str(&format!("<w:hyperlink r:id=\"{}\">\n", escape_xml(&rel_id)));
                    self.output.push_str("<w:r>\n");
                    self.output.push_str("<w:rPr>\n");
                    self.output.push_str("<w:rStyle w:val=\"Hyperlink\"/>\n");
                    self.output.push_str("</w:rPr>\n");
                    for text_inline in &link.text {
                        if let Inline::Text(text) = text_inline {
                            self.output
                                .push_str(&format!("<w:t>{}</w:t>\n", escape_xml(text)));
                        }
                    }
                    self.output.push_str("</w:r>\n");
                    self.output.push_str("</w:hyperlink>\n");
                }
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
                self.output
                    .push_str("<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>\n");
            }
            FormatType::Highlight => {
                self.output.push_str("<w:highlight w:val=\"yellow\"/>\n");
            }
            FormatType::Superscript => {
                self.output
                    .push_str("<w:vertAlign w:val=\"superscript\"/>\n");
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
        zip.start_file("word/_rels/document.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#,
        )
        .unwrap();

        // word/document.xml (placeholder, will be replaced)
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Template</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
        )
        .unwrap();

        zip.finish().unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_write_basic_doc() {
        let template = create_minimal_template();

        // Create a simple document
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
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
                        Inline::Format(
                            FormatType::Bold,
                            Box::new(Inline::Text("test".to_string())),
                        ),
                        Inline::Text(" document.".to_string()),
                    ],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
            ],
        };

        // Generate DOCX without diagram rendering
        let result = DocxWriter::generate_with_options(&doc, &template, false);
        assert!(
            result.is_ok(),
            "Failed to generate DOCX: {:?}",
            result.err()
        );

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
            intent: None,
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

        let result = DocxWriter::generate_with_options(&doc, &template, false);
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
            intent: None,
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

        let result = DocxWriter::generate_with_options(&doc, &template, false);
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
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![
                    Inline::Format(FormatType::Bold, Box::new(Inline::Text("bold".to_string()))),
                    Inline::Text(" ".to_string()),
                    Inline::Format(
                        FormatType::Italic,
                        Box::new(Inline::Text("italic".to_string())),
                    ),
                    Inline::Text(" ".to_string()),
                    Inline::Format(
                        FormatType::Monospace,
                        Box::new(Inline::Text("mono".to_string())),
                    ),
                ],
                style_id: None,
                attributes: HashMap::new(),
            })],
        };

        let result = DocxWriter::generate_with_options(&doc, &template, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        assert!(doc_xml.contains("<w:b/>"));
        assert!(doc_xml.contains("<w:i/>"));
        assert!(doc_xml.contains("Courier New"));
    }

    #[test]
    fn test_external_hyperlink_with_relationship() {
        let template = create_minimal_template();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Link(utf8dok_ast::Link {
                    url: "https://example.com".to_string(),
                    text: vec![Inline::Text("Example".to_string())],
                })],
                style_id: None,
                attributes: HashMap::new(),
            })],
        };

        let result = DocxWriter::generate_with_options(&doc, &template, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Check document.xml has hyperlink with r:id
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();
        assert!(doc_xml.contains("r:id=\"rId"), "Hyperlink should have r:id");
        assert!(doc_xml.contains("Example"), "Link text should be present");

        // Check relationships file has the hyperlink
        let rels_xml = archive
            .get_string("word/_rels/document.xml.rels")
            .unwrap()
            .unwrap();
        assert!(
            rels_xml.contains("https://example.com"),
            "Relationship should have URL"
        );
        assert!(
            rels_xml.contains("hyperlink"),
            "Relationship type should be hyperlink"
        );
    }

    #[test]
    fn test_diagram_styles_recognized() {
        // Test that known diagram styles are in the list
        assert!(DIAGRAM_STYLES.contains(&"mermaid"));
        assert!(DIAGRAM_STYLES.contains(&"plantuml"));
        assert!(DIAGRAM_STYLES.contains(&"graphviz"));
        assert!(DIAGRAM_STYLES.contains(&"d2"));
    }

    #[test]
    fn test_code_block_without_diagram_rendering() {
        let template = create_minimal_template();

        // Create a document with a mermaid block but disable diagram rendering
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Literal(utf8dok_ast::LiteralBlock {
                content: "graph TD; A-->B;".to_string(),
                language: None,
                title: None,
                style_id: Some("mermaid".to_string()),
            })],
        };

        // Generate without diagram rendering - should fall back to code block
        let result = DocxWriter::generate_with_options(&doc, &template, false);
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        // Should be rendered as code block, not image
        assert!(
            doc_xml.contains("graph TD"),
            "Content should be present as code"
        );
        assert!(
            doc_xml.contains("Courier New"),
            "Should have monospace font"
        );
        assert!(
            !doc_xml.contains("<w:drawing>"),
            "Should not have drawing element"
        );
    }

    /// Create a template with corporate-style naming for testing style mapping
    fn create_corporate_template() -> Vec<u8> {
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
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#).unwrap();

        // _rels/.rels
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).unwrap();

        // word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#,
        )
        .unwrap();

        // word/styles.xml with standard Word style names
        zip.start_file("word/styles.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Normal" w:default="1">
    <w:name w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
  </w:style>
  <w:style w:type="table" w:styleId="TableGrid">
    <w:name w:val="Table Grid"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="CodeBlock">
    <w:name w:val="Code Block"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet">
    <w:name w:val="List Bullet"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListNumber">
    <w:name w:val="List Number"/>
  </w:style>
</w:styles>"#,
        )
        .unwrap();

        // word/document.xml (empty template body)
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
  </w:body>
</w:document>"#,
        )
        .unwrap();

        zip.finish().unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_render_with_template() {
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        // Create a document with headings, paragraphs, and code
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Introduction".to_string())],
                    style_id: None,
                    anchor: Some("sec-intro".to_string()),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("This is the first paragraph.".to_string())],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
                Block::Heading(Heading {
                    level: 2,
                    text: vec![Inline::Text("Details".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![
                        Inline::Text("Here is some ".to_string()),
                        Inline::Format(
                            FormatType::Bold,
                            Box::new(Inline::Text("important".to_string())),
                        ),
                        Inline::Text(" information.".to_string()),
                    ],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
                Block::Table(Table {
                    rows: vec![utf8dok_ast::TableRow {
                        cells: vec![
                            utf8dok_ast::TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Name".to_string())],
                                    style_id: None,
                                    attributes: HashMap::new(),
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                            utf8dok_ast::TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Value".to_string())],
                                    style_id: None,
                                    attributes: HashMap::new(),
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                        ],
                        is_header: true,
                    }],
                    style_id: None, // Should use mapped TableGrid
                    caption: None,
                    columns: vec![],
                }),
            ],
        };

        // Generate using template-based method (without diagrams for test speed)
        let result = DocxWriter::generate_from_template_with_options(&doc, template, false, None);
        assert!(
            result.is_ok(),
            "Failed to generate DOCX: {:?}",
            result.err()
        );

        let output = result.unwrap();

        // Verify it's a valid ZIP
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Verify word/document.xml exists and contains correct styles
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        // Check heading styles are correctly applied
        assert!(
            doc_xml.contains("<w:pStyle w:val=\"Heading1\"/>"),
            "Should have Heading1 style"
        );
        assert!(
            doc_xml.contains("<w:pStyle w:val=\"Heading2\"/>"),
            "Should have Heading2 style"
        );

        // Check paragraph style
        assert!(
            doc_xml.contains("<w:pStyle w:val=\"Normal\"/>"),
            "Should have Normal style for paragraphs"
        );

        // Check table style
        assert!(
            doc_xml.contains("<w:tblStyle w:val=\"TableGrid\"/>"),
            "Should have TableGrid table style"
        );

        // Check content is present
        assert!(doc_xml.contains("Introduction"), "Heading text missing");
        assert!(
            doc_xml.contains("first paragraph"),
            "Paragraph text missing"
        );
        assert!(doc_xml.contains("important"), "Bold text missing");
        assert!(doc_xml.contains("<w:b/>"), "Bold formatting missing");

        // Verify styles.xml was preserved from template
        let styles_xml = archive.get_string("word/styles.xml").unwrap();
        assert!(styles_xml.is_some(), "styles.xml should be preserved");
    }

    #[test]
    fn test_custom_style_map() {
        use crate::styles::{ElementType, StyleMap};
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        // Create custom style map with different mappings
        let mut custom_map = StyleMap::new();
        custom_map.set(ElementType::Heading(1), "Heading2"); // Use Heading2 for level 1
        custom_map.set(ElementType::Paragraph, "Normal");

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Heading(Heading {
                level: 1,
                text: vec![Inline::Text("Title".to_string())],
                style_id: None,
                anchor: None,
            })],
        };

        let result = DocxWriter::generate_from_template_with_options(
            &doc,
            template,
            false,
            Some(custom_map),
        );
        assert!(result.is_ok());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        let doc_xml = archive.get_string("word/document.xml").unwrap().unwrap();

        // Should use Heading2 because of custom mapping
        assert!(
            doc_xml.contains("<w:pStyle w:val=\"Heading2\"/>"),
            "Should use custom-mapped Heading2 style"
        );
    }

    #[test]
    fn test_self_contained_docx() {
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        let source_content = r#"= My Document

This is the *original* AsciiDoc source.

== Section One

Some content here.
"#;

        let config_content = r#"# utf8dok configuration
[template]
path = "template.dotx"

[styles]
heading1 = "Heading1"
paragraph = "Normal"
"#;

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("My Document".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Some content.".to_string())],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
            ],
        };

        // Create writer and set embedded content
        let mut writer = DocxWriter::new();
        writer.set_source(source_content);
        writer.set_config(config_content);

        // Generate self-contained DOCX
        let result = writer.generate_with_template(&doc, template);
        assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

        let output = result.unwrap();

        // Verify embedded content exists
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Check source.adoc was embedded
        let embedded_source = archive.get_string("utf8dok/source.adoc").unwrap();
        assert!(
            embedded_source.is_some(),
            "utf8dok/source.adoc should exist"
        );
        assert_eq!(
            embedded_source.unwrap(),
            source_content,
            "Embedded source should match original"
        );

        // Check utf8dok.toml was embedded
        let embedded_config = archive.get_string("utf8dok/utf8dok.toml").unwrap();
        assert!(
            embedded_config.is_some(),
            "utf8dok/utf8dok.toml should exist"
        );
        assert_eq!(
            embedded_config.unwrap(),
            config_content,
            "Embedded config should match original"
        );

        // Check manifest exists and contains entries
        let manifest_json = archive.get_string("utf8dok/manifest.json").unwrap();
        assert!(
            manifest_json.is_some(),
            "utf8dok/manifest.json should exist"
        );
        let manifest = manifest_json.unwrap();
        assert!(
            manifest.contains("source"),
            "Manifest should have source entry"
        );
        assert!(
            manifest.contains("config"),
            "Manifest should have config entry"
        );
    }
}
