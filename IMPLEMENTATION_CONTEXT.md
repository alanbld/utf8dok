# utf8dok Implementation Context

> Context prompt for AI-assisted development sessions

## Project Vision

utf8dok is a Rust-based tool for template-aware document generation. Unlike existing AsciiDoc processors that generate plain/unstyled output, utf8dok injects content into corporate Word templates (.dotx), preserving styles, branding, and structure.

## Core Problem

Existing AsciiDoc → DOCX tools generate documents from scratch, producing generic output that doesn't match corporate standards. Organizations need documents that:

- Use approved corporate templates
- Preserve heading styles, table formatting, branding
- Include proper cover pages, TOC, headers/footers
- Support round-trip editing (DOCX ↔ AsciiDoc)

## Two Workflows

**Workflow A: Extract (docx → adoc)**
Bootstrap AsciiDoc authoring from existing documents:

```
existing.docx → utf8dok extract → document.adoc + template.dotx + utf8dok.toml
```

**Workflow B: Render (adoc → docx)**
Generate corporate-compliant output:

```
document.adoc + template.dotx + utf8dok.toml → utf8dok render → output.docx
```

## Architecture

```
crates/
├── utf8dok-core/     # AsciiDoc parser (pest grammar)
├── utf8dok-ast/      # AST type definitions
├── utf8dok-ooxml/    # DOCX/DOTX parsing and generation
├── utf8dok-cli/      # CLI (extract, render, validate commands)
└── utf8dok-wasm/     # WebAssembly bindings (future)
```

## Key Technical Decisions

1. **Parser**: pest (PEG) for AsciiDoc parsing
2. **OOXML handling**: Parse/generate Office Open XML format
3. **Template injection**: Replace placeholders, map styles, inject body content
4. **Configuration**: TOML-based style mappings (utf8dok.toml)

## Implementation Phases

### Phase 0: Extraction (docx → adoc)
- [ ] OOXML parser (unpack .docx/.dotx, parse document.xml)
- [ ] Style analyzer (detect heading levels, table styles)
- [ ] Content extractor (paragraphs, tables, lists → AsciiDoc)
- [ ] Config generator (create utf8dok.toml from detected styles)

### Phase 1: Core Infrastructure
- [ ] Template loader (.dotx parsing)
- [ ] Style registry (map AsciiDoc → Word styles)
- [ ] Placeholder replacement engine
- [ ] Basic content injection

### Phase 2: Content Rendering
- [ ] Paragraphs with style mapping
- [ ] Headings (auto-numbered)
- [ ] Tables (simple and styled)
- [ ] Lists (bulleted, numbered)

### Phase 3: Document Features
- [ ] TOC update
- [ ] Header/footer fields
- [ ] Cross-references
- [ ] Images

## External Knowledge: OOXML Format

The .docx format is a ZIP archive containing XML files:

```
document.docx (ZIP)
├── [Content_Types].xml     # Content type definitions
├── _rels/
│   └── .rels              # Package relationships
├── word/
│   ├── document.xml       # Main document content
│   ├── styles.xml         # Style definitions
│   ├── numbering.xml      # List numbering definitions
│   ├── settings.xml       # Document settings
│   ├── header1.xml        # Header content
│   ├── footer1.xml        # Footer content
│   ├── _rels/
│   │   └── document.xml.rels
│   └── media/             # Embedded images
└── docProps/
    ├── core.xml           # Core properties (title, author)
    └── app.xml            # Application properties
```

Key XML elements in `word/document.xml`:
- `<w:p>` - Paragraph
- `<w:r>` - Run (text with formatting)
- `<w:t>` - Text content
- `<w:pPr>` - Paragraph properties
- `<w:rPr>` - Run properties
- `<w:pStyle>` - Paragraph style reference
- `<w:tbl>` - Table
- `<w:sectPr>` - Section properties

## Configuration Model

```toml
# utf8dok.toml

[template]
path = "templates/design-document.dotx"
content_marker = "{{CONTENT}}"

[styles]
heading1 = "Heading 1"
heading2 = "Heading 2"
heading3 = "Heading 3"
paragraph = "Normal"
table = "Table Grid"
code_block = "Code"
list_bullet = "List Bullet"
list_number = "List Number"

[placeholders]
title = "{{TITLE}}"
subtitle = "{{SUBTITLE}}"
version = "{{VERSION}}"
date = "{{DATE}}"
author = "{{AUTHOR}}"
status = "{{STATUS}}"

[tables]
metadata = "DocumentInfo"
revision_history = "RevisionHistory"
requirements = "Requirements"
```

## CLI Commands

```bash
# Extract: Bootstrap from existing document
utf8dok extract document.docx --output project/
# Creates: project/document.adoc, project/template.dotx, project/utf8dok.toml

# Render: Generate DOCX from AsciiDoc
utf8dok render document.adoc --output final.docx

# Render with explicit config
utf8dok render document.adoc --config custom.toml --output final.docx

# Validate: Check AsciiDoc against template
utf8dok validate document.adoc --config utf8dok.toml
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## Key Differentiator

| Approach | How It Works | Result |
|----------|--------------|--------|
| **Existing tools** | Generate OOXML from scratch | Generic, unstyled |
| **utf8dok** | Inject content into .dotx template | Corporate-compliant |

## Success Criteria

- **Extract**: Convert existing .docx to editable AsciiDoc
- **Render**: Generate .docx that matches template exactly
- **Round-trip**: docx → adoc → docx preserves styling
- **Diff-friendly**: AsciiDoc changes are Git-trackable

## Dependencies (Planned)

```toml
[dependencies]
zip = "0.6"              # DOCX unpacking
quick-xml = "0.31"       # XML parsing
pest = "2.7"             # AsciiDoc grammar
pest_derive = "2.7"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"             # Configuration
thiserror = "1.0"        # Error handling
clap = { version = "4", features = ["derive"] }  # CLI
```

## Recommended Starting Point

1. Create `utf8dok-ooxml` crate for OOXML handling
2. Implement DOCX unpacking (ZIP extraction)
3. Parse `word/document.xml` to understand structure
4. Extract paragraphs with their styles as proof of concept
5. Generate minimal AsciiDoc output
6. Then iterate on the parser for full round-trip
