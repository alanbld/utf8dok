//! Integration tests for data include directives
//!
//! Tests the `include::` directive parsing and resolution.

use std::path::PathBuf;

use utf8dok_ast::Block;
use utf8dok_core::{parse_with_config, IncludeDirective, ParserConfig};

/// Get the path to test fixtures
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("utf8dok-data")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Get the fixture directory
fn fixture_dir() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("utf8dok-data")
        .join("tests")
        .join("fixtures")
        .to_string_lossy()
        .to_string()
}

#[test]
fn test_parse_include_directive() {
    let line = "include::data.xlsx[sheet=Sales,range=A1:C10,header]";
    let directive = IncludeDirective::parse(line).unwrap();

    assert_eq!(directive.path, "data.xlsx");
    assert_eq!(directive.sheet, Some("Sales".to_string()));
    assert_eq!(directive.range, Some("A1:C10".to_string()));
    assert!(directive.header);
    assert!(directive.is_data_file());
}

#[test]
fn test_parse_include_csv() {
    let line = "include::report.csv[header,delimiter=;]";
    let directive = IncludeDirective::parse(line).unwrap();

    assert_eq!(directive.path, "report.csv");
    assert!(directive.header);
    assert_eq!(directive.delimiter, Some(';'));
    assert!(directive.is_data_file());
}

#[test]
fn test_parse_include_not_data_file() {
    let line = "include::chapter.adoc[]";
    let directive = IncludeDirective::parse(line).unwrap();

    assert!(!directive.is_data_file());
}

#[test]
fn test_parse_document_with_excel_include() {
    let fixture = fixture_path("test_data.xlsx");
    if !fixture.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", fixture);
        return;
    }

    let input = format!(
        r#"= Report

== Data

include::test_data.xlsx[sheet=TestData,range=A1:C4,header]
"#
    );

    let config = ParserConfig::with_data_includes(fixture_dir());
    let doc = parse_with_config(&input, config).unwrap();

    // Should have 2 blocks: heading and table
    assert_eq!(doc.blocks.len(), 2);

    // First block should be heading
    assert!(matches!(doc.blocks[0], Block::Heading(_)));

    // Second block should be a table with 4 rows
    if let Block::Table(table) = &doc.blocks[1] {
        assert_eq!(table.rows.len(), 4);
        assert!(table.rows[0].is_header);
        assert!(!table.rows[1].is_header);

        // Check header content
        assert_eq!(table.rows[0].cells.len(), 3);
    } else {
        panic!("Expected Table block, got {:?}", doc.blocks[1]);
    }
}

#[test]
fn test_parse_document_with_include_disabled() {
    let input = r#"= Report

include::data.xlsx[header]
"#;

    // Parse without data includes enabled
    let config = ParserConfig::default();
    let doc = parse_with_config(&input, config).unwrap();

    // Should have 1 block (the include is ignored as paragraph)
    // Non-data includes just return None, so it becomes paragraph content
    assert!(!doc.blocks.is_empty());
}

#[test]
fn test_parse_document_with_missing_file() {
    let input = r#"= Report

include::nonexistent.xlsx[header]
"#;

    let config = ParserConfig::with_data_includes(".");
    let doc = parse_with_config(&input, config).unwrap();

    // Should have 1 block with error placeholder
    assert_eq!(doc.blocks.len(), 1);

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let text = p
            .inlines
            .iter()
            .filter_map(|i| {
                if let utf8dok_ast::Inline::Text(t) = i {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect::<String>();
        assert!(
            text.contains("Include error"),
            "Expected error placeholder, got: {}",
            text
        );
    } else {
        panic!("Expected Paragraph block with error");
    }
}

#[test]
fn test_include_directive_extensions() {
    assert!(IncludeDirective::parse("include::data.xlsx[]")
        .unwrap()
        .is_data_file());
    assert!(IncludeDirective::parse("include::data.xls[]")
        .unwrap()
        .is_data_file());
    assert!(IncludeDirective::parse("include::data.csv[]")
        .unwrap()
        .is_data_file());
    assert!(IncludeDirective::parse("include::data.tsv[]")
        .unwrap()
        .is_data_file());
    assert!(!IncludeDirective::parse("include::data.adoc[]")
        .unwrap()
        .is_data_file());
    assert!(!IncludeDirective::parse("include::data.txt[]")
        .unwrap()
        .is_data_file());
}

#[test]
fn test_parse_document_mixed_content() {
    let fixture = fixture_path("test_data.xlsx");
    if !fixture.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", fixture);
        return;
    }

    let input = r#"= Mixed Document

== Introduction

This document includes data from Excel.

include::test_data.xlsx[range=A1:C2,header]

== Conclusion

The data has been loaded.
"#;

    let config = ParserConfig::with_data_includes(fixture_dir());
    let doc = parse_with_config(&input, config).unwrap();

    // Count blocks
    let heading_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Heading(_)))
        .count();
    let table_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Table(_)))
        .count();
    let para_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Paragraph(_)))
        .count();

    assert_eq!(heading_count, 2, "Should have 2 headings");
    assert_eq!(table_count, 1, "Should have 1 table");
    assert_eq!(para_count, 2, "Should have 2 paragraphs");
}
