# ADR-010: PPTX Generation and Dual-Nature Documents

## Status
Accepted

## Context

utf8dok generates corporate-compliant DOCX from AsciiDoc. Users also need to create presentations from the same source material. Current workflows require:

1. Writing content in AsciiDoc for documentation
2. Manually recreating slides in PowerPoint for presentations
3. Keeping both in sync

This creates duplication and drift between documentation and presentation materials.

### Existing Solutions Review

| Tool | Output | Slide Syntax | Limitations |
|------|--------|--------------|-------------|
| Asciidoctor Reveal.js | HTML | `== Heading` | No PPTX, requires browser |
| Marp | HTML/PDF | `---` | Markdown only, no PPTX |
| Slidev | HTML | `---` | Vue.js dependency, no PPTX |
| asciidoc-odf | ODP | N/A | Requires LibreOffice conversion |

**Key Finding:** No direct AsciiDoc → PPTX tool exists. utf8dok would be first-to-market.

### PPTX Structure (PresentationML)

PPTX files are OOXML packages similar to DOCX:

```
presentation.pptx (ZIP)
├── [Content_Types].xml
├── _rels/
├── ppt/
│   ├── presentation.xml        # Root element
│   ├── presProps.xml           # Presentation properties
│   ├── slides/
│   │   ├── slide1.xml          # Individual slides
│   │   └── _rels/
│   ├── slideLayouts/           # Layout templates
│   │   ├── slideLayout1.xml    # Title Slide
│   │   ├── slideLayout2.xml    # Title and Content
│   │   └── ...
│   ├── slideMasters/
│   │   └── slideMaster1.xml    # Master formatting
│   ├── theme/
│   │   └── theme1.xml          # Color/font schemes
│   └── media/                  # Images, diagrams
└── docProps/
```

### Dual-Nature Document Concept

A single AsciiDoc source should generate:
1. **Full DOCX** - Complete documentation
2. **PPTX** - Presentation slides extracted from marked sections

## Decision

### 1. Slide Boundary Convention

Adopt Asciidoctor Reveal.js conventions for compatibility:

```asciidoc
= Presentation Title
:slides:

== First Slide Title
Content becomes bullet points

=== Vertical Slide (optional)
Deeper content

== Second Slide
* Explicit
* Bullet
* Points

[.notes]
--
Speaker notes here
--
```

**Rules:**
- `= Title` → Title slide
- `== Heading` → New slide with title
- `=== Subheading` → Vertical/sub-slide (if supported by template)
- `---` (horizontal rule) → Slide break without title
- `[.notes]` block → Speaker notes (not rendered on slide)

### 2. Dual-Nature Markup

#### Mode 1: Whole Document as Presentation

```asciidoc
= My Presentation
:doctype: slides
:slides:

== Slide One
...
```

#### Mode 2: Embedded Presentation Sections

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
[diagram of system]

== Timeline
Project milestones
--

== Detailed Requirements
More documentation...

[slides,title="Technical Deep Dive"]
--
== API Design
...
--
```

**Semantics:**
- `[slides]` block extracts content for PPTX
- Multiple `[slides]` blocks → Multiple presentations or concatenated
- `title` attribute overrides presentation title
- Content outside `[slides]` blocks → DOCX only

### 3. Content Mapping

| AsciiDoc Element | Slide Rendering |
|------------------|-----------------|
| `= Title` | Title slide (centered, large) |
| `== Heading` | Slide title |
| `=== Subheading` | Subtitle or vertical slide |
| Paragraph | Body text |
| Unordered list | Bullet points |
| Ordered list | Numbered points |
| Image | Media placeholder (scaled) |
| Diagram block | Rendered image |
| Table | Table shape |
| Code block | Code box with monospace |
| `[.notes]` | Speaker notes pane |
| Admonition | Styled callout box |

### 4. Template Injection (Same as DOCX)

PPTX templates (`.potx`) define:
- Slide masters (branding, colors)
- Slide layouts (Title, Content, Two Column, etc.)
- Theme (fonts, colors)

utf8dok injects content into template placeholders:

```
Template (.potx)          AsciiDoc               Output (.pptx)
┌─────────────────┐      ┌─────────────┐      ┌─────────────────┐
│ [Title Layout]  │  +   │ = My Talk   │  =   │ My Talk         │
│ [Content Layout]│      │ == Slide 1  │      │ ───────         │
│ [Two Column]    │      │ * Point A   │      │ • Point A       │
└─────────────────┘      └─────────────┘      └─────────────────┘
```

### 5. SlideContract (Extension of StyleContract)

```toml
# slide-contract.toml

