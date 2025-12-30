//! Writer Coverage Tests
//!
//! Tests for `DocxWriter` to increase code coverage beyond the inline tests.

use std::collections::HashMap;
use std::io::Cursor;

use utf8dok_ast::{
    Admonition, AdmonitionType, Block, BreakType, ColumnSpec, Document, DocumentMeta, FormatType,
    Heading, Inline, Link, List, ListItem, ListType, LiteralBlock, Paragraph, Table, TableCell,
    TableRow,
};
use utf8dok_ooxml::archive::OoxmlArchive;
use utf8dok_ooxml::writer::DocxWriter;

/// Create a minimal valid DOCX template for testing
fn create_minimal_template() -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    use zip::ZipWriter;

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

    // word/document.xml (placeholder)
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

/// Helper to extract document.xml from a generated DOCX
fn extract_document_xml(docx: &[u8]) -> String {
    let cursor = Cursor::new(docx);
    let archive = OoxmlArchive::from_reader(cursor).unwrap();
    archive.get_string("word/document.xml").unwrap().unwrap()
}

// =============================================================================
// Task 1: Complex Table Tests
// =============================================================================

#[test]
fn test_write_complex_table_with_colspan_rowspan() {
    let template = create_minimal_template();

    // Table with colspan and rowspan
    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Table(Table {
            rows: vec![
                // Header row
                TableRow {
                    cells: vec![TableCell {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Merged Header".to_string())],
                            style_id: None,
                            attributes: HashMap::new(),
                        })],
                        colspan: 2, // Spans 2 columns
                        rowspan: 1,
                        align: None,
                    }],
                    is_header: true,
                },
                // Data row with rowspan
                TableRow {
                    cells: vec![
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Spans 2 rows".to_string())],
                                style_id: None,
                                attributes: HashMap::new(),
                            })],
                            colspan: 1,
                            rowspan: 2, // Spans 2 rows
                            align: None,
                        },
                        TableCell {
                            content: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("Row 1".to_string())],
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
            columns: vec![
                ColumnSpec {
                    width: Some(2000),
                    align: None,
                },
                ColumnSpec {
                    width: Some(3000),
                    align: None,
                },
            ],
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify colspan
    assert!(
        xml.contains("<w:gridSpan w:val=\"2\"/>"),
        "Should have colspan: {}",
        xml
    );
    // Verify rowspan (vMerge)
    assert!(
        xml.contains("<w:vMerge w:val=\"restart\"/>"),
        "Should have rowspan: {}",
        xml
    );
    // Verify grid columns
    assert!(
        xml.contains("<w:tblGrid>"),
        "Should have table grid: {}",
        xml
    );
    assert!(
        xml.contains("<w:gridCol"),
        "Should have grid columns: {}",
        xml
    );
    // Verify header row
    assert!(
        xml.contains("<w:tblHeader/>"),
        "Should have header marker: {}",
        xml
    );
}

#[test]
fn test_write_table_with_empty_cell() {
    let template = create_minimal_template();

    // Table with an empty cell (should get an empty paragraph)
    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Table(Table {
            rows: vec![TableRow {
                cells: vec![
                    TableCell {
                        content: vec![], // Empty cell
                        colspan: 1,
                        rowspan: 1,
                        align: None,
                    },
                    TableCell {
                        content: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("Data".to_string())],
                            style_id: None,
                            attributes: HashMap::new(),
                        })],
                        colspan: 1,
                        rowspan: 1,
                        align: None,
                    },
                ],
                is_header: false,
            }],
            style_id: None,
            caption: None,
            columns: vec![],
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Empty cell should have an empty paragraph <w:p/>
    assert!(
        xml.contains("<w:p/>"),
        "Empty cell should have empty paragraph: {}",
        xml
    );
}

// =============================================================================
// Task 2: Admonition Tests
// =============================================================================

