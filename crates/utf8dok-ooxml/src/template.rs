//! Template loader and management for DOTX/DOCX templates
//!
//! This module provides functionality to load and work with Word templates (.dotx files)
//! for template-aware document generation.
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ooxml::Template;
//!
//! let template = Template::load("corporate.dotx")?;
//! let styles = template.get_styles()?;
//! println!("Available styles: {:?}", styles.all().map(|s| &s.name).collect::<Vec<_>>());
//! ```

use std::path::Path;

use crate::archive::OoxmlArchive;
use crate::error::Result;
use crate::relationships::Relationships;
use crate::styles::StyleSheet;

/// A Word template (.dotx) wrapper providing template-specific operations
#[derive(Debug)]
pub struct Template {
    /// The underlying OOXML archive
    archive: OoxmlArchive,
    /// Parsed stylesheet (cached)
    stylesheet: Option<StyleSheet>,
}

impl Template {
    /// Load a template from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the .dotx or .docx file
    ///
    /// # Returns
    /// A loaded template ready for content injection
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let archive = OoxmlArchive::open(path)?;
        Ok(Self {
            archive,
            stylesheet: None,
        })
    }

    /// Load a template from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cursor = std::io::Cursor::new(bytes);
        let archive = OoxmlArchive::from_reader(cursor)?;
        Ok(Self {
            archive,
            stylesheet: None,
        })
    }

    /// Get the parsed stylesheet from the template
    ///
    /// Styles are cached after first parse.
    pub fn get_styles(&mut self) -> Result<&StyleSheet> {
        if self.stylesheet.is_none() {
            let styles_xml = self.archive.styles_xml()?;
            let stylesheet = StyleSheet::parse(styles_xml)?;
            self.stylesheet = Some(stylesheet);
        }
        Ok(self.stylesheet.as_ref().unwrap())
    }

    /// Get a mutable reference to the underlying archive
    pub fn archive_mut(&mut self) -> &mut OoxmlArchive {
        &mut self.archive
    }

    /// Get a reference to the underlying archive
    pub fn archive(&self) -> &OoxmlArchive {
        &self.archive
    }

    /// Consume the template and return the underlying archive
    pub fn into_archive(self) -> OoxmlArchive {
        self.archive
    }

    /// Get the document relationships from the template
    pub fn get_relationships(&self) -> Result<Relationships> {
        if let Some(rels_xml) = self.archive.document_rels_xml() {
            Relationships::parse(rels_xml)
        } else {
            Ok(Relationships::new())
        }
    }

    /// Get the raw document.xml content
    pub fn document_xml(&self) -> Result<&[u8]> {
        self.archive.document_xml()
    }

    /// Get header content if present
    pub fn header(&self, index: u32) -> Option<&[u8]> {
        self.archive.header_xml(index)
    }

    /// Get footer content if present
    pub fn footer(&self, index: u32) -> Option<&[u8]> {
        self.archive.footer_xml(index)
    }

    /// Check if the template has a utf8dok manifest (round-trip document)
    pub fn has_manifest(&self) -> bool {
        self.archive.has_utf8dok_file("manifest.json")
    }

    /// Get the list of available style IDs in the template
    pub fn available_style_ids(&mut self) -> Result<Vec<String>> {
        let styles = self.get_styles()?;
        Ok(styles.all().map(|s| s.id.clone()).collect())
    }

    /// Check if a specific style ID exists in the template
    pub fn has_style(&mut self, style_id: &str) -> Result<bool> {
        let styles = self.get_styles()?;
        Ok(styles.get(style_id).is_some())
    }

    /// Get heading style IDs present in the template
    pub fn heading_style_ids(&mut self) -> Result<Vec<String>> {
        let styles = self.get_styles()?;
        Ok(styles.heading_styles().map(|s| s.id.clone()).collect())
    }

    /// Get table style IDs present in the template
    pub fn table_style_ids(&mut self) -> Result<Vec<String>> {
        let styles = self.get_styles()?;
        Ok(styles.table_styles().map(|s| s.id.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    /// Create a minimal valid DOTX template for testing
    fn create_test_template() -> Vec<u8> {
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

        // word/styles.xml with corporate styles
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
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="2"/></w:pPr>
  </w:style>
  <w:style w:type="table" w:styleId="TableGrid">
    <w:name w:val="Table Grid"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="CodeBlock">
    <w:name w:val="Code Block"/>
    <w:basedOn w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet">
    <w:name w:val="List Bullet"/>
    <w:basedOn w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListNumber">
    <w:name w:val="List Number"/>
    <w:basedOn w:val="Normal"/>
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
    fn test_load_from_bytes() {
        let template_bytes = create_test_template();
        let result = Template::from_bytes(&template_bytes);
        assert!(
            result.is_ok(),
            "Failed to load template: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_get_styles() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let styles = template.get_styles();
        assert!(styles.is_ok(), "Failed to get styles: {:?}", styles.err());

        let styles = styles.unwrap();
        assert!(styles.get("Normal").is_some(), "Normal style should exist");
        assert!(
            styles.get("Heading1").is_some(),
            "Heading1 style should exist"
        );
        assert!(
            styles.get("TableGrid").is_some(),
            "TableGrid style should exist"
        );
    }

    #[test]
    fn test_available_style_ids() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let style_ids = template.available_style_ids().unwrap();
        assert!(style_ids.contains(&"Normal".to_string()));
        assert!(style_ids.contains(&"Heading1".to_string()));
        assert!(style_ids.contains(&"Heading2".to_string()));
        assert!(style_ids.contains(&"TableGrid".to_string()));
    }

    #[test]
    fn test_has_style() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        assert!(template.has_style("Normal").unwrap());
        assert!(template.has_style("Heading1").unwrap());
        assert!(!template.has_style("NonExistent").unwrap());
    }

    #[test]
    fn test_heading_style_ids() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let heading_ids = template.heading_style_ids().unwrap();
        assert!(heading_ids.contains(&"Heading1".to_string()));
        assert!(heading_ids.contains(&"Heading2".to_string()));
        assert!(heading_ids.contains(&"Heading3".to_string()));
        assert!(!heading_ids.contains(&"Normal".to_string()));
    }

    #[test]
    fn test_table_style_ids() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let table_ids = template.table_style_ids().unwrap();
        assert!(table_ids.contains(&"TableGrid".to_string()));
    }

    #[test]
    fn test_get_relationships() {
        let template_bytes = create_test_template();
        let template = Template::from_bytes(&template_bytes).unwrap();

        let rels = template.get_relationships();
        assert!(rels.is_ok());
    }

    // ==================== Sprint 6: Boundary Tests ====================

    #[test]
    fn test_load_from_invalid_bytes() {
        // Completely invalid bytes (not a ZIP)
        let invalid_bytes = b"This is not a ZIP file";
        let result = Template::from_bytes(invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_truncated_zip() {
        // Start of a valid ZIP but truncated
        let truncated = &[0x50, 0x4b, 0x03, 0x04, 0x00, 0x00];
        let result = Template::from_bytes(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_styles_caching() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        // First call loads styles
        let styles1 = template.get_styles().unwrap();
        let has_normal1 = styles1.get("Normal").is_some();

        // Second call should return cached styles
        let styles2 = template.get_styles().unwrap();
        let has_normal2 = styles2.get("Normal").is_some();

        // Both calls should return consistent results
        assert!(has_normal1);
        assert!(has_normal2);
    }

    #[test]
    fn test_has_style_before_get_styles() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        // has_style should work without explicitly calling get_styles first
        assert!(template.has_style("Normal").unwrap());
        assert!(template.has_style("Heading1").unwrap());
        assert!(!template.has_style("NonExistent").unwrap());
    }

    #[test]
    fn test_available_style_ids_count() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let style_ids = template.available_style_ids().unwrap();
        // Our test template has: Normal, Heading1, Heading2, Heading3, TableGrid, CodeBlock, ListBullet, ListNumber
        assert!(style_ids.len() >= 7, "Expected at least 7 styles, got {}", style_ids.len());
    }

    #[test]
    fn test_heading_style_levels() {
        let template_bytes = create_test_template();
        let mut template = Template::from_bytes(&template_bytes).unwrap();

        let heading_ids = template.heading_style_ids().unwrap();
        // Should have Heading1, Heading2, Heading3 from test template
        assert_eq!(heading_ids.len(), 3);
    }
}
