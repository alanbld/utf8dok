# utf8dok PPTX Generation Specification

**Version:** 1.0.0
**Status:** Draft
**Date:** 2025-12-31
**Related ADR:** ADR-010

## 1. Overview

This specification defines how utf8dok generates PowerPoint presentations (PPTX) from AsciiDoc sources. The design supports two modes: dedicated presentation documents and "dual-nature" documents that produce both DOCX documentation and PPTX slides from a single source.

## 2. Terminology

| Term | Definition |
|------|------------|
| **Slide** | A single presentation page containing content |
| **Slide Deck** | Collection of slides forming a presentation |
| **Slide Layout** | Template defining placeholder positions (Title, Content, Two Column, etc.) |
| **Slide Master** | Master formatting applied to all slides |
| **Speaker Notes** | Hidden notes for presenter, not shown on slide |
| **Dual-Nature Document** | AsciiDoc source that generates both DOCX and PPTX |
| **SlideContract** | TOML configuration mapping semantic types to PPTX layouts |
| **POTX** | PowerPoint Template file format |

## 3. Document Modes

### 3.1 Presentation Mode (`:doctype: slides`)

The entire document is a presentation:

```asciidoc
= Quarterly Review
:doctype: slides
:slides:
Author Name
v1.0, 2025-12-31

== Introduction
Opening remarks for the presentation

== Key Metrics
* Revenue: +15%
* Users: 1.2M
* Satisfaction: 94%

[.notes]
--
Emphasize the user growth trend
--
```

### 3.2 Dual-Nature Mode (`[slides]` block)

Embedded presentations within documentation:

```asciidoc
= Technical Specification
:doctype: book

== Introduction
Full documentation content here...

[slides]
--
== Executive Summary
Key points for stakeholders

== Architecture Overview
image::diagrams/architecture.png[]

== Timeline
Project milestones
--

== Detailed Requirements
More documentation...

[slides,title="Deep Dive"]
--
== API Design
Technical implementation details
--
```

**Semantics:**
- Content inside `[slides]` blocks → PPTX output
- Content outside `[slides]` blocks → DOCX only
- Multiple `[slides]` blocks → Concatenated into single PPTX
- `title` attribute → Overrides presentation title

## 4. Slide Boundary Syntax

### 4.1 Heading-Based Boundaries (Reveal.js Compatible)

| Syntax | Slide Type | Description |
|--------|------------|-------------|
| `= Title` | Title Slide | Presentation title (centered, large) |
| `== Heading` | Content Slide | New slide with title |
| `=== Subheading` | Vertical Slide | Sub-slide or nested section |
| `---` | Blank Break | Slide break without title |

### 4.2 Examples

```asciidoc
= Company Overview
:slides:

== About Us
Founded in 2020 with a mission to simplify document workflows.

=== Our Team
* Engineering: 15 people
* Design: 5 people
* Support: 10 people

---

image::team-photo.png[Team photo,width=100%]

== Products
Our product lineup includes...
```

**Generated Slides:**
1. Title Slide: "Company Overview"
2. Content Slide: "About Us" with body text
3. Vertical/Sub Slide: "Our Team" with bullets
4. Image-Only Slide (no title)
5. Content Slide: "Products" with body text

## 5. Content Mapping

### 5.1 Block Element Mapping

| AsciiDoc Element | Slide Rendering | Layout Hint |
|------------------|-----------------|-------------|
| `= Title` | Title slide (centered, large) | `title` |
| `== Heading` | Slide title in title placeholder | `content` |
| `=== Subheading` | Subtitle or vertical slide | `section` |
| Paragraph | Body text in content placeholder | - |
| Unordered list (`*`) | Bullet points | - |
| Ordered list (`.`) | Numbered points | - |
| Image (`image::`) | Media placeholder (scaled) | `image` |
| Table | Table shape | `content` |
| Code block | Monospace code box | `content` |
| Admonition | Styled callout box | - |
| `[.notes]` block | Speaker notes pane | - |

### 5.2 Inline Element Mapping

| AsciiDoc | Slide Rendering |
|----------|-----------------|
| `*bold*` | Bold text run |
| `_italic_` | Italic text run |
| `+monospace+` | Monospace font |
| `link:url[text]` | Hyperlink |
| `image:inline.png[]` | Inline image |

### 5.3 Special Slide Types

#### Two-Column Layout

```asciidoc
== Comparison

[cols="2"]
|===
a|
=== Option A
* Fast
* Simple
* Limited

a|
=== Option B
* Flexible
* Complex
* Powerful
|===
```

#### Quote Slide

```asciidoc
== Inspiration

[quote,Steve Jobs]
____
Design is not just what it looks like. Design is how it works.
____
```