#[test]
fn test_write_all_admonition_types() {
    let template = create_minimal_template();

    let admonition_types = vec![
        (AdmonitionType::Note, "Note"),
        (AdmonitionType::Tip, "Tip"),
        (AdmonitionType::Important, "Important"),
        (AdmonitionType::Warning, "Warning"),
        (AdmonitionType::Caution, "Caution"),
    ];

    for (admon_type, type_name) in admonition_types {
        let doc = Document {
            metadata: DocumentMeta::default(),
            intent: None,
            blocks: vec![Block::Admonition(Admonition {
                admonition_type: admon_type,
                title: Some(vec![Inline::Text(format!("{} Title", type_name))]),
                content: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text(format!("This is a {} message.", type_name))],
                    style_id: None,
                    attributes: HashMap::new(),
                })],
            })],
        };

        let result = DocxWriter::generate(&doc, &template).unwrap();
        let xml = extract_document_xml(&result);

        // Verify title style
        assert!(
            xml.contains(&format!("<w:pStyle w:val=\"{}Title\"/>", type_name)),
            "Should have {} title style: {}",
            type_name,
            xml
        );
        // Verify content
        assert!(
            xml.contains(&format!("This is a {} message.", type_name)),
            "Should have {} content: {}",
            type_name,
            xml
        );
    }
}

#[test]
fn test_write_admonition_without_title() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Admonition(Admonition {
            admonition_type: AdmonitionType::Note,
            title: None, // No title
            content: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Just a note.".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            })],
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Should NOT have title style (no title paragraph)
    assert!(
        !xml.contains("<w:pStyle w:val=\"NoteTitle\"/>"),
        "Should not have NoteTitle style without title: {}",
        xml
    );
    // Should have content
    assert!(xml.contains("Just a note."), "Should have content: {}", xml);
}

// =============================================================================
// Task 3: Nested List Tests
// =============================================================================

#[test]
fn test_write_nested_unordered_list() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::List(List {
            list_type: ListType::Unordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Level 0 item".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Nested level 1".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 1,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Deep level 2".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 2,
                    term: None,
                },
            ],
            style_id: None,
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify different indent levels
    assert!(
        xml.contains("<w:ilvl w:val=\"0\"/>"),
        "Should have level 0: {}",
        xml
    );
    assert!(
        xml.contains("<w:ilvl w:val=\"1\"/>"),
        "Should have level 1: {}",
        xml
    );
    assert!(
        xml.contains("<w:ilvl w:val=\"2\"/>"),
        "Should have level 2: {}",
        xml
    );
    // Verify unordered list numId
    assert!(
        xml.contains("<w:numId w:val=\"1\"/>"),
        "Should have unordered numId: {}",
        xml
    );
}

#[test]
fn test_write_ordered_list() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::List(List {
            list_type: ListType::Ordered,
            items: vec![
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Step 1".to_string())],
                        style_id: None,
                        attributes: HashMap::new(),
                    })],
                    level: 0,
                    term: None,
                },
                ListItem {
                    content: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("Step 2".to_string())],
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

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify ordered list style and numId
    assert!(
        xml.contains("<w:pStyle w:val=\"ListNumber\"/>"),
        "Should have ListNumber style: {}",
        xml
    );
    assert!(
        xml.contains("<w:numId w:val=\"2\"/>"),
        "Should have ordered numId: {}",
        xml
    );
}

#[test]
fn test_write_description_list() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::List(List {
            list_type: ListType::Description,
            items: vec![ListItem {
                content: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Definition".to_string())],
                    style_id: None,
                    attributes: HashMap::new(),
                })],
                level: 0,
                term: Some(vec![Inline::Text("Term".to_string())]),
            }],
            style_id: None,
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify description list style and numId
    assert!(
        xml.contains("<w:pStyle w:val=\"ListParagraph\"/>"),
        "Should have ListParagraph style: {}",
        xml
    );
    assert!(
        xml.contains("<w:numId w:val=\"3\"/>"),
        "Should have description numId: {}",
        xml
    );
}

#[test]
fn test_write_list_with_custom_style() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::List(List {
            list_type: ListType::Unordered,
            items: vec![ListItem {
                content: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Custom styled item".to_string())],
                    style_id: None,
                    attributes: HashMap::new(),
                })],
                level: 0,
                term: None,
            }],
            style_id: Some("CustomListStyle".to_string()),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify custom style is used
    assert!(
        xml.contains("<w:pStyle w:val=\"CustomListStyle\"/>"),
        "Should have custom style: {}",
        xml
    );
}

