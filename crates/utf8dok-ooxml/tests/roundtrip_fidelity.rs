//! Round-Trip Fidelity Tests (ADR-006)
//!
//! TDD tests for closing the gaps in DOCX extraction → AsciiDoc → DOCX rendering.
//! These tests define expected behavior BEFORE implementation.
//!
//! Gap Analysis (SWP Application Architecture.docx):
//! - Paragraphs: 89.2% (933→833) → Target: 95%+
//! - Drawings: 72.9% (37→27) → Target: 85%+
//! - Hyperlinks: 75.3% (69→52) → Target: 90%+
//!
//! Test Categories:
//! 1. DrawingML Shape Text Extraction
//! 2. TOC Hyperlink Preservation
//! 3. Internal Cross-Reference Round-Trip
//! 4. Complex Shape Groups

// =============================================================================
// PART 1: DRAWINGML SHAPE TEXT EXTRACTION
// =============================================================================

mod shape_text_extraction {
    //! Tests for extracting text from various DrawingML shape types
    //!
    //! OOXML shape hierarchy:
    //! - <wps:wsp> - WordprocessingML shape (text boxes, callouts)
    //! - <wpg:wgp> - WordprocessingML group (container for multiple shapes)
    //! - <a:graphic> - DrawingML graphic container
    //! - <dgm:*> - Diagram shapes (SmartArt)

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

    /// Test extracting text from a simple WordprocessingML shape
    /// NOTE: This should pass - txbxContent is already handled
    #[test]
    fn test_extract_wsp_shape_text() {
        // <wps:wsp> contains <wps:txbx> with <w:txbxContent>
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
            <w:body>
                <w:p>
                    <w:r>
                        <mc:AlternateContent>
                            <mc:Choice>
                                <w:drawing>
                                    <wp:anchor>
                                        <a:graphic>
                                            <a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
                                                <wps:wsp>
                                                    <wps:txbx>
                                                        <w:txbxContent>
                                                            <w:p>
                                                                <w:r><w:t>Shape text content</w:t></w:r>
                                                            </w:p>
                                                        </w:txbxContent>
                                                    </wps:txbx>
                                                </wps:wsp>
                                            </a:graphicData>
                                        </a:graphic>
                                    </wp:anchor>
                                </w:drawing>
                            </mc:Choice>
                        </mc:AlternateContent>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should extract text from shape as paragraph
        let text = extract_all_text(&doc);
        assert!(
            text.contains("Shape text content"),
            "Should extract text from wps:wsp shape. Got: {}",
            text
        );
    }

    /// Test extracting text from grouped shapes
    /// NOTE: This should pass - txbxContent recursive parsing handles groups
    #[test]
    fn test_extract_group_shape_text() {
        // <wpg:wgp> contains multiple <wps:wsp> shapes
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"
                    xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:anchor>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup">
                                        <wpg:wgp>
                                            <wps:wsp>
                                                <wps:txbx>
                                                    <w:txbxContent>
                                                        <w:p><w:r><w:t>Box 1</w:t></w:r></w:p>
                                                    </w:txbxContent>
                                                </wps:txbx>
                                            </wps:wsp>
                                            <wps:wsp>
                                                <wps:txbx>
                                                    <w:txbxContent>
                                                        <w:p><w:r><w:t>Box 2</w:t></w:r></w:p>
                                                    </w:txbxContent>
                                                </wps:txbx>
                                            </wps:wsp>
                                            <wps:wsp>
                                                <wps:txbx>
                                                    <w:txbxContent>
                                                        <w:p><w:r><w:t>Box 3</w:t></w:r></w:p>
                                                    </w:txbxContent>
                                                </wps:txbx>
                                            </wps:wsp>
                                        </wpg:wgp>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:anchor>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        let text = extract_all_text(&doc);
        assert!(text.contains("Box 1"), "Should extract Box 1");
        assert!(text.contains("Box 2"), "Should extract Box 2");
        assert!(text.contains("Box 3"), "Should extract Box 3");
    }