[meta]
template = "corporate.potx"
locale = "it-IT"

[layouts]
# Map semantic slide types to template layout indices
title = 1           # slideLayout1.xml (Title Slide)
content = 2         # slideLayout2.xml (Title and Content)
section = 3         # slideLayout3.xml (Section Header)
two_column = 4      # slideLayout4.xml (Two Content)
comparison = 5      # slideLayout5.xml (Comparison)
title_only = 6      # slideLayout6.xml (Title Only)
blank = 7           # slideLayout7.xml (Blank)
image = 8           # slideLayout8.xml (Content with Caption)
quote = 9           # slideLayout9.xml (Quote)

[placeholders]
# Map content types to placeholder indices
title = 0           # Title placeholder
subtitle = 1        # Subtitle placeholder
body = 2            # Body/content placeholder
footer = 10         # Footer placeholder
slide_number = 11   # Slide number placeholder
date = 12           # Date placeholder

[notes]
# Speaker notes formatting
font_size = 12
font_family = "Arial"

[defaults]
# Default layout for different content types
heading_slide = "content"
bullet_slide = "content"
image_slide = "image"
table_slide = "content"
code_slide = "content"
```

### 6. CLI Integration

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

### 7. Crate Structure

```
crates/
├── utf8dok-ooxml/          # Existing - DOCX support
├── utf8dok-pptx/           # New - PPTX support
│   ├── src/
│   │   ├── lib.rs
│   │   ├── writer.rs       # PPTX generation
│   │   ├── template.rs     # .potx loading
│   │   ├── slide.rs        # Slide XML generation
│   │   ├── layout.rs       # Layout mapping
│   │   └── slide_contract.rs
│   └── Cargo.toml
└── utf8dok-core/           # Shared AST, parser
```

### 8. AST Extensions

```rust
// New block type for slide boundaries
pub enum Block {
    // ... existing variants
    SlideBreak,           // Explicit slide break (---)
    SlidesBlock(Slides),  // [slides] block container
}

pub struct Slides {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

// Speaker notes
pub struct SpeakerNotes {
    pub content: Vec<Inline>,
}
```

## Consequences

### Positive

1. **Single Source of Truth**: One AsciiDoc file for docs and slides
2. **Template Compliance**: Corporate POTX templates supported
3. **First-to-Market**: No existing AsciiDoc → PPTX solution
4. **Consistent Architecture**: Same pattern as DOCX (template injection)
5. **Standards-Based**: OOXML PresentationML compliance
6. **Reveal.js Compatible**: Slide syntax works with existing tools

### Negative

1. **Complexity**: Two output formats to maintain
2. **Layout Limitations**: Not all PPTX layouts easily mapped
3. **Fidelity Challenges**: Complex slides may need manual adjustment
4. **Testing Surface**: More combinations to test

## Implementation Phases

### Phase 1: Basic PPTX Generation
- Single-file presentation documents (`:doctype: slides`)
- Title, content, and bullet slides
- Basic template injection

### Phase 2: Dual-Nature Support
- `[slides]` block parsing
- Multiple presentation extraction
- Speaker notes

### Phase 3: Advanced Features
- Diagrams as slide images
- Table slides
- Code slides with syntax highlighting
- Animations (fragments)

## References

- [Asciidoctor Reveal.js](https://docs.asciidoctor.org/reveal.js-converter/latest/)
- [Marp Syntax](https://marp.app/)
- [Slidev Syntax Guide](https://sli.dev/guide/syntax)
- [PresentationML Structure](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document)
- [OOXML PresentationML](http://officeopenxml.com/anatomyofOOXML-pptx.php)
- ADR-007: Style Mapping Architecture
- ADR-009: Cover Page Architecture
