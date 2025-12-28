//! Document Parsing Coverage Tests
//!
//! Tests for `document::Document::parse` to increase code coverage.

use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

// =============================================================================
// Task 2: Complex Run Parsing
// =============================================================================

#[test]
fn test_parse_run_with_bold_false() {
    // Test that w:val="0" on <w:b> element (as Start event, not Empty) means NOT bold
    // Note: The parser checks w:val on Start events, not Empty events
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r>
                    <w:rPr>
                        <w:b w:val="0"></w:b>
                    </w:rPr>
                    <w:t>Not bold</w:t>
                </w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Run(run) = &p.children[0] {
            assert!(!run.bold, "Run with b w:val=\"0\" should not be bold");
            assert_eq!(run.text, "Not bold");
        } else {
            panic!("Expected Run");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_parse_run_with_italic_false() {
    // Test that w:val="false" on <w:i> element (as Start event) means NOT italic
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r>
                    <w:rPr>
                        <w:i w:val="false"></w:i>
                    </w:rPr>
                    <w:t>Not italic</w:t>
                </w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Run(run) = &p.children[0] {
            assert!(
                !run.italic,
                "Run with i w:val=\"false\" should not be italic"
            );
            assert_eq!(run.text, "Not italic");
        } else {
            panic!("Expected Run");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_parse_run_with_monospace_font() {
    // Parser expects <w:rFonts> as Start element, not Empty
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r>
                    <w:rPr>
                        <w:rFonts w:ascii="Consolas"></w:rFonts>
                    </w:rPr>
                    <w:t>Code text</w:t>
                </w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Run(run) = &p.children[0] {
            assert!(
                run.monospace,
                "Consolas font should be detected as monospace"
            );
            assert_eq!(run.text, "Code text");
        } else {
            panic!("Expected Run");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_parse_run_with_various_monospace_fonts() {
    // Parser expects <w:rFonts> as Start element, not Empty
    let fonts = vec![
        ("Courier New", true),     // Contains "courier"
        ("Menlo", true),           // Contains "menlo"
        ("Source Code Pro", true), // Contains "source code"
        ("Monaco", false),         // Not in monospace list
        ("Arial", false),          // Regular font
    ];

    for (font, expected) in fonts {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:rPr>
                            <w:rFonts w:ascii="{}"></w:rFonts>
                        </w:rPr>
                        <w:t>Test</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#,
            font
        );

        let doc = Document::parse(xml.as_bytes()).unwrap();
        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let ParagraphChild::Run(run) = &p.children[0] {
                assert_eq!(
                    run.monospace, expected,
                    "Font '{}' monospace detection",
                    font
                );
            }
        }
    }
}

#[test]
fn test_parse_empty_run() {
    // Runs with empty text should be filtered out
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r>
                    <w:t></w:t>
                </w:r>
                <w:r>
                    <w:t>Visible</w:t>
                </w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        assert_eq!(p.children.len(), 1, "Empty runs should be filtered out");
        if let ParagraphChild::Run(run) = &p.children[0] {
            assert_eq!(run.text, "Visible");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

// =============================================================================
// Task 2: Table Parsing
// =============================================================================

#[test]
fn test_parse_table_with_style() {
    // Parser expects <w:tblStyle> as Start element
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:tbl>
                <w:tblPr>
                    <w:tblStyle w:val="TableGrid"></w:tblStyle>
                </w:tblPr>
                <w:tr>
                    <w:tc>
                        <w:p><w:r><w:t>Cell</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
            </w:tbl>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Table(t) = &doc.blocks[0] {
        assert_eq!(t.style_id, Some("TableGrid".to_string()));
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.rows[0].cells.len(), 1);
    } else {
        panic!("Expected Table, got {:?}", doc.blocks[0]);
    }
}

#[test]
fn test_parse_table_multiple_rows_cells() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Table(t) = &doc.blocks[0] {
        assert_eq!(t.rows.len(), 2, "Should have 2 rows");
        assert_eq!(t.rows[0].cells.len(), 2, "Should have 2 cells per row");
        assert_eq!(t.rows[1].cells.len(), 2, "Should have 2 cells per row");
    } else {
        panic!("Expected Table");
    }
}

#[test]
fn test_parse_table_cell_multiple_paragraphs() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:tbl>
                <w:tr>
                    <w:tc>
                        <w:p><w:r><w:t>Line 1</w:t></w:r></w:p>
                        <w:p><w:r><w:t>Line 2</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
            </w:tbl>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Table(t) = &doc.blocks[0] {
        assert_eq!(
            t.rows[0].cells[0].paragraphs.len(),
            2,
            "Cell should have 2 paragraphs"
        );
    } else {
        panic!("Expected Table");
    }
}

// =============================================================================
// Task 2: Numbering / List Parsing
// =============================================================================

#[test]
fn test_parse_numbering_with_ilvl_first() {
    // Test when ilvl appears before numId
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
                <w:r><w:t>List item</w:t></w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        let num = p.numbering.as_ref().expect("Should have numbering");
        assert_eq!(num.ilvl, 2);
        assert_eq!(num.num_id, 5);
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_parse_numbering_empty_style() {
    // Test self-closing numId and ilvl
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:pPr>
                    <w:numPr>
                        <w:numId w:val="1"/>
                        <w:ilvl w:val="0"/>
                    </w:numPr>
                </w:pPr>
                <w:r><w:t>Bullet</w:t></w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        let num = p.numbering.as_ref().expect("Should have numbering");
        assert_eq!(num.ilvl, 0);
        assert_eq!(num.num_id, 1);
    } else {
        panic!("Expected Paragraph");
    }
}

// =============================================================================
// Task 2: Hyperlink Parsing
// =============================================================================

#[test]
fn test_parse_hyperlink_with_external_id() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
        <w:body>
            <w:p>
                <w:hyperlink r:id="rId1">
                    <w:r><w:t>External Link</w:t></w:r>
                </w:hyperlink>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Hyperlink(h) = &p.children[0] {
            assert_eq!(h.id, Some("rId1".to_string()));
            assert_eq!(h.anchor, None);
            assert_eq!(h.runs[0].text, "External Link");
        } else {
            panic!("Expected Hyperlink");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_parse_hyperlink_with_multiple_runs() {
    // Note: Parser uses trim_text(true), so trailing spaces may be trimmed
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:hyperlink w:anchor="bookmark1">
                    <w:r><w:t>Click</w:t></w:r>
                    <w:r><w:rPr><w:b/></w:rPr><w:t>here</w:t></w:r>
                </w:hyperlink>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Hyperlink(h) = &p.children[0] {
            assert_eq!(h.runs.len(), 2);
            assert_eq!(h.runs[0].text, "Click");
            assert!(!h.runs[0].bold);
            assert_eq!(h.runs[1].text, "here");
            assert!(h.runs[1].bold);
        } else {
            panic!("Expected Hyperlink");
        }
    } else {
        panic!("Expected Paragraph");
    }
}

// =============================================================================
// Task 2: Paragraph Methods
// =============================================================================

#[test]
fn test_paragraph_is_empty() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r><w:t>   </w:t></w:r>
            </w:p>
            <w:p>
                <w:r><w:t>Content</w:t></w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();

    if let Block::Paragraph(p1) = &doc.blocks[0] {
        assert!(p1.is_empty(), "Whitespace-only paragraph should be empty");
    }

    if let Block::Paragraph(p2) = &doc.blocks[1] {
        assert!(!p2.is_empty(), "Paragraph with content should not be empty");
    }
}

#[test]
fn test_paragraph_plain_text_with_hyperlink() {
    // Note: Parser uses trim_text(true), so leading/trailing spaces may be trimmed
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r><w:t>See</w:t></w:r>
                <w:hyperlink w:anchor="ref">
                    <w:r><w:t>link</w:t></w:r>
                </w:hyperlink>
                <w:r><w:t>for details.</w:t></w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        // Due to trim_text, spaces between elements are trimmed
        assert_eq!(p.plain_text(), "Seelinkfor details.");
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_paragraph_runs_iterator() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r><w:t>Run1</w:t></w:r>
                <w:hyperlink w:anchor="ref">
                    <w:r><w:t>Run2</w:t></w:r>
                    <w:r><w:t>Run3</w:t></w:r>
                </w:hyperlink>
                <w:r><w:t>Run4</w:t></w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        let runs: Vec<_> = p.runs().collect();
        assert_eq!(runs.len(), 4, "Should have 4 runs total");
        assert_eq!(runs[0].text, "Run1");
        assert_eq!(runs[1].text, "Run2");
        assert_eq!(runs[2].text, "Run3");
        assert_eq!(runs[3].text, "Run4");
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_document_paragraphs_iterator() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
            <w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Table Para</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>
            <w:p><w:r><w:t>Para 2</w:t></w:r></w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    let paragraphs: Vec<_> = doc.paragraphs().collect();

    assert_eq!(
        paragraphs.len(),
        3,
        "Should have 3 paragraphs (2 direct + 1 in table)"
    );
    assert_eq!(paragraphs[0].plain_text(), "Para 1");
    assert_eq!(paragraphs[1].plain_text(), "Table Para");
    assert_eq!(paragraphs[2].plain_text(), "Para 2");
}

#[test]
fn test_document_plain_text() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p><w:r><w:t>First</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second</w:t></w:r></w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    let text = doc.plain_text();

    assert!(text.contains("First"));
    assert!(text.contains("Second"));
    assert!(
        text.contains("\n\n"),
        "Paragraphs should be separated by double newline"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_parse_empty_document() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    assert!(
        doc.blocks.is_empty(),
        "Empty document should have no blocks"
    );
}

#[test]
fn test_parse_paragraph_outside_body() {
    // Paragraphs outside <w:body> should be ignored
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:p><w:r><w:t>Outside</w:t></w:r></w:p>
        <w:body>
            <w:p><w:r><w:t>Inside</w:t></w:r></w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(doc.plain_text(), "Inside");
}

#[test]
fn test_hyperlink_is_empty() {
    // Test is_empty for paragraph with hyperlink containing only whitespace
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:hyperlink w:anchor="ref">
                    <w:r><w:t>   </w:t></w:r>
                </w:hyperlink>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        assert!(
            p.is_empty(),
            "Paragraph with whitespace-only hyperlink should be empty"
        );
    }
}

#[test]
fn test_parse_self_closing_bold_italic() {
    // Test self-closing <w:b/> and <w:i/> elements
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
            <w:p>
                <w:r>
                    <w:rPr>
                        <w:b/>
                        <w:i/>
                    </w:rPr>
                    <w:t>Bold Italic</w:t>
                </w:r>
            </w:p>
        </w:body>
    </w:document>"#;

    let doc = Document::parse(xml).unwrap();
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let ParagraphChild::Run(run) = &p.children[0] {
            assert!(run.bold, "Self-closing <w:b/> should make text bold");
            assert!(run.italic, "Self-closing <w:i/> should make text italic");
        }
    }
}
