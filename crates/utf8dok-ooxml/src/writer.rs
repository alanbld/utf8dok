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
use crate::style_map::{CoverConfig, CoverMetadata, StyleContract, TextAlign};
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
    /// Style contract for round-trip fidelity (ADR-007)
    style_contract: Option<StyleContract>,
    /// Original AsciiDoc source (for self-contained DOCX)
    source_text: Option<String>,
    /// Configuration TOML (for self-contained DOCX)
    config_text: Option<String>,
    /// Comments to be added to comments.xml
    comments: Vec<Comment>,
    /// Next comment ID
    next_comment_id: usize,
    /// Next bookmark ID for unique bookmark IDs
    next_bookmark_id: usize,
    /// Cover image path and bytes (for title page)
    cover_image: Option<(String, Vec<u8>)>,
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
            style_contract: None,
            source_text: None,
            config_text: None,
            comments: Vec::new(),
            next_comment_id: 1,
            next_bookmark_id: 0,
            cover_image: None,
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
            style_contract: None,
            source_text: None,
            config_text: None,
            comments: Vec::new(),
            next_comment_id: 1,
            next_bookmark_id: 0,
            cover_image: None,
        }
    }

    /// Set a cover image for the title page
    ///
    /// The cover image will be rendered as a full-page image at the beginning
    /// of the document, followed by a page break.
    pub fn set_cover_image(&mut self, filename: impl Into<String>, data: Vec<u8>) {
        self.cover_image = Some((filename.into(), data));
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

    /// Set the style contract for round-trip fidelity (ADR-007)
    ///
    /// When set, the writer will use the contract to restore original
    /// bookmark names and anchor mappings from extraction.
    pub fn set_style_contract(&mut self, contract: StyleContract) {
        self.style_contract = Some(contract);
    }

    /// Resolve an anchor name to the original Word bookmark name
    ///
    /// If a StyleContract is set and contains a reverse mapping for this
    /// semantic anchor, returns the original Word bookmark name.
    /// Otherwise, returns the anchor name as-is.
    fn resolve_anchor_name(&self, semantic_anchor: &str) -> String {
        if let Some(ref contract) = self.style_contract {
            // Look for a mapping where semantic_id matches this anchor
            if let Some(word_bookmark) = contract.get_word_bookmark(semantic_anchor) {
                return word_bookmark.to_string();
            }
        }
        // Fall back to the semantic anchor name
        semantic_anchor.to_string()
    }

    /// Resolve heading level to Word style ID
    ///
    /// Uses StyleContract if available for round-trip fidelity,
    /// otherwise falls back to StyleMap defaults.
    fn resolve_heading_style(&self, level: u8) -> &str {
        if let Some(ref contract) = self.style_contract {
            if let Some(style) = contract.get_word_heading_style(level) {
                return style;
            }
        }
        // Fall back to style_map
        self.style_map.heading(level)
    }

    /// Resolve semantic role to Word paragraph style ID
    ///
    /// Uses StyleContract if available, otherwise falls back to StyleMap.
    fn resolve_paragraph_style(&self, role: &str) -> &str {
        if let Some(ref contract) = self.style_contract {
            if let Some(style) = contract.get_word_style_for_role(role) {
                return style;
            }
        }
        // Fall back to style_map for body text
        self.style_map.paragraph()
    }

    /// Get the next unique bookmark ID
    fn next_bookmark_id(&mut self) -> usize {
        let id = self.next_bookmark_id;
        self.next_bookmark_id += 1;
        id
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
        let author = doc
            .metadata
            .authors
            .first()
            .map(|s| s.as_str())
            .or_else(|| doc.metadata.attributes.get("author").map(|s| s.as_str()));

        // Check for revdate attribute
        let revdate = doc
            .metadata
            .revision
            .as_deref()
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
                        &format!(
                            "<dc:title>{}</dc:title></cp:coreProperties>",
                            escape_xml(new_title)
                        ),
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
                                format!(
                                    "<dc:creator>{}</dc:creator>{}",
                                    escape_xml(new_author),
                                    rest
                                )
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
                        &format!(
                            "<dc:creator>{}</dc:creator></cp:coreProperties>",
                            escape_xml(new_author)
                        ),
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
                            updated = format!(
                                "{}{}{}",
                                &updated[..start],
                                replacement,
                                &updated[end_pos..]
                            );
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

        // Generate cover page if set (with document metadata)
        self.generate_cover_page(doc);

        // Generate blocks
        for block in &doc.blocks {
            self.generate_block(block);
        }

        // Close body and document
        self.output.push_str("</w:body>\n");
        self.output.push_str("</w:document>");

        self.output.clone()
    }

    /// Generate a cover page with image as background and text overlaid on top
    /// Uses wp:anchor with behindDoc="1" for corporate-style layout
    /// Configuration is read from StyleContract.cover (ADR-009)
    fn generate_cover_page(&mut self, doc: &Document) {
        // Check if cover image is set
        let cover_data = match self.cover_image.take() {
            Some(data) => data,
            None => return,
        };

        let (filename, image_bytes) = cover_data;

        // Get cover configuration from StyleContract or use defaults
        let cover_config = self
            .style_contract
            .as_ref()
            .and_then(|sc| sc.cover.clone())
            .unwrap_or_else(CoverConfig::for_dark_background);

        // Add cover image to media files
        let media_filename = format!("cover_{}", filename);
        let archive_path = format!("word/media/{}", media_filename);
        let rel_path = format!("media/{}", media_filename);
        self.media_files.push((archive_path, image_bytes));

        // Generate relationship for the cover image
        let rel_id = self.relationships.add_image(&rel_path);

        // Get unique drawing ID
        let image_id = self.next_drawing_id;
        self.next_drawing_id += 1;

        // Full page dimensions (A4: 210mm x 297mm, minus margins ~25mm each side)
        // A4 in EMU: 210mm = 7560000 EMU, 297mm = 10692000 EMU
        let page_width_emu: i64 = 5943600; // ~16.5cm = reasonable page width
        let page_height_emu: i64 = 8419465; // ~23.4cm = fits most of page

        // Build cover metadata from document
        let metadata = self.extract_cover_metadata(doc);

        // === COVER IMAGE (anchored behind document) ===
        self.output.push_str("<w:p>\n");
        self.output.push_str("<w:r>\n");
        self.output.push_str("<w:drawing>\n");

        // wp:anchor places image at fixed position, behindDoc="1" puts it behind text
        self.output.push_str(&format!(
            r#"<wp:anchor xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                distT="0" distB="0" distL="0" distR="0"
                simplePos="0" relativeHeight="0" behindDoc="1"
                locked="0" layoutInCell="1" allowOverlap="1">
<wp:simplePos x="0" y="0"/>
<wp:positionH relativeFrom="page"><wp:align>center</wp:align></wp:positionH>
<wp:positionV relativeFrom="page"><wp:posOffset>457200</wp:posOffset></wp:positionV>
<wp:extent cx="{}" cy="{}"/>
<wp:effectExtent l="0" t="0" r="0" b="0"/>
<wp:wrapNone/>
<wp:docPr id="{}" name="Cover Image" descr="Document cover"/>
<wp:cNvGraphicFramePr><a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/></wp:cNvGraphicFramePr>
<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
<pic:nvPicPr><pic:cNvPr id="{}" name="Cover"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="{}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>
</pic:pic></a:graphicData></a:graphic></wp:anchor>"#,
            page_width_emu, page_height_emu,
            image_id, image_id,
            rel_id,
            page_width_emu, page_height_emu
        ));
        self.output.push('\n');
        self.output.push_str("</w:drawing>\n</w:r>\n</w:p>\n");

        // === TITLE ===
        if !metadata.title.is_empty() {
            self.generate_cover_element(&metadata.title, &cover_config.title, page_height_emu);
        }

        // === SUBTITLE ===
        if !metadata.subtitle.is_empty() {
            self.generate_cover_element(
                &metadata.subtitle,
                &cover_config.subtitle,
                page_height_emu,
            );
        }

        // === AUTHORS ===
        let authors_text = if let Some(ref template) = cover_config.authors.content {
            CoverConfig::expand_template(template, &metadata, &cover_config.revision.delimiter)
        } else {
            metadata.author.clone()
        };
        if !authors_text.is_empty() {
            self.generate_cover_element(&authors_text, &cover_config.authors, page_height_emu);
        }

        // === REVISION ===
        let revision_text = CoverConfig::expand_template(
            &cover_config.revision.content,
            &metadata,
            &cover_config.revision.delimiter,
        );
        // Only show if we have actual values (not just template placeholders)
        let revision_has_content = !metadata.revnumber.is_empty() || !metadata.revdate.is_empty();
        if revision_has_content && !revision_text.trim().is_empty() {
            self.generate_cover_revision_element(
                &revision_text,
                &cover_config.revision,
                page_height_emu,
            );
        }

        // === PAGE BREAK ===
        self.output.push_str("<w:p>\n<w:r>\n");
        self.output.push_str(r#"<w:br w:type="page"/>"#);
        self.output.push('\n');
        self.output.push_str("</w:r>\n</w:p>\n");
    }

    /// Extract cover metadata from document
    fn extract_cover_metadata(&self, doc: &Document) -> CoverMetadata {
        // Title
        let title = doc.metadata.title.clone().unwrap_or_default();

        // Authors: try direct field first, then attribute
        let author = if !doc.metadata.authors.is_empty() {
            doc.metadata.authors.join(", ")
        } else {
            doc.metadata
                .attributes
                .get("author")
                .cloned()
                .unwrap_or_default()
        };

        // Email
        let email = doc
            .metadata
            .attributes
            .get("email")
            .cloned()
            .unwrap_or_default();

        // Revision: try direct field first, then attribute
        let revnumber = doc
            .metadata
            .revision
            .clone()
            .or_else(|| doc.metadata.attributes.get("revnumber").cloned())
            .unwrap_or_default();

        let revdate = doc
            .metadata
            .attributes
            .get("revdate")
            .cloned()
            .unwrap_or_default();

        let revremark = doc
            .metadata
            .attributes
            .get("revremark")
            .cloned()
            .unwrap_or_default();

        // Subtitle: try multiple sources
        let subtitle = doc
            .metadata
            .attributes
            .get("description")
            .cloned()
            .or_else(|| doc.metadata.attributes.get("subtitle").cloned())
            .or_else(|| doc.metadata.attributes.get("revremark").cloned())
            .unwrap_or_default();

        CoverMetadata {
            title,
            subtitle,
            author,
            email,
            revnumber,
            revdate,
            revremark,
        }
    }

    /// Generate a cover text element using configuration
    fn generate_cover_element(
        &mut self,
        text: &str,
        config: &crate::style_map::CoverElementConfig,
        page_height_emu: i64,
    ) {
        // Calculate position in twips (1 twip = 1/20 pt = 1/1440 in = 635 EMU)
        let position_emu = CoverConfig::parse_position_to_emu(&config.top, page_height_emu);
        let position_twips = position_emu / 635;

        self.output.push_str("<w:p>\n<w:pPr>\n");

        // Alignment
        let align_val = match config.align {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        };
        self.output
            .push_str(&format!("<w:jc w:val=\"{}\"/>\n", align_val));

        // Frame positioning for absolute placement
        self.output.push_str(&format!(
            "<w:framePr w:vAnchor=\"page\" w:y=\"{}\"/>\n",
            position_twips
        ));

        self.output.push_str("</w:pPr>\n");
        self.output.push_str("<w:r>\n<w:rPr>\n");

        // Font size
        self.output
            .push_str(&format!("<w:sz w:val=\"{}\"/>\n", config.font_size));
        self.output
            .push_str(&format!("<w:szCs w:val=\"{}\"/>\n", config.font_size));

        // Color
        self.output
            .push_str(&format!("<w:color w:val=\"{}\"/>\n", config.color));

        // Bold
        if config.bold {
            self.output.push_str("<w:b/>\n");
        }

        // Italic
        if config.italic {
            self.output.push_str("<w:i/>\n");
        }

        // Font family
        if let Some(ref font) = config.font_family {
            self.output.push_str(&format!(
                "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>\n",
                escape_xml(font),
                escape_xml(font)
            ));
        }

        self.output.push_str("</w:rPr>\n");
        self.output.push_str("<w:t>");
        self.output.push_str(&escape_xml(text));
        self.output.push_str("</w:t>\n</w:r>\n</w:p>\n");
    }

    /// Generate a cover revision element using configuration
    fn generate_cover_revision_element(
        &mut self,
        text: &str,
        config: &crate::style_map::CoverRevisionConfig,
        page_height_emu: i64,
    ) {
        let position_emu = CoverConfig::parse_position_to_emu(&config.top, page_height_emu);
        let position_twips = position_emu / 635;

        self.output.push_str("<w:p>\n<w:pPr>\n");

        let align_val = match config.align {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        };
        self.output
            .push_str(&format!("<w:jc w:val=\"{}\"/>\n", align_val));
        self.output.push_str(&format!(
            "<w:framePr w:vAnchor=\"page\" w:y=\"{}\"/>\n",
            position_twips
        ));

        self.output.push_str("</w:pPr>\n");
        self.output.push_str("<w:r>\n<w:rPr>\n");

        self.output
            .push_str(&format!("<w:sz w:val=\"{}\"/>\n", config.font_size));
        self.output
            .push_str(&format!("<w:szCs w:val=\"{}\"/>\n", config.font_size));
        self.output
            .push_str(&format!("<w:color w:val=\"{}\"/>\n", config.color));

        if config.bold {
            self.output.push_str("<w:b/>\n");
        }
        if config.italic {
            self.output.push_str("<w:i/>\n");
        }

        if let Some(ref font) = config.font_family {
            self.output.push_str(&format!(
                "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>\n",
                escape_xml(font),
                escape_xml(font)
            ));
        }

        self.output.push_str("</w:rPr>\n");
        self.output.push_str("<w:t>");
        self.output.push_str(&escape_xml(text));
        self.output.push_str("</w:t>\n</w:r>\n</w:p>\n");
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
            Block::Open(open) => {
                // Render open block contents (e.g., [slides], [example])
                for inner in &open.blocks {
                    self.generate_block(inner);
                }
            }
            Block::Sidebar(sidebar) => {
                // Render sidebar as a styled paragraph block
                for inner in &sidebar.blocks {
                    self.generate_block(inner);
                }
            }
            Block::Quote(quote) => {
                // Render quote block contents
                for inner in &quote.blocks {
                    self.generate_block(inner);
                }
            }
            Block::ThematicBreak => {
                // Render as a horizontal rule / page break
                self.output
                    .push_str("<w:p><w:pPr><w:pBdr><w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"1\" w:color=\"auto\"/></w:pBdr></w:pPr></w:p>\n");
            }
        }
    }

    /// Generate XML for a paragraph
    fn generate_paragraph(&mut self, para: &Paragraph) {
        self.output.push_str("<w:p>\n");

        // Paragraph style resolution priority:
        // 1. Explicit style_id from AST
        // 2. StyleContract body role mapping (for round-trip fidelity)
        // 3. StyleMap default (English style IDs)
        let style: String = para
            .style_id
            .clone()
            .unwrap_or_else(|| self.resolve_paragraph_style("body").to_string());
        self.output.push_str("<w:pPr>\n");
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(&style)));
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

        // Heading style resolution priority:
        // 1. Explicit style_id from AST
        // 2. StyleContract heading level mapping (for round-trip fidelity)
        // 3. StyleMap default (English style IDs)
        let style: String = heading
            .style_id
            .clone()
            .unwrap_or_else(|| self.resolve_heading_style(heading.level).to_string());

        self.output.push_str("<w:pPr>\n");
        self.output
            .push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", escape_xml(&style)));
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
                    // Use StyleContract to restore original anchor name if available
                    let semantic_anchor = &link.url[1..]; // Strip the leading #
                    let anchor = self.resolve_anchor_name(semantic_anchor);
                    self.output.push_str(&format!(
                        "<w:hyperlink w:anchor=\"{}\">\n",
                        escape_xml(&anchor)
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
            Inline::Image(image) => {
                self.generate_image(image);
            }
            Inline::Break => {
                self.output.push_str("<w:r>\n");
                self.output.push_str("<w:br/>\n");
                self.output.push_str("</w:r>\n");
            }
            Inline::Anchor(name) => {
                // Generate bookmark start and end at this position
                // Use StyleContract to restore original bookmark name if available
                let bookmark_name = self.resolve_anchor_name(name);
                let bookmark_id = self.next_bookmark_id();
                self.output.push_str(&format!(
                    "<w:bookmarkStart w:id=\"{}\" w:name=\"{}\"/>\n",
                    bookmark_id,
                    escape_xml(&bookmark_name)
                ));
                self.output
                    .push_str(&format!("<w:bookmarkEnd w:id=\"{}\"/>\n", bookmark_id));
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

    /// Generate XML for an inline image
    fn generate_image(&mut self, image: &utf8dok_ast::Image) {
        use crate::image::pixels_to_emu;

        // Generate unique ID for this image
        let image_id = self.next_drawing_id;
        self.next_drawing_id += 1;

        // Get the image path (relative to project root or absolute)
        let src = &image.src;

        // Generate relationship ID for the image
        // The target should be relative from word/ to word/media/
        let media_target = if src.starts_with("media/") {
            src.clone()
        } else {
            format!("media/{}", src.rsplit('/').next().unwrap_or(src))
        };

        let rel_id = self.relationships.add_image(&media_target);

        // Get or estimate image dimensions
        // Default to 2 inches (192 pixels at 96 DPI) if not specified
        let default_width_px = 200i64;
        let default_height_px = 150i64;
        let width_emu = pixels_to_emu(default_width_px);
        let height_emu = pixels_to_emu(default_height_px);

        // Alt text (description)
        let alt_text = image.alt.clone().unwrap_or_default();
        let name = format!("Image {}", image_id);

        // Generate the drawing XML
        self.output.push_str("<w:r>\n");
        self.output.push_str("<w:drawing>\n");

        // Inline image (flows with text)
        self.output.push_str(
            r#"<wp:inline distT="0" distB="0" distL="0" distR="0" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">"#,
        );
        self.output.push('\n');

        // Extent (dimensions in EMUs)
        self.output.push_str(&format!(
            r#"<wp:extent cx="{}" cy="{}"/>"#,
            width_emu, height_emu
        ));
        self.output.push('\n');

        // Effect extent (padding)
        self.output
            .push_str(r#"<wp:effectExtent l="0" t="0" r="0" b="0"/>"#);
        self.output.push('\n');

        // Document properties (id, name, description/alt text)
        self.output.push_str(&format!(
            r#"<wp:docPr id="{}" name="{}" descr="{}"/>"#,
            image_id,
            escape_xml(&name),
            escape_xml(&alt_text)
        ));
        self.output.push('\n');

        // Non-visual properties
        self.output.push_str(r#"<wp:cNvGraphicFramePr>"#);
        self.output.push_str(r#"<a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/>"#);
        self.output.push_str(r#"</wp:cNvGraphicFramePr>"#);
        self.output.push('\n');

        // Graphic container
        self.output.push_str(
            r#"<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
        );
        self.output.push('\n');
        self.output.push_str(
            r#"<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">"#,
        );
        self.output.push('\n');

        // Picture element
        self.output.push_str(
            r#"<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#,
        );
        self.output.push('\n');

        // Non-visual picture properties
        self.output.push_str(r#"<pic:nvPicPr>"#);
        self.output.push_str(&format!(
            r#"<pic:cNvPr id="{}" name="{}"/>"#,
            image_id,
            escape_xml(&name)
        ));
        self.output.push_str(r#"<pic:cNvPicPr/>"#);
        self.output.push_str(r#"</pic:nvPicPr>"#);
        self.output.push('\n');

        // Blip fill (reference to actual image)
        self.output.push_str(r#"<pic:blipFill>"#);
        self.output.push_str(&format!(
            r#"<a:blip r:embed="{}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>"#,
            rel_id
        ));
        self.output
            .push_str(r#"<a:stretch><a:fillRect/></a:stretch>"#);
        self.output.push_str(r#"</pic:blipFill>"#);
        self.output.push('\n');

        // Shape properties (dimensions)
        self.output.push_str(r#"<pic:spPr>"#);
        self.output.push_str(&format!(
            r#"<a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm>"#,
            width_emu, height_emu
        ));
        self.output
            .push_str(r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>"#);
        self.output.push_str(r#"</pic:spPr>"#);
        self.output.push('\n');

        // Close all elements
        self.output.push_str(r#"</pic:pic>"#);
        self.output.push('\n');
        self.output.push_str(r#"</a:graphicData>"#);
        self.output.push('\n');
        self.output.push_str(r#"</a:graphic>"#);
        self.output.push('\n');
        self.output.push_str(r#"</wp:inline>"#);
        self.output.push('\n');

        self.output.push_str("</w:drawing>\n");
        self.output.push_str("</w:r>\n");

        // Note: The image files need to be copied separately by the caller
        // using the source path from image.src
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
        Inline::Anchor(_) => String::new(), // Anchors have no text content
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
    use crate::test_utils::create_minimal_template;
    use std::collections::HashMap;
    use std::io::{Cursor, Write};

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

    // ==================== Sprint 8: DocxWriter Public API Tests ====================

    #[test]
    fn test_docx_writer_default() {
        let writer = DocxWriter::default();
        assert!(writer.source_text.is_none());
        assert!(writer.config_text.is_none());
        assert!(writer.cover_image.is_none());
        assert!(writer.style_contract.is_none());
    }

    #[test]
    fn test_set_cover_image() {
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        // Create a minimal PNG (1x1 pixel)
        let png_data: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixel
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
            0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
            0x44, 0xAE, 0x42, 0x60, 0x82, // IEND chunk
        ];

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("After cover".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            })],
        };

        let mut writer = DocxWriter::new();
        writer.set_cover_image("cover.png", png_data.clone());

        let result = writer.generate_with_template(&doc, template);
        assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Verify the media folder contains an image
        let has_media = archive.file_list().any(|f| f.starts_with("word/media/"));
        assert!(has_media, "Cover image should be in word/media/");
    }

    #[test]
    fn test_set_style_contract_anchor_resolution() {
        use crate::style_map::{AnchorMapping, AnchorType, StyleContract};

        let mut contract = StyleContract::default();
        // HashMap key is the Word bookmark name
        contract.anchors.insert(
            "_Toc123456".to_string(),
            AnchorMapping {
                semantic_id: "introduction".to_string(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".to_string()),
                original_bookmark: Some("_Toc123456".to_string()),
            },
        );
        contract.anchors.insert(
            "_Toc789012".to_string(),
            AnchorMapping {
                semantic_id: "conclusion".to_string(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Conclusion".to_string()),
                original_bookmark: Some("_Toc789012".to_string()),
            },
        );

        let mut writer = DocxWriter::new();
        writer.set_style_contract(contract);

        // Test that anchor resolution works
        let resolved = writer.resolve_anchor_name("introduction");
        assert_eq!(resolved, "_Toc123456");

        let resolved2 = writer.resolve_anchor_name("conclusion");
        assert_eq!(resolved2, "_Toc789012");

        // Unknown anchor returns as-is
        let unknown = writer.resolve_anchor_name("unknown-section");
        assert_eq!(unknown, "unknown-section");
    }

    #[test]
    fn test_set_source_and_config_separately() {
        let mut writer = DocxWriter::new();

        writer.set_source("= My Doc\n\nContent here.");
        assert!(writer.source_text.is_some());
        assert!(writer.config_text.is_none());

        writer.set_config("[template]\npath = \"t.dotx\"");
        assert!(writer.source_text.is_some());
        assert!(writer.config_text.is_some());

        assert_eq!(
            writer.source_text.unwrap(),
            "= My Doc\n\nContent here."
        );
        assert_eq!(
            writer.config_text.unwrap(),
            "[template]\npath = \"t.dotx\""
        );
    }

    #[test]
    fn test_set_embedded_content_together() {
        let mut writer = DocxWriter::new();

        writer.set_embedded_content(
            "= Document Title\n\nParagraph.",
            "[styles]\nheading1 = \"H1\""
        );

        assert_eq!(
            writer.source_text.as_deref(),
            Some("= Document Title\n\nParagraph.")
        );
        assert_eq!(
            writer.config_text.as_deref(),
            Some("[styles]\nheading1 = \"H1\"")
        );
    }

    #[test]
    fn test_with_style_map_factory() {
        use crate::styles::StyleMap;

        let custom_map = StyleMap::default();
        let writer = DocxWriter::with_style_map(custom_map);

        // Verify the writer was created with the custom map
        assert!(writer.source_text.is_none());
        assert!(writer.style_contract.is_none());
        assert!(writer.cover_image.is_none());
    }

    #[test]
    fn test_diagram_source_embedding() {
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        // Create a document with a code block that would be treated as a diagram
        // Note: Without diagram engine, it won't render but structure should be tested
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Document with code.".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            })],
        };

        let mut writer = DocxWriter::new();
        writer.set_source("= Test\n\n[source,mermaid]\n----\ngraph TD\n----");

        let result = writer.generate_with_template(&doc, template);
        assert!(result.is_ok(), "Failed to generate: {:?}", result.err());
    }

    #[test]
    fn test_internal_hyperlink_generation() {
        use crate::Template;

        let template_bytes = create_corporate_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        // Create a document with an internal link
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Target Section".to_string())],
                    style_id: None,
                    anchor: Some("target-section".to_string()),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Link(utf8dok_ast::Link {
                        url: "#target-section".to_string(),
                        text: vec![Inline::Text("Link to target".to_string())],
                    })],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
            ],
        };

        let writer = DocxWriter::new();
        let result = writer.generate_with_template(&doc, template);
        assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

        let output = result.unwrap();
        let cursor = Cursor::new(&output);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Verify document.xml exists
        let doc_xml = archive.get_string("word/document.xml").unwrap();
        assert!(doc_xml.is_some(), "document.xml should exist");

        let content = doc_xml.unwrap();
        // Should have a bookmark
        assert!(
            content.contains("bookmarkStart") || content.contains("w:anchor"),
            "Should have bookmark or anchor"
        );
    }

    // ==================== Sprint 12: Style Contract Resolution Tests ====================

    #[test]
    fn test_resolve_heading_style_without_contract() {
        let writer = DocxWriter::new();

        // Without contract, should fall back to style_map defaults
        assert_eq!(writer.resolve_heading_style(1), "Heading1");
        assert_eq!(writer.resolve_heading_style(2), "Heading2");
        assert_eq!(writer.resolve_heading_style(9), "Heading9");
    }

    #[test]
    fn test_resolve_heading_style_with_contract() {
        use crate::style_map::{ParagraphStyleMapping, StyleContract};

        let mut contract = StyleContract::default();
        // Headings are stored in paragraph_styles with heading_level set
        contract.paragraph_styles.insert(
            "CustomH1".to_string(),
            ParagraphStyleMapping {
                role: "heading".to_string(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.paragraph_styles.insert(
            "CustomH2".to_string(),
            ParagraphStyleMapping {
                role: "heading".to_string(),
                heading_level: Some(2),
                ..Default::default()
            },
        );

        let mut writer = DocxWriter::new();
        writer.set_style_contract(contract);

        // Should use contract mappings
        assert_eq!(writer.resolve_heading_style(1), "CustomH1");
        assert_eq!(writer.resolve_heading_style(2), "CustomH2");

        // Level 3 not in contract - should fall back to style_map
        assert_eq!(writer.resolve_heading_style(3), "Heading3");
    }

    #[test]
    fn test_resolve_paragraph_style_without_contract() {
        let writer = DocxWriter::new();

        // Without contract, should fall back to style_map.paragraph()
        assert_eq!(writer.resolve_paragraph_style("body"), "Normal");
        assert_eq!(writer.resolve_paragraph_style("intro"), "Normal");
    }

    #[test]
    fn test_resolve_paragraph_style_with_contract() {
        use crate::style_map::{ParagraphStyleMapping, StyleContract};

        let mut contract = StyleContract::default();
        // Key is Word style ID, value contains the semantic role
        contract.paragraph_styles.insert(
            "AbstractStyle".to_string(),
            ParagraphStyleMapping {
                role: "abstract".to_string(),
                ..Default::default()
            },
        );
        contract.paragraph_styles.insert(
            "NoteStyle".to_string(),
            ParagraphStyleMapping {
                role: "note".to_string(),
                ..Default::default()
            },
        );

        let mut writer = DocxWriter::new();
        writer.set_style_contract(contract);

        // Should use contract mappings (looks up by role, returns Word style ID)
        assert_eq!(writer.resolve_paragraph_style("abstract"), "AbstractStyle");
        assert_eq!(writer.resolve_paragraph_style("note"), "NoteStyle");

        // Unknown role should fall back
        assert_eq!(writer.resolve_paragraph_style("unknown"), "Normal");
    }

    #[test]
    fn test_next_bookmark_id_increments() {
        let mut writer = DocxWriter::new();

        assert_eq!(writer.next_bookmark_id(), 0);
        assert_eq!(writer.next_bookmark_id(), 1);
        assert_eq!(writer.next_bookmark_id(), 2);
    }

    #[test]
    fn test_extract_cover_metadata_with_title_only() {
        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("My Document Title".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let cover_meta = writer.extract_cover_metadata(&doc);

        assert_eq!(cover_meta.title, "My Document Title");
        assert!(cover_meta.author.is_empty());
        assert!(cover_meta.revnumber.is_empty());
    }

    #[test]
    fn test_extract_cover_metadata_with_authors_vec() {
        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("Title".to_string());
        meta.authors = vec!["John Doe".to_string(), "Jane Smith".to_string()];

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let cover_meta = writer.extract_cover_metadata(&doc);

        assert_eq!(cover_meta.author, "John Doe, Jane Smith");
    }

    #[test]
    fn test_extract_cover_metadata_with_author_attribute() {
        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.attributes
            .insert("author".to_string(), "Attribute Author".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let cover_meta = writer.extract_cover_metadata(&doc);

        // When authors vec is empty, falls back to attribute
        assert_eq!(cover_meta.author, "Attribute Author");
    }

    #[test]
    fn test_extract_cover_metadata_with_revision() {
        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.revision = Some("1.0".to_string());
        meta.attributes
            .insert("revdate".to_string(), "2025-01-01".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let cover_meta = writer.extract_cover_metadata(&doc);

        assert_eq!(cover_meta.revnumber, "1.0");
        assert_eq!(cover_meta.revdate, "2025-01-01");
    }

    #[test]
    fn test_extract_cover_metadata_empty() {
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let cover_meta = writer.extract_cover_metadata(&doc);

        assert!(cover_meta.title.is_empty());
        assert!(cover_meta.author.is_empty());
        assert!(cover_meta.revnumber.is_empty());
        assert!(cover_meta.revdate.is_empty());
    }

    #[test]
    fn test_update_core_properties_with_no_metadata() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let result = writer.update_core_properties(&mut archive, &doc);
        assert!(result.is_ok());

        // Should not create core.xml when no metadata
        assert!(archive.get("docProps/core.xml").is_none());
    }

    #[test]
    fn test_update_core_properties_creates_new() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("New Title".to_string());
        meta.authors = vec!["Author Name".to_string()];

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let result = writer.update_core_properties(&mut archive, &doc);
        assert!(result.is_ok());

        // Should create core.xml
        let core_xml = archive.get_string("docProps/core.xml").unwrap();
        assert!(core_xml.is_some());

        let content = core_xml.unwrap();
        assert!(content.contains("<dc:title>New Title</dc:title>"));
        assert!(content.contains("<dc:creator>Author Name</dc:creator>"));
    }

    #[test]
    fn test_update_core_properties_updates_existing() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Pre-populate with existing core.xml
        let existing = r#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
    xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>Old Title</dc:title>
<dc:creator>Old Author</dc:creator>
</cp:coreProperties>"#;
        archive.set_string("docProps/core.xml", existing.to_string());

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("Updated Title".to_string());
        meta.authors = vec!["Updated Author".to_string()];

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let result = writer.update_core_properties(&mut archive, &doc);
        assert!(result.is_ok());

        let core_xml = archive.get_string("docProps/core.xml").unwrap().unwrap();
        assert!(core_xml.contains("<dc:title>Updated Title</dc:title>"));
        assert!(core_xml.contains("<dc:creator>Updated Author</dc:creator>"));
        assert!(!core_xml.contains("Old Title"));
        assert!(!core_xml.contains("Old Author"));
    }

    #[test]
    fn test_update_core_properties_with_revdate() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.attributes
            .insert("revdate".to_string(), "2025-06-15".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let result = writer.update_core_properties(&mut archive, &doc);
        assert!(result.is_ok());

        let core_xml = archive.get_string("docProps/core.xml").unwrap().unwrap();
        // Date should have ISO format with time
        assert!(core_xml.contains("2025-06-15T00:00:00Z"));
    }

    #[test]
    fn test_update_core_properties_insert_into_existing_without_title() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Existing core.xml without title element
        let existing = r#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
    xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:creator>Existing Author</dc:creator>
</cp:coreProperties>"#;
        archive.set_string("docProps/core.xml", existing.to_string());

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("Inserted Title".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![],
        };

        let writer = DocxWriter::new();
        let result = writer.update_core_properties(&mut archive, &doc);
        assert!(result.is_ok());

        let core_xml = archive.get_string("docProps/core.xml").unwrap().unwrap();
        assert!(core_xml.contains("<dc:title>Inserted Title</dc:title>"));
        // Existing author should remain
        assert!(core_xml.contains("<dc:creator>Existing Author</dc:creator>"));
    }

    // ==================== Sprint 18: Writer Block Generation Tests ====================

    #[test]
    fn test_generate_break_page() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::BreakType;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Before".to_string())],
                    ..Default::default()
                }),
                Block::Break(BreakType::Page),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("After".to_string())],
                    ..Default::default()
                }),
            ],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();
        assert!(!result.is_empty());

        // Verify the output contains a page break
        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("<w:br w:type=\"page\"/>"));
    }

    #[test]
    fn test_generate_break_section() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::BreakType;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Before section".to_string())],
                    ..Default::default()
                }),
                Block::Break(BreakType::Section),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("After section".to_string())],
                    ..Default::default()
                }),
            ],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        // Section break should generate sectPr or similar
        assert!(doc_xml.contains("Before section"));
        assert!(doc_xml.contains("After section"));
    }

    #[test]
    fn test_generate_literal_block() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::LiteralBlock;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Literal(LiteralBlock {
                content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
                language: Some("rust".to_string()),
                title: None,
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        // Should contain the code content
        assert!(doc_xml.contains("fn main()"));
        assert!(doc_xml.contains("println!"));
    }

    #[test]
    fn test_generate_literal_block_with_title() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::LiteralBlock;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Literal(LiteralBlock {
                content: "example code".to_string(),
                language: None,
                title: Some("Example".to_string()),
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("example code"));
    }

    #[test]
    fn test_generate_admonition_note() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::{Admonition, AdmonitionType};

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Admonition(Admonition {
                admonition_type: AdmonitionType::Note,
                title: None,
                content: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Important note here".to_string())],
                    ..Default::default()
                })],
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Important note here"));
    }

    #[test]
    fn test_generate_admonition_warning() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::{Admonition, AdmonitionType};

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Admonition(Admonition {
                admonition_type: AdmonitionType::Warning,
                title: Some(vec![Inline::Text("Danger!".to_string())]),
                content: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Be careful".to_string())],
                    ..Default::default()
                })],
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Be careful"));
    }

    #[test]
    fn test_generate_admonition_all_types() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::{Admonition, AdmonitionType};

        let types = [
            AdmonitionType::Note,
            AdmonitionType::Tip,
            AdmonitionType::Important,
            AdmonitionType::Warning,
            AdmonitionType::Caution,
        ];

        for admon_type in &types {
            let doc = Document {
                metadata: Default::default(),
                intent: None,
                blocks: vec![Block::Admonition(Admonition {
                    admonition_type: admon_type.clone(),
                    title: None,
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Content".to_string())],
                        ..Default::default()
                    })],
                })],
            };

            let template = create_minimal_template();
            let result = DocxWriter::generate(&doc, &template);
            assert!(result.is_ok(), "Failed for admonition type {:?}", admon_type);
        }
    }

    #[test]
    fn test_generate_image_inline() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Image(utf8dok_ast::Image {
                    src: "test.png".to_string(),
                    alt: Some("Test image".to_string()),
                })],
                ..Default::default()
            })],
        };

        let template = create_minimal_template();
        // This won't embed the actual image (file doesn't exist), but should not panic
        let result = DocxWriter::generate(&doc, &template);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_nested_list() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::List(List {
                list_type: ListType::Unordered,
                items: vec![
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Item 1".to_string())],
                            ..Default::default()
                        })],
                        level: 0,
                        term: None,
                    },
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Nested item".to_string())],
                            ..Default::default()
                        })],
                        level: 1,
                        term: None,
                    },
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Item 2".to_string())],
                            ..Default::default()
                        })],
                        level: 0,
                        term: None,
                    },
                ],
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Item 1"));
        assert!(doc_xml.contains("Nested item"));
        assert!(doc_xml.contains("Item 2"));
    }

    #[test]
    fn test_generate_ordered_list() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::List(List {
                list_type: ListType::Ordered,
                items: vec![
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("First".to_string())],
                            ..Default::default()
                        })],
                        level: 0,
                        term: None,
                    },
                    ListItem {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Second".to_string())],
                            ..Default::default()
                        })],
                        level: 0,
                        term: None,
                    },
                ],
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("First"));
        assert!(doc_xml.contains("Second"));
    }

    #[test]
    fn test_generate_table_with_header() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::{Table, TableCell, TableRow};

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Table(Table {
                caption: None,
                rows: vec![
                    TableRow {
                        cells: vec![
                            TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Col A".to_string())],
                                    ..Default::default()
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                            TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Col B".to_string())],
                                    ..Default::default()
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
                                    ..Default::default()
                                })],
                                colspan: 1,
                                rowspan: 1,
                                align: None,
                            },
                            TableCell {
                                content: vec![Block::Paragraph(Paragraph {
                                    inlines: vec![Inline::Text("Data 2".to_string())],
                                    ..Default::default()
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
                columns: vec![],
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Col A"));
        assert!(doc_xml.contains("Col B"));
        assert!(doc_xml.contains("Data 1"));
        assert!(doc_xml.contains("Data 2"));
    }

    #[test]
    fn test_generate_heading_with_anchor() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Heading(Heading {
                level: 2,
                text: vec![Inline::Text("Section Title".to_string())],
                anchor: Some("section-title".to_string()),
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Section Title"));
        // Heading level 2 should have Heading2 style
        assert!(doc_xml.contains("Heading2") || doc_xml.contains("w:pStyle"));
    }

    #[test]
    fn test_generate_inline_link_external() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::Link;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Link(Link {
                    url: "https://example.com".to_string(),
                    text: vec![Inline::Text("Example".to_string())],
                })],
                ..Default::default()
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Example"));
        assert!(doc_xml.contains("hyperlink"));
    }

    #[test]
    fn test_generate_inline_subscript_superscript() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![
                    Inline::Text("H".to_string()),
                    Inline::Format(
                        FormatType::Subscript,
                        Box::new(Inline::Text("2".to_string())),
                    ),
                    Inline::Text("O".to_string()),
                    Inline::Format(
                        FormatType::Superscript,
                        Box::new(Inline::Text("note".to_string())),
                    ),
                ],
                ..Default::default()
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("H"));
        assert!(doc_xml.contains("2"));
        assert!(doc_xml.contains("O"));
        // Subscript uses <w:vertAlign w:val="subscript"/>
        assert!(doc_xml.contains("vertAlign"));
    }

    #[test]
    fn test_generate_inline_highlight() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Format(
                    FormatType::Highlight,
                    Box::new(Inline::Text("important".to_string())),
                )],
                ..Default::default()
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("important"));
    }

    #[test]
    fn test_generate_empty_document() {
        use crate::test_utils::create_minimal_template;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        // Should still produce valid DOCX
        assert!(!result.is_empty());
        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("w:document"));
        assert!(doc_xml.contains("w:body"));
    }

    #[test]
    fn test_docx_writer_set_config_only() {
        let mut writer = DocxWriter::new();
        writer.set_config("key = \"value\"");
        assert!(writer.config_text.is_some());
        assert_eq!(writer.config_text.as_ref().unwrap(), "key = \"value\"");
    }

    #[test]
    fn test_docx_writer_set_source_only() {
        let mut writer = DocxWriter::new();
        writer.set_source("= Title\n\nContent");
        assert!(writer.source_text.is_some());
        assert!(writer.source_text.as_ref().unwrap().contains("Title"));
    }

    #[test]
    fn test_generate_with_manifest() {
        use crate::test_utils::create_template_with_styles;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Test".to_string())],
                ..Default::default()
            })],
        };

        let mut writer = DocxWriter::new();
        writer.set_source("= Test\n\nContent");
        writer.set_config("[utf8dok]\nversion = \"1.0\"");

        let template = create_template_with_styles();
        let template_obj = Template::from_bytes(&template).unwrap();
        let result = writer.generate_with_template(&doc, template_obj).unwrap();

        // Should have embedded content
        let cursor = Cursor::new(&result);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();
        assert!(archive.contains("utf8dok/source.adoc"));
        assert!(archive.contains("utf8dok/utf8dok.toml"));
    }

    // ==================== Sprint 21: Comments and Content Types Tests ====================

    #[test]
    fn test_generate_comments_xml_empty() {
        let writer = DocxWriter::new();
        assert!(writer.generate_comments_xml().is_none());
    }

    #[test]
    fn test_generate_comments_xml_with_comments() {
        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "rust".to_string(),
            author: "utf8dok".to_string(),
        });

        let xml = writer.generate_comments_xml();
        assert!(xml.is_some());

        let content = xml.unwrap();
        assert!(content.contains("w:comments"));
        assert!(content.contains("w:id=\"1\""));
        assert!(content.contains("w:author=\"utf8dok\""));
        assert!(content.contains("rust"));
    }

    #[test]
    fn test_generate_comments_xml_escapes_special_chars() {
        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "code with <tags> & \"quotes\"".to_string(),
            author: "Test <Author>".to_string(),
        });

        let xml = writer.generate_comments_xml().unwrap();
        assert!(xml.contains("&lt;tags&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&quot;quotes&quot;"));
        assert!(xml.contains("Test &lt;Author&gt;"));
    }

    #[test]
    fn test_generate_comments_xml_multiple_comments() {
        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "First".to_string(),
            author: "Author1".to_string(),
        });
        writer.comments.push(Comment {
            id: 2,
            text: "Second".to_string(),
            author: "Author2".to_string(),
        });

        let xml = writer.generate_comments_xml().unwrap();
        assert!(xml.contains("w:id=\"1\""));
        assert!(xml.contains("w:id=\"2\""));
        assert!(xml.contains("First"));
        assert!(xml.contains("Second"));
    }

    #[test]
    fn test_write_comments_adds_to_archive() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "test language".to_string(),
            author: "utf8dok".to_string(),
        });

        let result = writer.write_comments(&mut archive);
        assert!(result.is_ok());

        // Check comments.xml was created
        let comments = archive.get_string("word/comments.xml").unwrap();
        assert!(comments.is_some());
        assert!(comments.unwrap().contains("test language"));
    }

    #[test]
    fn test_write_comments_updates_relationships() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "test".to_string(),
            author: "utf8dok".to_string(),
        });

        writer.write_comments(&mut archive).unwrap();

        // Check relationships were updated
        let rels = archive
            .get_string("word/_rels/document.xml.rels")
            .unwrap()
            .unwrap();
        assert!(rels.contains("comments.xml"));
        assert!(rels.contains("relationships/comments"));
    }

    #[test]
    fn test_write_comments_updates_content_types() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let mut writer = DocxWriter::new();
        writer.comments.push(Comment {
            id: 1,
            text: "test".to_string(),
            author: "utf8dok".to_string(),
        });

        writer.write_comments(&mut archive).unwrap();

        // Check content types were updated
        let content_types = archive.get_string("[Content_Types].xml").unwrap().unwrap();
        assert!(content_types.contains("/word/comments.xml"));
    }

    #[test]
    fn test_write_comments_empty_does_nothing() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        let writer = DocxWriter::new();
        let result = writer.write_comments(&mut archive);
        assert!(result.is_ok());

        // Should not have created comments.xml
        assert!(archive.get("word/comments.xml").is_none());
    }

    #[test]
    fn test_update_content_types_adds_png() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Simulate having media files
        let mut writer = DocxWriter::new();
        writer.media_files.push(("word/media/image1.png".to_string(), vec![0x89, 0x50]));

        let result = writer.update_content_types(&mut archive);
        assert!(result.is_ok());

        let content_types = archive.get_string("[Content_Types].xml").unwrap().unwrap();
        assert!(content_types.contains("Extension=\"png\""));
        assert!(content_types.contains("image/png"));
    }

    #[test]
    fn test_update_content_types_does_not_duplicate() {
        use crate::archive::OoxmlArchive;
        use crate::test_utils::create_minimal_template;

        let template = create_minimal_template();
        let cursor = Cursor::new(&template);
        let mut archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Pre-add PNG extension
        let existing = archive.get_string("[Content_Types].xml").unwrap().unwrap();
        let with_png = existing.replace(
            "</Types>",
            "<Default Extension=\"png\" ContentType=\"image/png\"/></Types>",
        );
        archive.set_string("[Content_Types].xml", with_png);

        let mut writer = DocxWriter::new();
        writer.media_files.push(("word/media/image1.png".to_string(), vec![]));

        writer.update_content_types(&mut archive).unwrap();

        // Should not have duplicated
        let content = archive.get_string("[Content_Types].xml").unwrap().unwrap();
        let count = content.matches("Extension=\"png\"").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_literal_block_with_language_creates_comment() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::LiteralBlock;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Literal(LiteralBlock {
                content: "fn main() {}".to_string(),
                language: Some("rust".to_string()),
                title: None,
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        // Extract and check comments
        let cursor = Cursor::new(&result);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        let comments = archive.get_string("word/comments.xml").unwrap();
        assert!(comments.is_some());
        assert!(comments.unwrap().contains("rust"));
    }

    #[test]
    fn test_literal_block_without_language_no_comment() {
        use crate::test_utils::create_minimal_template;
        use utf8dok_ast::LiteralBlock;

        let doc = Document {
            metadata: Default::default(),
            intent: None,
            blocks: vec![Block::Literal(LiteralBlock {
                content: "plain text".to_string(),
                language: None,
                title: None,
                style_id: None,
            })],
        };

        let template = create_minimal_template();
        let result = DocxWriter::generate(&doc, &template).unwrap();

        let cursor = Cursor::new(&result);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Should not have comments.xml
        assert!(archive.get("word/comments.xml").is_none());
    }

    #[test]
    fn test_cover_page_with_full_metadata() {
        use crate::test_utils::create_template_with_styles;

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("Corporate Report".to_string());
        meta.authors = vec!["John Doe".to_string()];
        meta.revision = Some("1.0".to_string());
        meta.attributes
            .insert("revdate".to_string(), "2025-01-15".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Content".to_string())],
                ..Default::default()
            })],
        };

        let mut writer = DocxWriter::new();
        // Create a minimal cover image (1x1 PNG)
        let cover_png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        writer.set_cover_image("cover.png", cover_png);

        let template = create_template_with_styles();
        let template_obj = Template::from_bytes(&template).unwrap();
        let result = writer.generate_with_template(&doc, template_obj).unwrap();

        let cursor = Cursor::new(&result);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        // Should have cover image in media
        assert!(archive.contains("word/media/cover_cover.png"));

        // Document should contain the metadata text
        let doc_xml = crate::test_utils::extract_document_xml(&result);
        assert!(doc_xml.contains("Corporate Report"));
        assert!(doc_xml.contains("John Doe"));
    }

    #[test]
    fn test_cover_page_generates_page_break() {
        use crate::test_utils::create_template_with_styles;

        let mut meta = utf8dok_ast::DocumentMeta::default();
        meta.title = Some("Title".to_string());

        let doc = Document {
            metadata: meta,
            intent: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Body content".to_string())],
                ..Default::default()
            })],
        };

        let mut writer = DocxWriter::new();
        writer.set_cover_image("cover.png", vec![0x89, 0x50, 0x4E, 0x47]);

        let template = create_template_with_styles();
        let template_obj = Template::from_bytes(&template).unwrap();
        let result = writer.generate_with_template(&doc, template_obj).unwrap();

        let doc_xml = crate::test_utils::extract_document_xml(&result);
        // After cover page, should have page break before content
        assert!(doc_xml.contains("<w:br w:type=\"page\"/>"));
    }

    #[test]
    fn test_next_comment_id_increments() {
        let mut writer = DocxWriter::new();

        assert_eq!(writer.next_comment_id, 1);
        writer.next_comment_id += 1;
        assert_eq!(writer.next_comment_id, 2);
        writer.next_comment_id += 1;
        assert_eq!(writer.next_comment_id, 3);
    }

    #[test]
    fn test_next_image_id_increments() {
        let mut writer = DocxWriter::new();

        assert_eq!(writer.next_image_id, 1);
        writer.next_image_id += 1;
        assert_eq!(writer.next_image_id, 2);
    }

    #[test]
    fn test_next_drawing_id_increments() {
        let mut writer = DocxWriter::new();

        assert_eq!(writer.next_drawing_id, 1);
        writer.next_drawing_id += 1;
        assert_eq!(writer.next_drawing_id, 2);
    }

    #[test]
    fn test_media_files_collection() {
        let mut writer = DocxWriter::new();
        assert!(writer.media_files.is_empty());

        writer
            .media_files
            .push(("word/media/image1.png".to_string(), vec![1, 2, 3]));
        writer
            .media_files
            .push(("word/media/image2.png".to_string(), vec![4, 5, 6]));

        assert_eq!(writer.media_files.len(), 2);
        assert_eq!(writer.media_files[0].0, "word/media/image1.png");
        assert_eq!(writer.media_files[1].0, "word/media/image2.png");
    }

    #[test]
    fn test_diagram_sources_collection() {
        let mut writer = DocxWriter::new();
        assert!(writer.diagram_sources.is_empty());

        writer
            .diagram_sources
            .push(("utf8dok/diagrams/d1.mmd".to_string(), "graph TD".to_string()));

        assert_eq!(writer.diagram_sources.len(), 1);
        assert!(writer.diagram_sources[0].1.contains("graph TD"));
    }
}