#[test]
fn test_write_list_with_non_paragraph_content() {
    let template = create_minimal_template();

    // List item containing a heading (non-paragraph block)
    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::List(List {
            list_type: ListType::Unordered,
            items: vec![ListItem {
                content: vec![Block::Heading(Heading {
                    level: 2,
                    text: vec![Inline::Text("Heading in list".to_string())],
                    style_id: None,
                    anchor: None,
                })],
                level: 0,
                term: None,
            }],
            style_id: None,
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify heading is rendered as a block
    assert!(
        xml.contains("Heading in list"),
        "Should have heading text: {}",
        xml
    );
    assert!(
        xml.contains("<w:pStyle w:val=\"Heading2\"/>"),
        "Should have Heading2 style: {}",
        xml
    );
}

// =============================================================================
// Task 4: Hyperlink Tests
// =============================================================================

#[test]
fn test_write_internal_hyperlink() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![Inline::Link(Link {
                url: "#section-intro".to_string(), // Internal link
                text: vec![Inline::Text("See Introduction".to_string())],
            })],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify internal hyperlink
    assert!(
        xml.contains("<w:hyperlink w:anchor=\"section-intro\">"),
        "Should have anchor hyperlink: {}",
        xml
    );
    assert!(
        xml.contains("<w:rStyle w:val=\"Hyperlink\"/>"),
        "Should have Hyperlink style: {}",
        xml
    );
    assert!(
        xml.contains("See Introduction"),
        "Should have link text: {}",
        xml
    );
}

#[test]
fn test_write_external_hyperlink() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![Inline::Link(Link {
                url: "https://example.com".to_string(), // External link
                text: vec![Inline::Text("Visit Example".to_string())],
            })],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate_with_options(&doc, &template, false).unwrap();
    let xml = extract_document_xml(&result);

    // External links use r:id and Hyperlink style
    assert!(
        xml.contains("r:id=\"rId"),
        "Should have relationship ID: {}",
        xml
    );
    assert!(
        xml.contains("<w:rStyle w:val=\"Hyperlink\"/>"),
        "Should have Hyperlink style: {}",
        xml
    );
    assert!(
        xml.contains("Visit Example"),
        "Should have link text: {}",
        xml
    );
}

// =============================================================================
// Additional Coverage Tests
// =============================================================================

#[test]
fn test_write_all_format_types() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Format(
                    FormatType::Highlight,
                    Box::new(Inline::Text("highlighted".to_string())),
                ),
                Inline::Text(" ".to_string()),
                Inline::Format(
                    FormatType::Superscript,
                    Box::new(Inline::Text("super".to_string())),
                ),
                Inline::Text(" ".to_string()),
                Inline::Format(
                    FormatType::Subscript,
                    Box::new(Inline::Text("sub".to_string())),
                ),
            ],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify highlight
    assert!(
        xml.contains("<w:highlight w:val=\"yellow\"/>"),
        "Should have highlight: {}",
        xml
    );
    // Verify superscript
    assert!(
        xml.contains("<w:vertAlign w:val=\"superscript\"/>"),
        "Should have superscript: {}",
        xml
    );
    // Verify subscript
    assert!(
        xml.contains("<w:vertAlign w:val=\"subscript\"/>"),
        "Should have subscript: {}",
        xml
    );
}

#[test]
fn test_write_break_types() {
    let template = create_minimal_template();

    // Test page break
    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![
            Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Before break".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            }),
            Block::Break(BreakType::Page),
            Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("After break".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            }),
        ],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    assert!(
        xml.contains("<w:br w:type=\"page\"/>"),
        "Should have page break: {}",
        xml
    );
}

#[test]
fn test_write_section_break() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![
            Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Section 1".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            }),
            Block::Break(BreakType::Section),
            Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Section 2".to_string())],
                style_id: None,
                attributes: HashMap::new(),
            }),
        ],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Section break uses sectPr
    assert!(xml.contains("<w:sectPr>"), "Should have sectPr: {}", xml);
    assert!(
        xml.contains("<w:type w:val=\"nextPage\"/>"),
        "Should have nextPage type: {}",
        xml
    );
}

