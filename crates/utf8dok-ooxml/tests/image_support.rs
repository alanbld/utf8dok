//! Image Support Tests
//!
//! Comprehensive TDD tests for OOXML image extraction and rendering.
//! These tests define the expected behavior before implementation.
//!
//! Test Categories:
//! 1. Image Parsing (document.rs) - Parse <w:drawing> elements
//! 2. Image Extraction (extract.rs) - Extract images and generate AsciiDoc
//! 3. Image Writing (writer.rs) - Render images to OOXML
//! 4. Round-Trip Tests - Full cycle preservation

// =============================================================================
// PART 1: IMAGE DATA STRUCTURE TESTS
// =============================================================================

mod image_struct_tests {
    //! Tests for the Image data structure itself

    use utf8dok_ooxml::image::{Image, ImagePosition, WrapType};

    #[test]
    fn test_image_struct_creation() {
        let image = Image {
            id: 1,
            rel_id: "rId11".to_string(),
            target: "media/image1.png".to_string(),
            alt: Some("Sample image".to_string()),
            name: Some("Image 1".to_string()),
            width_emu: Some(914400),  // 1 inch
            height_emu: Some(914400), // 1 inch
            position: ImagePosition::Inline,
        };

        assert_eq!(image.id, 1);
        assert_eq!(image.rel_id, "rId11");
        assert_eq!(image.target, "media/image1.png");
        assert_eq!(image.alt, Some("Sample image".to_string()));
    }

    #[test]
    fn test_image_inline_position() {
        let pos = ImagePosition::Inline;
        assert!(matches!(pos, ImagePosition::Inline));
    }

    #[test]
    fn test_image_anchor_position() {
        let pos = ImagePosition::Anchor {
            horizontal: 100000,
            vertical: 200000,
            wrap: WrapType::Square,
        };

        if let ImagePosition::Anchor {
            horizontal,
            vertical,
            wrap,
        } = pos
        {
            assert_eq!(horizontal, 100000);
            assert_eq!(vertical, 200000);
            assert!(matches!(wrap, WrapType::Square));
        } else {
            panic!("Expected Anchor position");
        }
    }

    #[test]
    fn test_emu_to_pixels_conversion() {
        use utf8dok_ooxml::image::emu_to_pixels;
        // 914400 EMUs = 1 inch = 96 pixels at 96 DPI
        assert_eq!(emu_to_pixels(914400), 96);
    }

    #[test]
    fn test_pixels_to_emu_conversion() {
        use utf8dok_ooxml::image::pixels_to_emu;
        // 96 pixels = 914400 EMUs
        assert_eq!(pixels_to_emu(96), 914400);
    }

    #[test]
    fn test_image_filename_extraction() {
        let img = Image::new_inline(1, "rId1".to_string(), "media/image1.png".to_string());
        assert_eq!(img.filename(), "image1.png");
    }

    #[test]
    fn test_image_extension_extraction() {
        let img = Image::new_inline(1, "rId1".to_string(), "media/image1.png".to_string());
        assert_eq!(img.extension(), Some("png"));
    }

    #[test]
    fn test_image_dimensions_helper() {
        let img = Image::new_inline(1, "rId1".to_string(), "media/test.png".to_string())
            .with_dimensions_px(200, 150);

        assert_eq!(img.width_px(), Some(200));
        assert_eq!(img.height_px(), Some(150));
    }

