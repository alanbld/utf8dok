# ADR-005: OOXML Image Support for Round-Trip Fidelity

**Status**: Accepted
**Date**: 2025-12-30
**Deciders**: Development Team
**Technical Story**: Enable rich document round-trip by supporting embedded images

## Context and Problem Statement

The current utf8dok DOCX extraction and rendering focuses on text content, leaving embedded images unsupported. Analysis of corporate documents shows:

- 37 `<w:drawing>` elements lost during round-trip
- 40 image files present but unreferenced in regenerated documents
- Visual fidelity significantly degraded for image-heavy documents

This limits utf8dok's usefulness for enterprise documents containing diagrams, screenshots, and visual content.

## Decision Drivers

* Corporate documents typically contain 20-50 embedded images
* Images are critical for technical documentation (diagrams, screenshots)
* Round-trip must preserve visual elements, not just text
* AsciiDoc has native image support via `image::` macro

## Considered Options

### Option 1: Full Drawing Element Preservation
Preserve complete `<w:drawing>` XML and re-inject on render.

**Pros**: Perfect fidelity
**Cons**: Requires storing raw XML, complex relationship management

### Option 2: Image Reference Extraction (Chosen)
Extract images as files, generate AsciiDoc `image::` macros, re-render as new drawings.

**Pros**: Clean round-trip, editable AsciiDoc, standard approach
**Cons**: May lose some positioning metadata

### Option 3: Image Placeholder Only
Mark image positions without preserving actual images.

**Pros**: Simple implementation
**Cons**: Loses actual image content

## Decision Outcome

**Chosen option**: Option 2 - Image Reference Extraction

This approach:
1. Extracts embedded images to `media/` folder during extraction
2. Generates `image::media/filename.ext[alt text, width, height]` in AsciiDoc
3. Re-embeds images during rendering with proper OOXML structure

## Technical Specification

### OOXML Image Structure

```xml
<w:drawing>
  <wp:inline|wp:anchor>
    <wp:extent cx="..." cy="..."/>           <!-- Dimensions in EMUs -->
    <wp:docPr id="..." name="..." descr="..."/>  <!-- Alt text -->
    <a:graphic>
      <a:graphicData uri="...picture">
        <pic:pic>
          <pic:blipFill>
            <a:blip r:embed="rIdNN"/>        <!-- Relationship ID -->
          </pic:blipFill>
        </pic:pic>
      </a:graphicData>
    </a:graphic>
  </wp:inline|wp:anchor>
</w:drawing>
```

### Relationship Structure

```xml
<!-- word/_rels/document.xml.rels -->
<Relationship
  Id="rId11"
  Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
  Target="media/image1.jpeg"/>
```

### Content Types

```xml
<!-- [Content_Types].xml -->
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Default Extension="svg" ContentType="image/svg+xml"/>
<Default Extension="emf" ContentType="image/x-emf"/>
```

### Extraction Algorithm

```
1. Parse document.xml for <w:drawing> elements
2. For each drawing:
   a. Extract relationship ID from <a:blip r:embed="..."/>
   b. Look up target in document.xml.rels
   c. Copy image from word/media/ to output/media/
   d. Extract dimensions from <wp:extent>
   e. Extract alt text from <wp:docPr descr="...">
   f. Generate: image::media/filename.ext[alt, width, height]
3. Track image positions relative to paragraphs
```

### Rendering Algorithm

```
1. Parse AsciiDoc for image:: macros
2. For each image:
   a. Copy source file to word/media/
   b. Generate relationship ID
   c. Add relationship to document.xml.rels
   d. Add content type if new extension
   e. Generate <w:drawing> element with:
      - Inline positioning (default)
      - Proper dimensions
      - Alt text as docPr
```

### Unit Conversion

- OOXML uses EMUs (English Metric Units)
- 914400 EMUs = 1 inch
- 635 EMUs = 1 pixel (at 96 DPI)
- AsciiDoc uses pixels or percentages

## Data Structures

### Image struct (Rust)

```rust
/// Represents an embedded image
#[derive(Debug, Clone)]
pub struct Image {
    /// Unique identifier
    pub id: u32,
    /// Relationship ID (e.g., "rId11")
    pub rel_id: String,
    /// Target path (e.g., "media/image1.png")
    pub target: String,
    /// Alt text / description
    pub alt: Option<String>,
    /// Width in EMUs
    pub width_emu: Option<i64>,
    /// Height in EMUs
    pub height_emu: Option<i64>,
    /// Position type
    pub position: ImagePosition,
}

#[derive(Debug, Clone)]
pub enum ImagePosition {
    /// Flows with text
    Inline,
    /// Floating, anchored to paragraph
    Anchor {
        horizontal: i64,
        vertical: i64,
        wrap: WrapType,
    },
}

#[derive(Debug, Clone)]
pub enum WrapType {
    None,
    Square,
    Tight,
    Through,
    TopAndBottom,
}
```

## Test Cases

### Extraction Tests
1. Parse inline image with relationship
2. Parse anchored image with positioning
3. Handle multiple images in document
4. Handle images in different formats (PNG, JPEG, SVG, EMF)
5. Extract image dimensions correctly
6. Extract alt text from docPr
7. Handle missing image files gracefully
8. Handle images in tables
9. Handle images with no dimensions specified
10. Generate correct AsciiDoc image macro syntax

### Writer Tests
1. Write inline image with proper OOXML structure
2. Create relationship for new image
3. Copy image file to media folder
4. Update Content_Types.xml for new extensions
5. Handle multiple images with unique IDs
6. Convert pixel dimensions to EMUs
7. Generate proper namespace declarations
8. Handle images with alt text
9. Handle images without explicit dimensions
10. Handle various image formats

### Round-Trip Tests
1. Image file content preserved exactly
2. Multiple images maintain order
3. Alt text survives round-trip
4. Dimensions approximately preserved
5. Different formats all work (PNG, JPEG, SVG)
6. Images in tables survive round-trip
7. Large images handled correctly
8. Images with special characters in names

## Consequences

### Positive
* Rich documents can be round-tripped with images
* Corporate documents become fully supported
* AsciiDoc source includes proper image references
* Visual fidelity significantly improved

### Negative
* Increased complexity in extraction and writing
* Larger extracted project size (media files)
* Some positioning metadata may be simplified

### Neutral
* Need to handle various image formats
* Relationship management adds bookkeeping

## Implementation Plan

1. Add `Image` struct to document model
2. Implement image parsing in `document.rs`
3. Add image extraction to `extract.rs`
4. Implement image writing in `writer.rs`
5. Update relationship handling
6. Add comprehensive tests
7. Test with real corporate documents

## References

* [ECMA-376 Office Open XML](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/)
* [DrawingML Reference](http://officeopenxml.com/drwOverview.php)
* [AsciiDoc Image Macro](https://docs.asciidoctor.org/asciidoc/latest/macros/images/)