    /// Test extracting text from SmartArt diagrams
    #[test]
    #[ignore = "TDD: Implement SmartArt text extraction"]
    fn test_extract_smartart_text() {
        // SmartArt is stored in separate diagrams/*.xml files
        // Document contains <dgm:relIds> reference
        // For now, test inline <a:t> elements

        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
                                        <dgm:relIds r:dm="rId1" r:lo="rId2" r:qs="rId3" r:cs="rId4"/>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        // Note: Full SmartArt extraction requires reading diagrams/data*.xml
        // This is a placeholder for the test structure
        let doc = Document::parse(xml).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    /// Test extracting text from chart titles and labels
    #[test]
    #[ignore = "TDD: Implement chart text extraction"]
    fn test_extract_chart_text() {
        // Charts reference external chart*.xml files
        // Text appears in <c:tx> and <c:txPr> elements

        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                                        <c:chart r:id="rId10"/>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        // Note: Chart text extraction requires parsing charts/chart*.xml
        let doc = Document::parse(xml).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    /// Helper to extract all text from a document
    fn extract_all_text(doc: &Document) -> String {
        let mut text = String::new();
        for block in &doc.blocks {
            if let Block::Paragraph(p) = block {
                for child in &p.children {
                    if let ParagraphChild::Run(run) = child {
                        text.push_str(&run.text);
                        text.push(' ');
                    }
                }
            }
        }
        text
    }
}

// =============================================================================
// PART 2: TOC HYPERLINK PRESERVATION
// =============================================================================

mod toc_hyperlink_preservation {
    //! Tests for preserving Table of Contents hyperlinks
    //!
    //! TOC links use w:anchor attribute pointing to _Toc... bookmarks
    //! These should map to AsciiDoc cross-references: <<section-title>>

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

    /// Test parsing TOC internal hyperlink with anchor
    #[test]
    #[ignore = "TDD: Implement TOC anchor link parsing"]
    fn test_parse_toc_anchor_link() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:hyperlink w:anchor="_Toc123456789">
                        <w:r><w:t>1. Introduction</w:t></w:r>
                    </w:hyperlink>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            let has_link = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Hyperlink(_)));
            assert!(has_link, "Should parse TOC hyperlink");

            if let Some(ParagraphChild::Hyperlink(link)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Hyperlink(_)))
            {
                assert_eq!(link.anchor, Some("_Toc123456789".to_string()));
                assert!(link.id.is_none(), "Internal links have no r:id");
            }
        }
    }

    /// Test converting TOC link to AsciiDoc cross-reference
    #[test]
    #[ignore = "TDD: Implement TOC to xref conversion"]
    fn test_convert_toc_to_xref() {
        // Given: DOCX with TOC entry
        // When: Extract to AsciiDoc
        // Then: Should produce <<section-anchor,Title>> or <<_toc...>>

        // This tests the extract.rs conversion logic
        // For now, just verify the hyperlink structure is correct
    }

    /// Test round-trip of internal cross-references
    #[test]
    #[ignore = "TDD: Implement cross-reference round-trip"]
    fn test_crossref_roundtrip() {
        // Given: <<section-anchor>>
        // When: Render to DOCX
        // Then: Should produce <w:hyperlink w:anchor="section-anchor">

        // Then: Extract back
        // Then: Should produce <<section-anchor>>
    }

    /// Test preserving _Ref style bookmarks (cross-references)
    #[test]
    #[ignore = "TDD: Implement _Ref bookmark preservation"]
    fn test_ref_bookmark_preservation() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="0" w:name="_Ref123456"/>
                    <w:r><w:t>Figure 1</w:t></w:r>
                    <w:bookmarkEnd w:id="0"/>
                </w:p>
                <w:p>
                    <w:hyperlink w:anchor="_Ref123456">
                        <w:r><w:t>See Figure 1</w:t></w:r>
                    </w:hyperlink>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should have bookmark in first paragraph
        if let Block::Paragraph(p) = &doc.blocks[0] {
            let has_bookmark = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Bookmark(_)));
            // Note: _Ref bookmarks are currently filtered out like _Toc
            // This test defines desired behavior to preserve them
            assert!(has_bookmark, "Should preserve _Ref bookmarks for references");
        }
    }
}

// =============================================================================
// PART 3: FIELD CODE HYPERLINKS
// =============================================================================