    #[test]
    fn test_content_type_mapping() {
        use utf8dok_ooxml::image::content_type_for_extension;

        assert_eq!(content_type_for_extension("png"), "image/png");
        assert_eq!(content_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(content_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(content_type_for_extension("svg"), "image/svg+xml");
        assert_eq!(content_type_for_extension("emf"), "image/x-emf");
    }

    #[test]
    fn test_wrap_type_parsing() {
        assert_eq!(WrapType::from_element_name("wrapSquare"), WrapType::Square);
        assert_eq!(WrapType::from_element_name("wrapTight"), WrapType::Tight);
        assert_eq!(
            WrapType::from_element_name("wrapTopAndBottom"),
            WrapType::TopAndBottom
        );
        assert_eq!(WrapType::from_element_name("unknown"), WrapType::None);
    }
}

// =============================================================================
// PART 2: DOCUMENT PARSING TESTS - Parse <w:drawing> elements
// =============================================================================

mod document_parsing_tests {
    //! Tests for parsing images from document.xml

    use utf8dok_ooxml::document::{Block, Document, ParagraphChild};
    use utf8dok_ooxml::image::ImagePosition;

    /// Test parsing a simple inline image
    #[test]
    fn test_parse_inline_image() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="914400" cy="914400"/>
                                <wp:docPr id="1" name="Image 1" descr="Test image"/>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                        <pic:pic>
                                            <pic:blipFill>
                                                <a:blip r:embed="rId4"/>
                                            </pic:blipFill>
                                        </pic:pic>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();
        assert_eq!(doc.blocks.len(), 1);

        if let Block::Paragraph(p) = &doc.blocks[0] {
            assert!(!p.children.is_empty(), "Paragraph should have children");
            // After implementation, should have an Image child
            let has_image = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Image(_)));
            assert!(has_image, "Paragraph should contain an image");
        } else {
            panic!("Expected Paragraph");
        }
    }

    /// Test parsing an anchored (floating) image
    #[test]
    fn test_parse_anchored_image() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:anchor distT="0" distB="0" distL="114300" distR="114300">
                                <wp:positionH relativeFrom="column">
                                    <wp:posOffset>0</wp:posOffset>
                                </wp:positionH>
                                <wp:positionV relativeFrom="paragraph">
                                    <wp:posOffset>0</wp:posOffset>
                                </wp:positionV>
                                <wp:extent cx="1828800" cy="1371600"/>
                                <wp:wrapSquare wrapText="bothSides"/>
                                <wp:docPr id="2" name="Image 2" descr="Anchored image"/>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                        <pic:pic>
                                            <pic:blipFill>
                                                <a:blip r:embed="rId5"/>
                                            </pic:blipFill>
                                        </pic:pic>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:anchor>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            let has_image = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Image(_)));
            assert!(has_image, "Paragraph should contain an anchored image");

            // Verify anchor properties after implementation
            if let Some(ParagraphChild::Image(img)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Image(_)))
            {
                assert!(matches!(img.position, ImagePosition::Anchor { .. }));
            }
        }
    }

    /// Test extracting relationship ID from blip
    #[test]
    fn test_extract_relationship_id() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="914400" cy="914400"/>
                                <wp:docPr id="1" name="Image 1"/>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                        <pic:pic>
                                            <pic:blipFill>
                                                <a:blip r:embed="rId42"/>
                                            </pic:blipFill>
                                        </pic:pic>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let Some(ParagraphChild::Image(img)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Image(_)))
            {
                assert_eq!(img.rel_id, "rId42");
            }
        }
    }

    /// Test extracting dimensions from extent
    #[test]
    fn test_extract_image_dimensions() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="1828800" cy="1371600"/>
                                <wp:docPr id="1" name="Image 1"/>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                        <pic:pic>
                                            <pic:blipFill>
                                                <a:blip r:embed="rId4"/>
                                            </pic:blipFill>
                                        </pic:pic>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let Some(ParagraphChild::Image(img)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Image(_)))
            {
                assert_eq!(img.width_emu, Some(1828800)); // 2 inches
                assert_eq!(img.height_emu, Some(1371600)); // 1.5 inches
            }
        }
    }

    /// Test extracting alt text from docPr
    #[test]
    fn test_extract_alt_text() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="914400" cy="914400"/>
                                <wp:docPr id="1" name="Company Logo" descr="Engineering company logo"/>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                        <pic:pic>
                                            <pic:blipFill>
                                                <a:blip r:embed="rId4"/>
                                            </pic:blipFill>
                                        </pic:pic>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let Some(ParagraphChild::Image(img)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Image(_)))
            {
                assert_eq!(img.alt, Some("Engineering company logo".to_string()));
            }
        }
    }

    /// Test parsing multiple images in one document
    #[test]
    fn test_parse_multiple_images() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="914400" cy="914400"/>
                                <wp:docPr id="1" name="Image 1"/>
                                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:pic><pic:blipFill><a:blip r:embed="rId4"/></pic:blipFill></pic:pic>
                                </a:graphicData></a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="1828800" cy="1371600"/>
                                <wp:docPr id="2" name="Image 2"/>
                                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:pic><pic:blipFill><a:blip r:embed="rId5"/></pic:blipFill></pic:pic>
                                </a:graphicData></a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="2743200" cy="2057400"/>
                                <wp:docPr id="3" name="Image 3"/>
                                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:pic><pic:blipFill><a:blip r:embed="rId6"/></pic:blipFill></pic:pic>
                                </a:graphicData></a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        let image_count = doc
            .blocks
            .iter()
            .filter(|b| {
                if let Block::Paragraph(p) = b {
                    p.children
                        .iter()
                        .any(|c| matches!(c, ParagraphChild::Image(_)))
                } else {
                    false
                }
            })
            .count();

        assert_eq!(image_count, 3, "Should parse all 3 images");
    }

    /// Test parsing image in a table cell
    #[test]
    fn test_parse_image_in_table() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:tbl>
                    <w:tr>
                        <w:tc>
                            <w:p>
                                <w:r>
                                    <w:drawing>
                                        <wp:inline>
                                            <wp:extent cx="914400" cy="914400"/>
                                            <wp:docPr id="1" name="Table Image"/>
                                            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                                <pic:pic><pic:blipFill><a:blip r:embed="rId4"/></pic:blipFill></pic:pic>
                                            </a:graphicData></a:graphic>
                                        </wp:inline>
                                    </w:drawing>
                                </w:r>
                            </w:p>
                        </w:tc>
                    </w:tr>
                </w:tbl>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Table(table) = &doc.blocks[0] {
            let cell_para = &table.rows[0].cells[0].paragraphs[0];
            let has_image = cell_para
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Image(_)));
            assert!(has_image, "Table cell should contain an image");
        } else {
            panic!("Expected Table");
        }
    }

    /// Test handling image without dimensions
    #[test]
    fn test_parse_image_without_dimensions() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:docPr id="1" name="Image 1"/>
                                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:pic><pic:blipFill><a:blip r:embed="rId4"/></pic:blipFill></pic:pic>
                                </a:graphicData></a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            if let Some(ParagraphChild::Image(img)) = p
                .children
                .iter()
                .find(|c| matches!(c, ParagraphChild::Image(_)))
            {
                assert!(img.width_emu.is_none());
                assert!(img.height_emu.is_none());
            }
        }
    }

    /// Test image mixed with text in same paragraph
    #[test]
    fn test_parse_image_with_text() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <w:body>
                <w:p>
                    <w:r><w:t>Before image: </w:t></w:r>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <wp:extent cx="914400" cy="914400"/>
                                <wp:docPr id="1" name="Inline Image"/>
                                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:pic><pic:blipFill><a:blip r:embed="rId4"/></pic:blipFill></pic:pic>
                                </a:graphicData></a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                    <w:r><w:t> After image.</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let doc = Document::parse(xml).unwrap();

        if let Block::Paragraph(p) = &doc.blocks[0] {
            assert!(p.children.len() >= 3, "Should have text + image + text");

            let has_text = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Run(_)));
            let has_image = p
                .children
                .iter()
                .any(|c| matches!(c, ParagraphChild::Image(_)));

            assert!(has_text, "Should have text runs");
            assert!(has_image, "Should have image");
        }
    }
}