#### Image-Only Slide

```asciidoc
---

image::hero-image.png[,width=100%,height=100%]

---
```

## 6. Speaker Notes

### 6.1 Syntax

```asciidoc
== Key Findings

Our research revealed three critical insights.

[.notes]
--
* Mention the methodology used
* Emphasize the sample size (n=1000)
* This slide should take about 2 minutes
--
```

### 6.2 Alternative Syntax (Inline Role)

```asciidoc
== Budget Overview

[.notes]
Remember to discuss contingency fund.

* Q1: $1.2M
* Q2: $1.5M
```

## 7. SlideContract Configuration

### 7.1 Schema

```toml
# slide-contract.toml

[meta]
template = "corporate.potx"
template_name = "Corporate Presentation"
locale = "it-IT"
version = "1.0.0"
description = "Official corporate presentation template"

# =============================================================================
# LAYOUT MAPPINGS
# =============================================================================
# Map semantic slide types to template slideLayout indices
# Layout indices are 1-based, corresponding to slideLayout1.xml, etc.

[layouts]
title = 1           # Title Slide layout
content = 2         # Title and Content layout
section = 3         # Section Header layout
two_column = 4      # Two Content layout
comparison = 5      # Comparison layout
title_only = 6      # Title Only layout
blank = 7           # Blank layout
image = 8           # Picture with Caption layout
quote = 9           # Quote layout

# =============================================================================
# PLACEHOLDER MAPPINGS
# =============================================================================
# Map content types to placeholder indices within layouts
# Placeholder indices correspond to <p:ph idx="N"/> in slideLayout XML

[placeholders]
title = 0           # Title placeholder (usually idx=0)
subtitle = 1        # Subtitle placeholder
body = 2            # Body/content placeholder
footer = 10         # Footer placeholder
slide_number = 11   # Slide number placeholder
date = 12           # Date placeholder

# =============================================================================
# SPEAKER NOTES
# =============================================================================

[notes]
enabled = true
font_size = 24          # Half-points (12pt)
font_family = "Arial"
line_spacing = 1.15     # Line height multiplier

# =============================================================================
# CONTENT DEFAULTS
# =============================================================================
# Default layout selection for different content patterns

[defaults]
heading_slide = "content"      # Default for == Heading
bullet_slide = "content"       # Default for bullet lists
image_slide = "image"          # Default for full-image slides
table_slide = "content"        # Default for tables
code_slide = "content"         # Default for code blocks
quote_slide = "quote"          # Default for block quotes
section_break = "section"      # Default for === Subheading

# =============================================================================
# SLIDE TRANSITIONS (Optional)
# =============================================================================

[transitions]
default = "fade"               # none, fade, push, wipe, split
duration = 500                 # Milliseconds

# =============================================================================
# CODE BLOCK STYLING
# =============================================================================

[code]
font_family = "JetBrains Mono"
font_size = 20                 # Half-points (10pt)
background_color = "1E1E1E"    # Dark background
text_color = "D4D4D4"          # Light text
border = true
border_color = "3C3C3C"
padding = "10pt"

# =============================================================================
# TABLE STYLING
# =============================================================================

[table]
header_background = "2563EB"   # Blue header
header_text_color = "FFFFFF"   # White text
row_background = "FFFFFF"      # White rows
alt_row_background = "F3F4F6"  # Alternating gray
border_color = "D1D5DB"        # Gray border
font_size = 20                 # Half-points (10pt)
```

### 7.2 Layout Discovery

To discover layout indices from a POTX template:

```bash
# List layouts in template
utf8dok inspect template.potx --layouts

# Output:
# Layout 1: Title Slide (slideLayout1.xml)
# Layout 2: Title and Content (slideLayout2.xml)
# Layout 3: Section Header (slideLayout3.xml)
# ...
```

## 8. OOXML Implementation

### 8.1 PPTX Package Structure

```
presentation.pptx (ZIP)
├── [Content_Types].xml
├── _rels/
│   └── .rels
├── ppt/
│   ├── presentation.xml          # Root document
│   ├── presProps.xml            # Presentation properties
│   ├── tableStyles.xml          # Table styles
│   ├── viewProps.xml            # View properties
│   ├── slides/
│   │   ├── slide1.xml           # Individual slides
│   │   ├── slide2.xml
│   │   └── _rels/
│   │       ├── slide1.xml.rels  # Slide relationships
│   │       └── slide2.xml.rels
│   ├── slideLayouts/
│   │   ├── slideLayout1.xml     # Title Slide
│   │   ├── slideLayout2.xml     # Title and Content
│   │   └── _rels/
│   ├── slideMasters/
│   │   ├── slideMaster1.xml     # Master formatting
│   │   └── _rels/
│   ├── theme/
│   │   └── theme1.xml           # Color/font schemes
│   ├── notesMasters/            # Notes master (if notes exist)
│   │   └── notesMaster1.xml
│   ├── notesSlides/             # Per-slide speaker notes
│   │   ├── notesSlide1.xml
│   │   └── _rels/
│   └── media/                   # Embedded images
│       ├── image1.png
│       └── image2.jpg
└── docProps/
    ├── app.xml                  # Application properties
    └── core.xml                 # Dublin Core metadata
```