mod field_code_hyperlinks {
    //! Tests for handling HYPERLINK field codes
    //!
    //! Some hyperlinks appear as field codes:
    //! <w:fldChar w:fldCharType="begin"/>
    //! <w:instrText> HYPERLINK \l "bookmark" </w:instrText>
    //! <w:fldChar w:fldCharType="separate"/>
    //! <w:t>Link text</w:t>
    //! <w:fldChar w:fldCharType="end"/>

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

    /// Test parsing HYPERLINK field code
    #[test]
    #[ignore = "TDD: Implement field code hyperlink parsing"]
    fn test_parse_hyperlink_field_code() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:fldChar w:fldCharType="begin"/>
                    </w:r>
                    <w:r>
                        <w:instrText> HYPERLINK \l "_Ref123" </w:instrText>
                    </w:r>
                    <w:r>
                        <w:fldChar w:fldCharType="separate"/>
                    </w:r>
                    <w:r>
                        <w:t>See reference</w:t>
                    </w:r>
                    <w:r>
                        <w:fldChar w:fldCharType="end"/>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should be converted to hyperlink
        if let Block::Paragraph(p) = &doc.blocks[0] {
            let has_link = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Hyperlink(_)));
            assert!(
                has_link,
                "Field code HYPERLINK should be parsed as Hyperlink"
            );
        }
    }

    /// Test HYPERLINK with external URL
    #[test]
    #[ignore = "TDD: Implement external HYPERLINK field"]
    fn test_parse_external_hyperlink_field() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                    <w:r><w:instrText> HYPERLINK "https://example.com" </w:instrText></w:r>
                    <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                    <w:r><w:t>Example Site</w:t></w:r>
                    <w:r><w:fldChar w:fldCharType="end"/></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let Some(ParagraphChild::Hyperlink(link)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Hyperlink(_)))
            {
                // External links from field codes should have the URL
                assert!(link.id.is_some() || link.anchor.is_none());
            }
        }
    }
}

// =============================================================================
// PART 4: EQUATION/OMML CONTENT
// =============================================================================

mod equation_content {
    //! Tests for Office Math Markup Language (OMML) content
    //!
    //! Math equations appear as <m:oMath> or <m:oMathPara> blocks
    //! These should be preserved as LaTeX or MathML

    use utf8dok_ooxml::document::Document;

    /// Test parsing inline equation
    #[test]
    #[ignore = "TDD: Implement OMML parsing"]
    fn test_parse_inline_equation() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
            <w:body>
                <w:p>
                    <m:oMath>
                        <m:r>
                            <m:t>E</m:t>
                        </m:r>
                        <m:r>
                            <m:t>=</m:t>
                        </m:r>
                        <m:r>
                            <m:t>mc</m:t>
                        </m:r>
                        <m:sSup>
                            <m:e><m:r><m:t></m:t></m:r></m:e>
                            <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                        </m:sSup>
                    </m:oMath>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Should extract equation content
        // Could be as stem:[E=mc^2] or latexmath:[E=mc^2]
        assert!(!doc.blocks.is_empty());
    }

    /// Test parsing display equation (oMathPara)
    #[test]
    #[ignore = "TDD: Implement oMathPara parsing"]
    fn test_parse_display_equation() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
            <w:body>
                <w:p>
                    <m:oMathPara>
                        <m:oMath>
                            <m:r><m:t>x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}</m:t></m:r>
                        </m:oMath>
                    </m:oMathPara>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert!(!doc.blocks.is_empty());
    }
}

// =============================================================================
// PART 5: INTEGRATION / ROUND-TRIP TESTS
// =============================================================================

mod roundtrip_integration {
    //! Full round-trip tests for fidelity verification
    //!
    //! These tests verify the complete cycle:
    //! DOCX → extract → AsciiDoc → render → DOCX → extract → verify

    /// Test that all extracted text survives round-trip
    #[test]
    #[ignore = "TDD: Requires full extraction pipeline"]
    fn test_text_preservation_roundtrip() {
        // Given: A DOCX with various content types
        // When: Extract to AsciiDoc
        // And: Render back to DOCX
        // And: Extract again
        // Then: All text from original is present in final

        // This is a property-based test concept:
        // for all t in original_text_spans:
        //   assert(t in roundtrip_text_spans)
    }

