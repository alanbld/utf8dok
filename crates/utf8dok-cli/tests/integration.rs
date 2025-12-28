//! Integration tests for utf8dok CLI
//!
//! These tests verify the round-trip capability of utf8dok:
//! AsciiDoc -> DOCX -> AsciiDoc

use std::fs;
use std::io::{Cursor, Write};

use tempfile::TempDir;
use utf8dok_core::parse;
use utf8dok_ooxml::{DocxWriter, OoxmlArchive, Template};
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

    // word/styles.xml
    zip.start_file("word/styles.xml", options).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Normal" w:default="1">
    <w:name w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
  </w:style>
  <w:style w:type="table" w:styleId="TableGrid">
    <w:name w:val="Table Grid"/>
  </w:style>
</w:styles>"#).unwrap();

    // word/document.xml (empty body)
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
fn test_self_contained_roundtrip() {
    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create test AsciiDoc content
    let original_source = r#"= My Test Document

This is a *bold* and _italic_ test document.

== First Section

Here is some content in the first section.

=== Subsection

More detailed content here.

== Second Section

Final content.
"#;

    // Create test config
    let original_config = r#"# utf8dok configuration
[template]
path = "template.dotx"

[styles]
heading1 = "Heading1"
heading2 = "Heading2"
paragraph = "Normal"
"#;

    // Write test files
    let adoc_path = temp_path.join("test.adoc");
    fs::write(&adoc_path, original_source).expect("Failed to write test.adoc");

    let template_path = temp_path.join("template.dotx");
    fs::write(&template_path, create_test_template()).expect("Failed to write template.dotx");

    // Step 1: Parse AsciiDoc to AST
    let ast = parse(original_source).expect("Failed to parse AsciiDoc");

    // Step 2: Load template and create writer
    let template = Template::from_bytes(&create_test_template()).expect("Failed to load template");

    let mut writer = DocxWriter::new();
    writer.set_source(original_source);
    writer.set_config(original_config);

    // Step 3: Generate self-contained DOCX
    let docx_bytes = writer
        .generate_with_template(&ast, template)
        .expect("Failed to generate DOCX");

    // Step 4: Write output DOCX
    let output_path = temp_path.join("output.docx");
    fs::write(&output_path, &docx_bytes).expect("Failed to write output.docx");

    // Step 5: Verify the DOCX contains embedded content
    let archive = OoxmlArchive::open(&output_path).expect("Failed to open output.docx");

    // Check source.adoc was embedded
    let embedded_source = archive
        .get_string("utf8dok/source.adoc")
        .expect("Failed to read utf8dok/source.adoc")
        .expect("utf8dok/source.adoc not found");

    assert_eq!(
        embedded_source, original_source,
        "Embedded source should match original"
    );

    // Check utf8dok.toml was embedded
    let embedded_config = archive
        .get_string("utf8dok/utf8dok.toml")
        .expect("Failed to read utf8dok/utf8dok.toml")
        .expect("utf8dok/utf8dok.toml not found");

    assert_eq!(
        embedded_config, original_config,
        "Embedded config should match original"
    );

    // Check manifest exists and contains expected entries
    let manifest_json = archive
        .get_string("utf8dok/manifest.json")
        .expect("Failed to read manifest")
        .expect("manifest.json not found");

    assert!(manifest_json.contains("\"source\""), "Manifest should have source entry");
    assert!(manifest_json.contains("\"config\""), "Manifest should have config entry");
    assert!(manifest_json.contains("utf8dok/source.adoc"), "Manifest should reference source path");
    assert!(manifest_json.contains("utf8dok/utf8dok.toml"), "Manifest should reference config path");

    // Verify document content was generated
    let doc_xml = archive
        .get_string("word/document.xml")
        .expect("Failed to read document.xml")
        .expect("document.xml not found");

    // Check for content that should be present
    // Note: Level-1 headings may become document title metadata
    assert!(doc_xml.contains("First Section"), "Document should contain heading");
    assert!(doc_xml.contains("bold"), "Document should contain formatted text");
    assert!(doc_xml.contains("<w:pStyle"), "Document should have paragraph styles");
}

#[test]
fn test_embedded_source_extraction() {
    // Create test content
    let original_source = "= Simple Doc\n\nHello world.\n";
    let original_config = "[template]\npath = \"test.dotx\"\n";

    // Generate DOCX with embedded content
    let template = Template::from_bytes(&create_test_template()).expect("Failed to load template");
    let ast = parse(original_source).expect("Failed to parse");

    let mut writer = DocxWriter::new();
    writer.set_source(original_source);
    writer.set_config(original_config);

    let docx_bytes = writer
        .generate_with_template(&ast, template)
        .expect("Failed to generate");

    // Open the DOCX and extract embedded source
    let cursor = Cursor::new(&docx_bytes);
    let archive = OoxmlArchive::from_reader(cursor).expect("Failed to read DOCX");

    // Read the embedded source
    let extracted_source = archive
        .get_string("utf8dok/source.adoc")
        .expect("Failed to read")
        .expect("source not found");

    // Verify it matches
    assert_eq!(extracted_source, original_source);
}

#[test]
fn test_manifest_contains_hashes() {
    let source = "= Doc\n\nContent.\n";
    let config = "[template]\npath = \"x.dotx\"\n";

    let template = Template::from_bytes(&create_test_template()).unwrap();
    let ast = parse(source).unwrap();

    let mut writer = DocxWriter::new();
    writer.set_source(source);
    writer.set_config(config);

    let docx_bytes = writer.generate_with_template(&ast, template).unwrap();

    let cursor = Cursor::new(&docx_bytes);
    let archive = OoxmlArchive::from_reader(cursor).unwrap();

    let manifest = archive.get_string("utf8dok/manifest.json").unwrap().unwrap();

    // Manifest should contain hash fields for integrity verification
    assert!(manifest.contains("\"hash\""), "Manifest should contain hash field");

    // Parse manifest to verify structure
    let parsed: serde_json::Value = serde_json::from_str(&manifest).expect("Valid JSON");

    // Check source entry has hash
    if let Some(source_entry) = parsed.get("source") {
        assert!(source_entry.get("hash").is_some(), "Source entry should have hash");
    }

    // Check config entry has hash
    if let Some(config_entry) = parsed.get("config") {
        assert!(config_entry.get("hash").is_some(), "Config entry should have hash");
    }
}