### 8.2 Slide XML Structure

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>

      <!-- Title Shape -->
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Title 1"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="title"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US"/>
              <a:t>Slide Title</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>

      <!-- Content Shape -->
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Content Placeholder 2"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:pPr lvl="0"/>
            <a:r>
              <a:rPr lang="en-US"/>
              <a:t>First bullet point</a:t>
            </a:r>
          </a:p>
          <a:p>
            <a:pPr lvl="0"/>
            <a:r>
              <a:rPr lang="en-US"/>
              <a:t>Second bullet point</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>
```

### 8.3 Speaker Notes XML

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>...</p:nvGrpSpPr>
      <p:grpSpPr/>

      <!-- Slide thumbnail placeholder -->
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Slide Image"/>
          <p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="sldImg"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
      </p:sp>

      <!-- Notes text -->
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="3" name="Notes Placeholder"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US"/>
              <a:t>Speaker notes content here...</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:notes>
```

### 8.4 Bullet List Levels

```xml
<p:txBody>
  <a:bodyPr/>
  <a:lstStyle/>
  <!-- Level 0 (first level) -->
  <a:p>
    <a:pPr lvl="0"/>
    <a:r><a:t>Top level item</a:t></a:r>
  </a:p>
  <!-- Level 1 (nested) -->
  <a:p>
    <a:pPr lvl="1"/>
    <a:r><a:t>Nested item</a:t></a:r>
  </a:p>
  <!-- Level 2 (deeply nested) -->
  <a:p>
    <a:pPr lvl="2"/>
    <a:r><a:t>Deeply nested item</a:t></a:r>
  </a:p>
</p:txBody>
```

### 8.5 Image Embedding

```xml
<!-- In slide XML -->
<p:pic>
  <p:nvPicPr>
    <p:cNvPr id="4" name="Picture 1"/>
    <p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr>
    <p:nvPr/>
  </p:nvPicPr>
  <p:blipFill>
    <a:blip r:embed="rId2"/>
    <a:stretch><a:fillRect/></a:stretch>
  </p:blipFill>
  <p:spPr>
    <a:xfrm>
      <a:off x="1524000" y="1397000"/>  <!-- Position (EMU) -->
      <a:ext cx="6096000" cy="4572000"/> <!-- Size (EMU) -->
    </a:xfrm>
    <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
  </p:spPr>
</p:pic>

<!-- In slide relationships -->
<Relationship Id="rId2"
  Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
  Target="../media/image1.png"/>
```

## 9. Template Injection Process

### 9.1 Workflow

```
1. Load POTX template
   └── Extract to temporary directory

2. Parse AsciiDoc source
   └── Build slide structure from AST

3. Map content to layouts
   └── Use SlideContract for layout indices

4. Generate slide XML
   └── Inject content into placeholder positions

5. Update relationships
   └── Add slides to presentation.xml
   └── Add media references

6. Package PPTX
   └── Rezip with updated content
```

### 9.2 Template Requirements

A valid POTX template must have:

- `ppt/slideMasters/slideMaster1.xml` - At least one slide master
- `ppt/slideLayouts/slideLayout1.xml` - At least Title Slide layout
- `ppt/theme/theme1.xml` - Theme definition
- `ppt/presentation.xml` - Presentation root

## 10. CLI Reference

### 10.1 Basic Commands

```bash
# Generate PPTX from presentation document
utf8dok render presentation.adoc --format pptx --output slides.pptx

# Generate PPTX from dual-nature document (extract [slides] blocks)
utf8dok render document.adoc --format pptx --output slides.pptx

# Generate both DOCX and PPTX
utf8dok render document.adoc --output document.docx --slides slides.pptx

# Use custom template
utf8dok render document.adoc --format pptx \
  --template corporate.potx \
  --contract slide-contract.toml \
  --output presentation.pptx
```

### 10.2 Template Inspection

```bash
# List slide layouts in template
utf8dok inspect template.potx --layouts

# Show slide master structure
utf8dok inspect template.potx --masters

# Extract theme colors
utf8dok inspect template.potx --theme
```

### 10.3 Validation