    /// Test paragraph count fidelity >= 95%
    #[test]
    #[ignore = "TDD: Requires metric computation"]
    fn test_paragraph_fidelity_threshold() {
        // Given: Real document with known paragraph count
        // When: Round-trip
        // Then: Fidelity >= 0.95

        let original_para_count = 933;
        let threshold = 0.95;
        let min_acceptable = (original_para_count as f64 * threshold) as usize;

        // After round-trip:
        let roundtrip_para_count = 886; // Placeholder, should be computed
        assert!(
            roundtrip_para_count >= min_acceptable,
            "Paragraph fidelity {:.1}% below 95% threshold",
            (roundtrip_para_count as f64 / original_para_count as f64) * 100.0
        );
    }

    /// Test hyperlink fidelity >= 90%
    #[test]
    #[ignore = "TDD: Requires metric computation"]
    fn test_hyperlink_fidelity_threshold() {
        let original_link_count = 69;
        let threshold = 0.90;
        let min_acceptable = (original_link_count as f64 * threshold) as usize;

        let roundtrip_link_count = 62; // Placeholder
        assert!(
            roundtrip_link_count >= min_acceptable,
            "Hyperlink fidelity {:.1}% below 90% threshold",
            (roundtrip_link_count as f64 / original_link_count as f64) * 100.0
        );
    }

    /// Test media fidelity == 100%
    #[test]
    #[ignore = "TDD: Requires media comparison"]
    fn test_media_fidelity_lossless() {
        // Media (images) should be perfectly preserved
        // Compare by hash or binary equality

        let original_media_hashes: Vec<&str> = vec![
            "abc123...", // image1.png
            "def456...", // image2.jpg
        ];

        let roundtrip_media_hashes: Vec<&str> = vec![
            "abc123...", // same image1.png
            "def456...", // same image2.jpg
        ];

        assert_eq!(
            original_media_hashes, roundtrip_media_hashes,
            "All media must be byte-identical"
        );
    }
}

// =============================================================================
// PART 6: BOOKMARK ANCHOR TESTS (ALREADY IMPLEMENTED)
// =============================================================================

mod bookmark_anchor_tests {
    //! Verify existing bookmark support is complete

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

