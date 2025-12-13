# utf8dok: Template-Aware Document Generation

## Problem Statement

Existing AsciiDoc-to-DOCX solutions generate documents from scratch, producing "plain" output that doesn't match corporate document standards. Organizations need:

- **Template compliance**: Documents must use approved corporate templates
- **Style preservation**: Headings, tables, and formatting must match company standards  
- **Metadata integration**: Document properties, revision history, cover pages
- **Professional output**: TOC, headers/footers, logos, branding

## Target Use Case

Technical documentation that must conform to corporate templates:
- Design documents
- Architecture specifications
- Requirements documents
- Technical proposals

## Key Differentiator

| Approach | How It Works | Result |
|----------|--------------|--------|
| **Existing tools** | Generate OOXML from scratch | Generic, unstyled documents |
| **utf8dok** | Inject content into .dotx template | Corporate-compliant output |

---

## Template Capabilities Required

### 1. Cover Page Support
- Logo placeholders
- Title/subtitle fields
- Date, version, author metadata
- Confidentiality notices

### 2. Corporate Styling
- Custom heading styles (colors, fonts, numbering)
- Table styles (alternating rows, borders)
- Paragraph formatting (spacing, justification)

### 3. Document Structure
- Auto-generated Table of Contents
- Headers/footers with document info
- Page numbering
- Section breaks

### 4. Metadata Tables
- Document information (ID, version, status)
- Revision history
- Definitions/acronyms
- Requirements matrices

---

## Architecture

### Two Workflows

**Workflow A: Bootstrap (docx → adoc)**
Extract structure from existing documents to bootstrap AsciiDoc authoring.

```
┌─────────────────────┐      ┌─────────────────────┐
│  existing.docx      │      │  template.dotx      │
│  (or template)      │      │  (preserved)        │
└─────────────────────┘      └─────────────────────┘
          │                            │
          ▼                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    utf8dok extract                           │
│                                                              │
│  1. Parse document structure                                │
│  2. Identify styles → AsciiDoc mapping                      │
│  3. Extract content as AsciiDoc                             │
│  4. Generate utf8dok.toml configuration                     │
│  5. Preserve template for round-trip                        │
└─────────────────────────────────────────────────────────────┘
          │                            │
          ▼                            ▼
┌─────────────────────┐      ┌─────────────────────┐
│  document.adoc      │      │  utf8dok.toml       │
│  (editable source)  │      │  (style mappings)   │
└─────────────────────┘      └─────────────────────┘
```

**Workflow B: Render (adoc → docx)**
Generate corporate-compliant documents from AsciiDoc source.

```
┌─────────────────────┐      ┌─────────────────────┐
│  document.adoc      │      │  utf8dok.toml       │
│  (authored content) │      │  (configuration)    │
└─────────────────────┘      └─────────────────────┘
          │                            │
          ▼                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    utf8dok render                            │
│                                                              │
│  1. Parse AsciiDoc → AST                                    │
│  2. Load template (.dotx)                                   │
│  3. Map AST nodes → template styles                         │
│  4. Replace metadata placeholders                           │
│  5. Inject body content                                     │
│  6. Update TOC, fields                                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  output.docx                                 │
│            (corporate styling preserved)                     │
└─────────────────────────────────────────────────────────────┘
```

### Round-Trip Benefits

| Benefit | Description |
|---------|-------------|
| **Version control** | AsciiDoc is plain text, perfect for Git |
| **Diff-friendly** | See exactly what changed between versions |
| **Collaborative** | Multiple authors, merge conflicts are manageable |
| **Template fidelity** | Output always matches corporate standards |
| **Migration path** | Convert existing docs without losing formatting |

---

## Configuration Model

