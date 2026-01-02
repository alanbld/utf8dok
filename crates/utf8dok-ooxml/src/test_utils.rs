//! Shared test utilities for utf8dok-ooxml
//!
//! This module provides common fixtures and helpers used across tests.

use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::archive::OoxmlArchive;

/// Create a minimal valid DOCX template for testing
///
/// This creates a valid DOCX ZIP structure with:
/// - [Content_Types].xml
/// - _rels/.rels
/// - word/_rels/document.xml.rels
/// - word/document.xml (placeholder content)
///
/// # Example
/// ```ignore
/// use utf8dok_ooxml::test_utils::create_minimal_template;
/// let template = create_minimal_template();
/// ```
pub fn create_minimal_template() -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .unwrap();

    // _rels/.rels
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .unwrap();

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

/// Create a minimal DOCX template with styles.xml
///
/// Includes basic heading styles for testing style-aware features.
pub fn create_template_with_styles() -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#,
    )
    .unwrap();

    // _rels/.rels
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .unwrap();

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

    // word/styles.xml
    zip.start_file("word/styles.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
</w:styles>"#,
    )
    .unwrap();

    // word/document.xml
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Template with styles</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
    )
    .unwrap();

    zip.finish().unwrap();
    buffer.into_inner()
}

/// Extract document.xml content from a DOCX byte array
pub fn extract_document_xml(docx: &[u8]) -> String {
    let cursor = Cursor::new(docx);
    let archive = OoxmlArchive::from_reader(cursor).unwrap();
    archive.get_string("word/document.xml").unwrap().unwrap()
}

/// Extract any file content from a DOCX byte array
pub fn extract_file(docx: &[u8], path: &str) -> Option<String> {
    let cursor = Cursor::new(docx);
    let archive = OoxmlArchive::from_reader(cursor).unwrap();
    archive.get_string(path).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_minimal_template() {
        let template = create_minimal_template();
        assert!(!template.is_empty());

        // Verify it's a valid ZIP
        let cursor = Cursor::new(&template);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        assert!(archive.contains("[Content_Types].xml"));
        assert!(archive.contains("word/document.xml"));
        assert!(archive.contains("_rels/.rels"));
    }

    #[test]
    fn test_create_template_with_styles() {
        let template = create_template_with_styles();
        assert!(!template.is_empty());

        let cursor = Cursor::new(&template);
        let archive = OoxmlArchive::from_reader(cursor).unwrap();

        assert!(archive.contains("word/styles.xml"));

        let styles = archive.get_string("word/styles.xml").unwrap().unwrap();
        assert!(styles.contains("Heading1"));
        assert!(styles.contains("Heading2"));
    }

    #[test]
    fn test_extract_document_xml() {
        let template = create_minimal_template();
        let doc_xml = extract_document_xml(&template);

        assert!(doc_xml.contains("w:document"));
        assert!(doc_xml.contains("Template"));
    }

    #[test]
    fn test_extract_file() {
        let template = create_minimal_template();

        let content_types = extract_file(&template, "[Content_Types].xml");
        assert!(content_types.is_some());
        assert!(content_types.unwrap().contains("Types"));

        let missing = extract_file(&template, "nonexistent.xml");
        assert!(missing.is_none());
    }
}
