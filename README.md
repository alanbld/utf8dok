# utf8dok

Template-aware document generation from AsciiDoc to corporate-compliant DOCX.

## Problem

Existing AsciiDoc → DOCX tools generate generic, unstyled documents. Organizations need output that matches approved corporate templates with:

- Custom heading styles, colors, and numbering
- Logo placements and branding
- Proper TOC, headers, footers
- Metadata tables and revision history

## Solution

utf8dok injects content into Word templates (.dotx) instead of generating OOXML from scratch.

## Workflows

### Extract (Bootstrap)

Convert existing Word documents to AsciiDoc for editing:

```bash
utf8dok extract existing.docx --output project/
# Creates:
#   project/document.adoc    (editable content)
#   project/template.dotx    (preserved styling)
#   project/utf8dok.toml     (style mappings)
```

### Render (Generate)

Generate corporate-compliant DOCX from AsciiDoc:

```bash
utf8dok render project/document.adoc --output final.docx
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## Project Structure

```
crates/
├── utf8dok-ooxml/    # OOXML parsing and generation
├── utf8dok-ast/      # AST type definitions (planned)
├── utf8dok-core/     # AsciiDoc parser (planned)
└── utf8dok-cli/      # CLI commands (planned)
```

## Configuration

```toml
# utf8dok.toml

[template]
path = "templates/design-document.dotx"

[styles]
heading1 = "Heading 1"
heading2 = "Heading 2"
paragraph = "Normal"
table = "Table Grid"

[placeholders]
title = "{{TITLE}}"
version = "{{VERSION}}"
```

## Status

- [x] OOXML archive handling (ZIP read/write)
- [x] Document parsing (paragraphs, tables, runs)
- [x] Style parsing (heading detection, inheritance)
- [x] Basic extraction (DOCX → AsciiDoc)
- [ ] AsciiDoc parser
- [ ] Template injection
- [ ] TOC update
- [ ] CLI

## License

MIT OR Apache-2.0