```toml
# utf8dok.toml

[template]
path = "templates/design-document.dotx"
content_marker = "{{CONTENT}}"  # Where body content is injected

[styles]
# Map AsciiDoc elements to Word style names
heading1 = "Heading 1"
heading2 = "Heading 2"
heading3 = "Heading 3"
paragraph = "Normal"
table = "Table Grid"
code_block = "Code"
list_bullet = "List Bullet"
list_number = "List Number"

[placeholders]
# Map document attributes to template placeholders
title = "{{TITLE}}"
subtitle = "{{SUBTITLE}}"
version = "{{VERSION}}"
date = "{{DATE}}"
author = "{{AUTHOR}}"
status = "{{STATUS}}"

[tables]
# Special table types with predefined structure
metadata = "DocumentInfo"
revision_history = "RevisionHistory"
requirements = "Requirements"
```

---

## AsciiDoc Extensions for Corporate Documents

### Document Metadata

```asciidoc
= API Gateway Migration Design
:doctype: design
:doc-id: DES-001
:version: 1.0
:status: DRAFT
:date: 2025-01-15
:author: Architecture Team
:reviewer: Technical Lead
:approver: CTO
```

### Metadata Tables

```asciidoc
[.document-info]
--
Document Title:: API Gateway Migration Design
Document ID:: DES-001
Version:: 1.0
Status:: DRAFT | IN REVIEW | APPROVED
--

[.revision-history]
|===
|Version |Date |Author |Changes |Status

|0.1 |2025-01-10 |J. Smith |Initial draft |DRAFT
|1.0 |2025-01-15 |J. Smith |Incorporated review feedback |IN REVIEW
|===
```

### Requirements Tables

```asciidoc
[.requirements, type=functional]
|===
|ID |Description |Priority

|FR-001 |Support OAuth 2.0 authentication |0
|FR-002 |Rate limiting per API endpoint |1
|===
```

---

## Implementation Phases

### Phase 0: Extraction (docx → adoc)
- [ ] OOXML parser (unpack .docx/.dotx)
- [ ] Style analyzer (detect heading levels, table styles)
- [ ] Content extractor (paragraphs, tables, lists → AsciiDoc)
- [ ] Config generator (create utf8dok.toml from detected styles)
- [ ] Template preservation (strip content, keep structure)

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
- [ ] Table of Contents update
- [ ] Header/footer field updates
- [ ] Cross-references
- [ ] Images

### Phase 4: Advanced
- [ ] Metadata tables (auto-generated)
- [ ] Admonitions → styled callouts
- [ ] Code blocks with formatting
- [ ] Conditional content

---

## Comparison with Existing Tools

| Feature | asciidocr | asciidork | utf8dok |
|---------|-----------|-----------|---------|
| HTML output | ✅ | ✅ | Planned |
| Basic DOCX | ⚠️ Experimental | ❌ | ✅ Primary |
| Template support | ❌ | ❌ | ✅ |
| Style mapping | ❌ | ❌ | ✅ |
| TOC generation | ❌ | ❌ | ✅ |
| Corporate templates | ❌ | ❌ | ✅ |
| **DOCX → AsciiDoc** | ❌ | ❌ | ✅ |
| **Round-trip editing** | ❌ | ❌ | ✅ |
| WASM | ❌ | ✅ | Planned |
| TCK compliance | ✅ | Partial | Goal |

---

## Why Build This?

1. **Unmet need**: No existing tool produces template-compliant DOCX
2. **Real-world requirement**: Organizations have mandatory templates
3. **AI-assisted development**: Clean architecture without legacy constraints
4. **Learning opportunity**: Parser construction, OOXML, Rust patterns
5. **Contribution potential**: Fills a gap in the AsciiDoc ecosystem

---

## CLI Usage

```bash
# Extract: Bootstrap AsciiDoc from existing document
utf8dok extract document.docx --output project/
# Creates:
#   project/document.adoc      (content as AsciiDoc)
#   project/template.dotx      (template with styles, no content)
#   project/utf8dok.toml       (auto-detected style mappings)

# Render: Generate docx from AsciiDoc
utf8dok render project/document.adoc --output final.docx
# Uses project/utf8dok.toml for configuration

# Render with explicit config
utf8dok render document.adoc --config custom.toml --output final.docx

# Validate: Check AsciiDoc against template requirements
utf8dok validate document.adoc --config utf8dok.toml
```