// =============================================================================
// PART 3: RELATIONSHIP HANDLING TESTS
// =============================================================================

mod relationship_tests {
    //! Tests for image relationship management

    use utf8dok_ooxml::relationships::Relationships;

    /// Test adding image relationship
    #[test]
    fn test_add_image_relationship() {
        let mut rels = Relationships::new();
        let id = rels.add_image("media/image1.png");

        assert!(id.starts_with("rId"));
        assert!(rels.get(&id).is_some());
    }

    /// Test multiple image relationships have unique IDs
    #[test]
    fn test_unique_relationship_ids() {
        let mut rels = Relationships::new();
        let id1 = rels.add_image("media/image1.png");
        let id2 = rels.add_image("media/image2.png");
        let id3 = rels.add_image("media/image3.png");

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    /// Test relationship serialization includes image type
    #[test]
    fn test_relationship_xml_image_type() {
        let mut rels = Relationships::new();
        rels.add_image("media/test.png");

        let xml = rels.to_xml();
        assert!(
            xml.contains("relationships/image"),
            "Should specify image type"
        );
    }

    /// Test parsing existing image relationships
    #[test]
    fn test_parse_image_relationships() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
            <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.jpg"/>
        </Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();

        assert_eq!(
            rels.get("rId1"),
            Some("media/image1.png".to_string()).as_deref()
        );
        assert_eq!(
            rels.get("rId2"),
            Some("media/image2.jpg".to_string()).as_deref()
        );
    }
}

// =============================================================================
// PART 4: CONTENT TYPE TESTS
// =============================================================================

mod content_type_tests {
    //! Tests for Content_Types.xml image handling

    use utf8dok_ooxml::image::content_type_for_extension;

    /// Test PNG content type
    #[test]
    fn test_png_content_type() {
        assert_eq!(content_type_for_extension("png"), "image/png");
    }

    /// Test JPEG content type
    #[test]
    fn test_jpeg_content_type() {
        assert_eq!(content_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(content_type_for_extension("jpg"), "image/jpeg");
    }

    /// Test SVG content type
    #[test]
    fn test_svg_content_type() {
        assert_eq!(content_type_for_extension("svg"), "image/svg+xml");
    }

    /// Test EMF content type
    #[test]
    fn test_emf_content_type() {
        assert_eq!(content_type_for_extension("emf"), "image/x-emf");
    }

    /// Test GIF content type
    #[test]
    fn test_gif_content_type() {
        assert_eq!(content_type_for_extension("gif"), "image/gif");
    }
}