#[test]
fn test_write_literal_block() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        blocks: vec![Block::Literal(LiteralBlock {
            content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            language: Some("rust".to_string()),
            title: None,
            style_id: Some("CodeBlock".to_string()),
        })],
        intent: None,
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify code block style
    assert!(
        xml.contains("<w:pStyle w:val=\"CodeBlock\"/>"),
        "Should have CodeBlock style: {}",
        xml
    );
    // Verify monospace font
    assert!(
        xml.contains("Courier New"),
        "Should have Courier New font: {}",
        xml
    );
    // Verify xml:space preserve
    assert!(
        xml.contains("xml:space=\"preserve\""),
        "Should preserve whitespace: {}",
        xml
    );
}

#[test]
fn test_write_inline_span() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![Inline::Span(vec![
                Inline::Text("First ".to_string()),
                Inline::Text("Second".to_string()),
            ])],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    assert!(xml.contains("First "), "Should have First: {}", xml);
    assert!(xml.contains("Second"), "Should have Second: {}", xml);
}

#[test]
fn test_write_inline_image() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![Inline::Image(utf8dok_ast::Image {
                src: "media/image.png".to_string(),
                alt: Some("Alt text".to_string()),
            })],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Images should now generate actual drawing XML
    assert!(
        xml.contains("<w:drawing>"),
        "Should have drawing element: {}",
        xml
    );
    assert!(
        xml.contains("<wp:inline"),
        "Should have inline positioning: {}",
        xml
    );
    assert!(
        xml.contains(r#"descr="Alt text""#),
        "Should have alt text: {}",
        xml
    );
    assert!(
        xml.contains("<a:blip"),
        "Should have blip reference: {}",
        xml
    );
    assert!(
        xml.contains("<pic:pic"),
        "Should have picture element: {}",
        xml
    );
}

#[test]
fn test_write_inline_break() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![
                Inline::Text("Line 1".to_string()),
                Inline::Break,
                Inline::Text("Line 2".to_string()),
            ],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    assert!(xml.contains("<w:br/>"), "Should have inline break: {}", xml);
}

#[test]
fn test_write_heading_levels() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![
            Block::Heading(Heading {
                level: 1,
                text: vec![Inline::Text("H1".to_string())],
                style_id: None,
                anchor: None,
            }),
            Block::Heading(Heading {
                level: 4,
                text: vec![Inline::Text("H4".to_string())],
                style_id: None,
                anchor: None,
            }),
            Block::Heading(Heading {
                level: 6,
                text: vec![Inline::Text("H6".to_string())],
                style_id: None,
                anchor: None,
            }),
            Block::Heading(Heading {
                level: 7, // Beyond 6, should default to Heading1
                text: vec![Inline::Text("H7".to_string())],
                style_id: None,
                anchor: None,
            }),
        ],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    assert!(
        xml.contains("<w:pStyle w:val=\"Heading1\"/>"),
        "Should have Heading1: {}",
        xml
    );
    assert!(
        xml.contains("<w:pStyle w:val=\"Heading4\"/>"),
        "Should have Heading4: {}",
        xml
    );
    assert!(
        xml.contains("<w:pStyle w:val=\"Heading6\"/>"),
        "Should have Heading6: {}",
        xml
    );
}

#[test]
fn test_write_heading_with_custom_style() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Custom".to_string())],
            style_id: Some("MyHeadingStyle".to_string()),
            anchor: None,
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    assert!(
        xml.contains("<w:pStyle w:val=\"MyHeadingStyle\"/>"),
        "Should use custom style: {}",
        xml
    );
}

#[test]
fn test_xml_escaping() {
    let template = create_minimal_template();

    let doc = Document {
        metadata: DocumentMeta::default(),
        intent: None,
        blocks: vec![Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text("A & B < C > D \"E\" 'F'".to_string())],
            style_id: None,
            attributes: HashMap::new(),
        })],
    };

    let result = DocxWriter::generate(&doc, &template).unwrap();
    let xml = extract_document_xml(&result);

    // Verify all special characters are escaped
    assert!(xml.contains("&amp;"), "Should escape ampersand: {}", xml);
    assert!(xml.contains("&lt;"), "Should escape less-than: {}", xml);
    assert!(xml.contains("&gt;"), "Should escape greater-than: {}", xml);
    assert!(xml.contains("&quot;"), "Should escape quote: {}", xml);
    assert!(xml.contains("&apos;"), "Should escape apostrophe: {}", xml);
}