```bash
# Validate SlideContract against template
utf8dok validate slide-contract.toml --template corporate.potx

# Check AsciiDoc slide syntax
utf8dok validate presentation.adoc --format slides
```

## 11. AST Extensions

### 11.1 New Block Types

```rust
/// Block types for presentation support
pub enum Block {
    // ... existing variants ...

    /// Explicit slide break (---)
    SlideBreak,

    /// [slides] block container for dual-nature documents
    SlidesBlock(SlidesBlock),

    /// Speaker notes block
    SpeakerNotes(SpeakerNotes),
}

/// Container for embedded presentation content
pub struct SlidesBlock {
    /// Optional presentation title (overrides document title)
    pub title: Option<String>,

    /// Blocks within the slides section
    pub blocks: Vec<Block>,

    /// Source location for diagnostics
    pub location: SourceLocation,
}

/// Speaker notes for a slide
pub struct SpeakerNotes {
    /// Note content (may contain inline formatting)
    pub content: Vec<Inline>,

    /// Source location
    pub location: SourceLocation,
}
```

### 11.2 Document Attributes

```rust
/// Document-level presentation attributes
pub struct DocumentAttributes {
    // ... existing fields ...

    /// Document type: "slides" for presentation mode
    pub doctype: Option<String>,

    /// Enable slide generation
    pub slides: bool,
}
```

## 12. Error Handling

### 12.1 Diagnostic Messages

| Code | Message | Cause |
|------|---------|-------|
| `PPTX001` | Invalid slide layout index | SlideContract references non-existent layout |
| `PPTX002` | Template missing slide master | POTX has no slideMaster1.xml |
| `PPTX003` | Orphan speaker notes | Notes block without preceding slide |
| `PPTX004` | Nested slides block | `[slides]` block inside another |
| `PPTX005` | Image not found | Referenced image file missing |
| `PPTX006` | Unsupported media type | Image format not supported |

### 12.2 Recovery Strategies

| Error | Recovery |
|-------|----------|
| Missing layout | Fall back to default content layout |
| Missing placeholder | Create shape with absolute positioning |
| Large image | Scale to fit slide dimensions |
| Missing template | Use built-in minimal template |

## 13. Compatibility Matrix

### 13.1 vs Existing Tools

| Feature | Asciidoctor Reveal.js | Marp | utf8dok |
|---------|----------------------|------|---------|
| Output format | HTML | HTML/PDF/PPTX* | PPTX |
| Slide syntax | `== Heading` | `---` | Both |
| Speaker notes | Yes | Yes | Yes |
| Custom templates | Reveal.js themes | CSS themes | POTX |
| Dual-nature docs | No | No | Yes |
| Corporate compliance | No | No | Yes |
| AsciiDoc input | Yes | Markdown only | Yes |

*Marp PPTX is PDF-in-PPTX, not native slides

### 13.2 PowerPoint Version Support

| Feature | PowerPoint 2016+ | PowerPoint Online | LibreOffice Impress |
|---------|------------------|-------------------|---------------------|
| Basic slides | Yes | Yes | Yes |
| Speaker notes | Yes | Yes | Yes |
| Animations | Limited | Limited | Limited |
| Custom layouts | Yes | Yes | Partial |
| Theme colors | Yes | Yes | Yes |

## 14. Implementation Phases

### Phase 1: Basic PPTX Generation

- [x] ADR-010 approval
- [ ] `utf8dok-pptx` crate scaffold
- [ ] POTX template loading
- [ ] Basic slide XML generation
- [ ] Title and content slides
- [ ] Bullet point lists

### Phase 2: Dual-Nature Support

- [ ] `[slides]` block parsing in AST
- [ ] Slide extraction from dual-nature documents
- [ ] Multiple `[slides]` block concatenation
- [ ] Speaker notes support

### Phase 3: Advanced Features

- [ ] Image embedding
- [ ] Table slides
- [ ] Code blocks with styling
- [ ] Diagram integration (Kroki/Mermaid output)
- [ ] SlideContract validation

### Phase 4: Polish

- [ ] CLI integration
- [ ] Template inspection commands
- [ ] Round-trip support (PPTX → AsciiDoc)
- [ ] Documentation and examples

## 15. References

- [Asciidoctor Reveal.js](https://docs.asciidoctor.org/reveal.js-converter/latest/)
- [Marp Syntax](https://marp.app/)
- [PresentationML Structure](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document)
- [OOXML PresentationML](http://officeopenxml.com/anatomyofOOXML-pptx.php)
- [DrawingML Specification](http://officeopenxml.com/drwSp.php)
- ADR-007: Style Mapping Architecture
- ADR-009: Cover Page Architecture
- ADR-010: PPTX Generation and Dual-Nature Documents