    /// Test user-defined bookmarks are extracted
    #[test]
    fn test_user_bookmark_extraction() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="0" w:name="custom_anchor"/>
                    <w:r><w:t>Anchored text</w:t></w:r>
                    <w:bookmarkEnd w:id="0"/>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            let has_bookmark = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Bookmark(b) if b.name == "custom_anchor"));
            assert!(has_bookmark, "Should extract user-defined bookmark");
        }
    }

    /// Test bookmark filtering: _Toc and _Ref are kept, _Hlk and other internal are filtered
    #[test]
    fn test_internal_bookmark_filtering() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="0" w:name="_Toc123456"/>
                    <w:r><w:t>TOC entry</w:t></w:r>
                    <w:bookmarkEnd w:id="0"/>
                </w:p>
                <w:p>
                    <w:bookmarkStart w:id="1" w:name="_Ref789012"/>
                    <w:r><w:t>Reference</w:t></w:r>
                    <w:bookmarkEnd w:id="1"/>
                </w:p>
                <w:p>
                    <w:bookmarkStart w:id="2" w:name="_Hlk345678"/>
                    <w:r><w:t>Highlighted</w:t></w:r>
                    <w:bookmarkEnd w:id="2"/>
                </w:p>
                <w:p>
                    <w:bookmarkStart w:id="3" w:name="_GoBack"/>
                    <w:r><w:t>GoBack</w:t></w:r>
                    <w:bookmarkEnd w:id="3"/>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        let mut found_toc = false;
        let mut found_ref = false;
        let mut found_hlk = false;
        let mut found_goback = false;

        for block in &doc.blocks {
            if let Block::Paragraph(p) = block {
                for child in &p.children {
                    if let ParagraphChild::Bookmark(b) = child {
                        match b.name.as_str() {
                            "_Toc123456" => found_toc = true,
                            "_Ref789012" => found_ref = true,
                            "_Hlk345678" => found_hlk = true,
                            "_GoBack" => found_goback = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        // _Toc and _Ref should be kept (semantically meaningful)
        assert!(found_toc, "_Toc bookmarks should be kept for anchor mapping");
        assert!(found_ref, "_Ref bookmarks should be kept for cross-references");

        // _Hlk and _GoBack should be filtered (truly internal)
        assert!(!found_hlk, "_Hlk bookmarks should be filtered out");
        assert!(!found_goback, "_GoBack bookmarks should be filtered out");
    }
}

// =============================================================================
// PART 7: TEXT BOX TESTS (ALREADY IMPLEMENTED)
// =============================================================================

// =============================================================================
// PART 8: STYLE CONTRACT TESTS (ADR-007)
// =============================================================================

mod style_contract_tests {
    //! Verify StyleContract extraction per ADR-007

    use utf8dok_ooxml::{AnchorType, StyleContract};

    /// Test that StyleContract is built during extraction
    #[test]
    fn test_style_contract_extraction() {
        // Note: XML fixtures defined here for future archive-based tests
        // For now, we test the StyleContract struct directly
        let mut contract = StyleContract::new();
        contract.add_paragraph_style(
            "Heading1",
            utf8dok_ooxml::ParagraphStyleMapping {
                role: "h1".into(),
                heading_level: Some(1),
                is_list: false,
                list_type: None,
                based_on: None,
            },
        );
        contract.add_anchor(
            "_Toc123456",
            utf8dok_ooxml::AnchorMapping {
                semantic_id: "introduction".into(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".into()),
                original_bookmark: Some("_Toc123456".into()),
            },
        );

        // Verify TOML serialization
        let toml = contract.to_toml().unwrap();
        assert!(toml.contains("[paragraph_styles.Heading1]"));
        assert!(toml.contains("role = \"h1\""));
        assert!(toml.contains("[anchors._Toc123456]"));
        assert!(toml.contains("semantic_id = \"introduction\""));
    }

    /// Test anchor normalization for TOC entries
    #[test]
    fn test_anchor_semantic_normalization() {
        use utf8dok_ooxml::style_map::{classify_bookmark, normalize_heading_to_anchor};

        // TOC bookmarks get semantic IDs from heading text
        assert_eq!(classify_bookmark("_Toc192197374"), AnchorType::Toc);
        assert_eq!(
            normalize_heading_to_anchor("1.2 Purpose and Scope"),
            "purpose-and-scope"
        );

        // Reference bookmarks keep ref prefix
        assert_eq!(classify_bookmark("_Ref123456"), AnchorType::Reference);

        // User bookmarks are normalized
        assert_eq!(classify_bookmark("custom_anchor"), AnchorType::UserDefined);
        assert_eq!(
            normalize_heading_to_anchor("Custom Anchor Name"),
            "custom-anchor-name"
        );
    }

    /// Test StyleContract round-trip via TOML
    #[test]
    fn test_style_contract_toml_roundtrip() {
        let mut original = StyleContract::with_source("test.docx");
        original.add_paragraph_style(
            "Heading2",
            utf8dok_ooxml::ParagraphStyleMapping {
                role: "h2".into(),
                heading_level: Some(2),
                is_list: false,
                list_type: None,
                based_on: Some("Heading1".into()),
            },
        );
        original.add_hyperlink(
            "link1",
            utf8dok_ooxml::HyperlinkMapping {
                is_external: true,
                url: Some("https://example.com".into()),
                anchor_target: None,
                original_rel_id: Some("rId5".into()),
                original_anchor: None,
            },
        );

        // Serialize and deserialize
        let toml = original.to_toml().unwrap();
        let parsed = StyleContract::from_toml(&toml).unwrap();

        // Verify contents
        assert_eq!(parsed.get_heading_level("Heading2"), Some(2));
        assert!(parsed.hyperlinks.contains_key("link1"));
        assert_eq!(
            parsed.hyperlinks.get("link1").unwrap().url,
            Some("https://example.com".into())
        );
    }
}

// =============================================================================
// PART 9: REAL-DOCUMENT CONFORMANCE TESTS
// =============================================================================

mod conformance_tests {
    //! Real-document conformance tests per StyleContract validation spec
    //!
    //! These tests verify StyleContract works with actual DOCX files.

    use std::path::Path;
    use utf8dok_ooxml::{AsciiDocExtractor, StyleContractValidator};

    const SWP_DOCX: &str = "../../corporate/SWP Application Architecture.docx";

    /// Test that SWP document extraction produces a valid StyleContract
    #[test]
    fn test_swp_style_contract_validation() {
        let path = Path::new(SWP_DOCX);
        if !path.exists() {
            eprintln!("Skipping test: {} not found", SWP_DOCX);
            return;
        }

        let extractor = AsciiDocExtractor::new().with_force_parse(true);
        let result = extractor.extract_file(path).unwrap();

        // Run full validation
        let validation = StyleContractValidator::validate(&result.style_contract);

        // Print any issues for debugging
        for issue in &validation.issues {
            eprintln!(
                "[{:?}] {:?}: {} (field: {:?})",
                issue.severity, issue.category, issue.message, issue.field
            );
        }

        // Must pass all validation (no errors)
        assert!(
            validation.is_valid(),
            "StyleContract validation failed with {} errors",
            validation.errors().len()
        );
    }

    /// Test that SWP document has expected paragraph style mappings
    #[test]
    fn test_swp_paragraph_styles_present() {
        let path = Path::new(SWP_DOCX);
        if !path.exists() {
            return;
        }

        let extractor = AsciiDocExtractor::new().with_force_parse(true);
        let result = extractor.extract_file(path).unwrap();
        let contract = &result.style_contract;

        // Should have heading styles
        let has_headings = contract
            .paragraph_styles
            .values()
            .any(|m| m.heading_level.is_some());
        assert!(has_headings, "Should extract heading style mappings");

        // Print extracted styles for debugging
        eprintln!("Extracted {} paragraph styles:", contract.paragraph_styles.len());
        for (id, mapping) in &contract.paragraph_styles {
            eprintln!("  {} -> role={}, level={:?}", id, mapping.role, mapping.heading_level);
        }
    }

    /// Test that SWP document has anchor mappings for TOC entries
    #[test]
    fn test_swp_anchor_mappings_present() {
        use utf8dok_ooxml::document::{Block, Document, ParagraphChild};
        use utf8dok_ooxml::OoxmlArchive;

        let path = Path::new(SWP_DOCX);
        if !path.exists() {
            return;
        }

        // First, let's see what bookmarks the document parser extracts
        let archive = OoxmlArchive::open(path).unwrap();
        let doc_xml = archive.document_xml().unwrap();
        let document = Document::parse(doc_xml).unwrap();

        let mut bookmark_count = 0;
        for block in &document.blocks {
            if let Block::Paragraph(para) = block {
                for child in &para.children {
                    if let ParagraphChild::Bookmark(bookmark) = child {
                        bookmark_count += 1;
                        if bookmark_count <= 10 {
                            eprintln!("Parsed bookmark: {}", bookmark.name);
                        }
                    }
                }
            }
        }
        eprintln!("Total bookmarks parsed from document: {}", bookmark_count);

        // Now check the extraction
        let extractor = AsciiDocExtractor::new().with_force_parse(true);
        let result = extractor.extract_file(path).unwrap();
        let contract = &result.style_contract;

        // Should have anchor mappings
        assert!(
            !contract.anchors.is_empty(),
            "Should extract anchor mappings (found 0)"
        );

        // Count TOC anchors
        let toc_anchors = contract
            .anchors
            .values()
            .filter(|m| matches!(m.anchor_type, utf8dok_ooxml::AnchorType::Toc))
            .count();

        eprintln!("Extracted {} total anchors, {} are TOC anchors",
            contract.anchors.len(), toc_anchors);

        // Print first 10 anchors
        for (bookmark, mapping) in contract.anchors.iter().take(10) {
            eprintln!(
                "  {} -> semantic_id={}, type={:?}",
                bookmark, mapping.semantic_id, mapping.anchor_type
            );
        }
    }

    /// Test that StyleContract TOML serialization round-trips correctly
    #[test]
    fn test_swp_style_contract_toml_roundtrip() {
        let path = Path::new(SWP_DOCX);
        if !path.exists() {
            return;
        }

        let extractor = AsciiDocExtractor::new().with_force_parse(true);
        let result = extractor.extract_file(path).unwrap();
        let contract = &result.style_contract;

        // Serialize to TOML
        let toml = contract.to_toml().expect("TOML serialization failed");

        // Deserialize back
        let roundtripped = utf8dok_ooxml::StyleContract::from_toml(&toml)
            .expect("TOML deserialization failed");

        // Verify counts match
        assert_eq!(
            contract.paragraph_styles.len(),
            roundtripped.paragraph_styles.len(),
            "Paragraph style count mismatch after TOML round-trip"
        );
        assert_eq!(
            contract.anchors.len(),
            roundtripped.anchors.len(),
            "Anchor count mismatch after TOML round-trip"
        );
        assert_eq!(
            contract.hyperlinks.len(),
            roundtripped.hyperlinks.len(),
            "Hyperlink count mismatch after TOML round-trip"
        );

        // Validate the round-tripped contract
        let validation = StyleContractValidator::validate(&roundtripped);
        assert!(
            validation.is_valid(),
            "Round-tripped StyleContract failed validation"
        );

        eprintln!("TOML round-trip successful: {} bytes", toml.len());
    }
}

mod textbox_tests {
    //! Verify existing text box support

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};

    /// Test text box content is extracted
    #[test]
    fn test_textbox_content_extraction() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:pict>
                            <v:shape xmlns:v="urn:schemas-microsoft-com:vml">
                                <v:textbox>
                                    <w:txbxContent>
                                        <w:p>
                                            <w:r><w:t>Text box content</w:t></w:r>
                                        </w:p>
                                    </w:txbxContent>
                                </v:textbox>
                            </v:shape>
                        </w:pict>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        // Text box content should be extracted as paragraphs
        let all_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph(p) = b {
                    Some(
                        p.children
                            .iter()
                            .filter_map(|c| {
                                if let ParagraphChild::Run(r) = c {
                                    Some(r.text.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            all_text.contains("Text box content"),
            "Should extract text box content. Got: {}",
            all_text
        );
    }
}

// =============================================================================
// PART 10: ESSENTIAL TEMPLATE TESTS (ADR-008)
// =============================================================================

mod essential_template_tests {
    //! Tests for the utf8dok Essential open-source template
    //!
    //! Verifies that the canonical style-contract.toml works with open_template.dotx

    use std::path::Path;

    const ESSENTIAL_TEMPLATE: &str = "../../templates/utf8dok-essential/open_template.dotx";
    const ESSENTIAL_CONTRACT: &str = "../../templates/utf8dok-essential/style-contract.toml";

    /// Test that the essential style-contract.toml can be parsed
    #[test]
    fn test_essential_style_contract_parses() {
        let path = Path::new(ESSENTIAL_CONTRACT);
        if !path.exists() {
            eprintln!("Skipping test: {} not found", ESSENTIAL_CONTRACT);
            return;
        }

        let toml_content = std::fs::read_to_string(path).expect("Failed to read style-contract.toml");

        // Try to parse - extra fields are ignored by serde default
        let contract = utf8dok_ooxml::StyleContract::from_toml(&toml_content);

        match contract {
            Ok(c) => {
                eprintln!("Successfully parsed style-contract.toml");
                eprintln!("  {} paragraph styles", c.paragraph_styles.len());
                eprintln!("  {} character styles", c.character_styles.len());
                eprintln!("  {} table styles", c.table_styles.len());

                // Verify key Italian styles are present
                assert!(
                    c.paragraph_styles.contains_key("Titolo1"),
                    "Should have Titolo1 style"
                );
                assert!(
                    c.paragraph_styles.contains_key("Normale"),
                    "Should have Normale style"
                );

                // Verify heading levels
                let titolo1 = c.paragraph_styles.get("Titolo1").unwrap();
                assert_eq!(titolo1.heading_level, Some(1), "Titolo1 should be level 1");
            }
            Err(e) => {
                panic!("Failed to parse style-contract.toml: {}", e);
            }
        }
    }

    /// Test that the essential template file exists and is valid
    #[test]
    fn test_essential_template_exists() {
        let path = Path::new(ESSENTIAL_TEMPLATE);
        if !path.exists() {
            eprintln!("Skipping test: {} not found", ESSENTIAL_TEMPLATE);
            return;
        }

        // Try to open as ZIP
        let file = std::fs::File::open(path).expect("Failed to open template");
        let archive = zip::ZipArchive::new(file).expect("Template is not a valid ZIP");

        // Verify key OOXML parts exist
        let mut has_document = false;
        let mut has_styles = false;
        let mut has_content_types = false;

        for i in 0..archive.len() {
            let name = archive.name_for_index(i).unwrap();
            if name == "word/document.xml" {
                has_document = true;
            }
            if name == "word/styles.xml" {
                has_styles = true;
            }
            if name == "[Content_Types].xml" {
                has_content_types = true;
            }
        }

        assert!(has_content_types, "Template should have [Content_Types].xml");
        assert!(has_document, "Template should have word/document.xml");
        assert!(has_styles, "Template should have word/styles.xml");

        eprintln!("Essential template is valid OOXML with {} entries", archive.len());
    }

    /// Test that essential template has expected Italian style IDs
    #[test]
    fn test_essential_template_style_ids() {
        let path = Path::new(ESSENTIAL_TEMPLATE);
        if !path.exists() {
            return;
        }

        let mut template = utf8dok_ooxml::Template::load(path).expect("Failed to load template");

        // Check for Italian heading styles
        let expected_styles = [
            "Titolo1", "Titolo2", "Titolo3", "Titolo4", "Titolo5",
            "Titolo6", "Titolo7", "Titolo8", "Titolo9", "Normale",
        ];

        let available = template.available_style_ids().expect("Failed to get style IDs");

        for style in &expected_styles {
            assert!(
                available.contains(&style.to_string()),
                "Template should have {} style. Available: {:?}",
                style,
                available.iter().take(20).collect::<Vec<_>>()
            );
        }

        eprintln!("Essential template has {} styles", available.len());
    }

    /// Test round-trip using essential template
    #[test]
    fn test_essential_template_roundtrip() {
        use utf8dok_ooxml::{DocxWriter, Template};

        let template_path = Path::new(ESSENTIAL_TEMPLATE);
        let contract_path = Path::new(ESSENTIAL_CONTRACT);

        if !template_path.exists() || !contract_path.exists() {
            eprintln!("Skipping round-trip test: template or contract not found");
            return;
        }

        // Load template
        let template = Template::load(template_path).expect("Failed to load template");

        // Load style contract
        let toml_content = std::fs::read_to_string(contract_path).unwrap();
        let contract = utf8dok_ooxml::StyleContract::from_toml(&toml_content).unwrap();

        // Create a simple AST using correct utf8dok_ast types
        use utf8dok_ast::{Block, Document, DocumentMeta, Heading, Inline, Paragraph};

        let doc = Document {
            metadata: DocumentMeta {
                title: Some("Test Document".into()),
                ..Default::default()
            },
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Introduction".into())],
                    anchor: Some("introduction".into()),
                    style_id: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("This is the introduction paragraph.".into())],
                    ..Default::default()
                }),
                Block::Heading(Heading {
                    level: 2,
                    text: vec![Inline::Text("Background".into())],
                    anchor: Some("background".into()),
                    style_id: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Some background information.".into())],
                    ..Default::default()
                }),
            ],
            intent: None,
        };

        // Render to DOCX
        let mut writer = DocxWriter::new();
        writer.set_style_contract(contract);
        let docx_bytes = writer
            .generate_with_template(&doc, template)
            .expect("Failed to generate DOCX");

        // Verify output is valid OOXML
        let cursor = std::io::Cursor::new(docx_bytes);
        let archive = zip::ZipArchive::new(cursor).expect("Output is not valid OOXML");

        // Check key parts exist
        let mut found_document = false;
        for i in 0..archive.len() {
            if archive.name_for_index(i).unwrap() == "word/document.xml" {
                found_document = true;
            }
        }

        assert!(found_document, "Output should have word/document.xml");
        eprintln!("Round-trip produced valid DOCX with {} entries", archive.len());
    }
}
