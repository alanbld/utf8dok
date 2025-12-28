# utf8dok: Template-Aware Document Generation

## Problem Statement

Existing AsciiDoc-to-DOCX solutions generate documents from scratch, producing "plain" output that doesn't match corporate document standards. Organizations need:

- **Template compliance**: Documents must use approved corporate templates
- **Style preservation**: Headings, tables, and formatting must match company standards  
- **Metadata integration**: Document properties, revision history, cover pages
- **Professional output**: TOC, headers/footers, logos, branding
- **Diagrams-as-code**: Technical diagrams version-controlled alongside text

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
| **utf8dok** | Self-contained DOCX with embedded sources | Lossless round-trip editing |

---

## Core Innovation: Self-Contained DOCX

DOCX files are ZIP archives. Word ignores folders it doesn't recognize. utf8dok embeds source files directly inside the DOCX:

```
document.docx (ZIP archive)
├── [Content_Types].xml
├── _rels/
├── docProps/
├── word/
│   ├── document.xml
│   ├── styles.xml
│   └── media/
│       ├── image1.png          # Rendered diagram (what Word displays)
│       └── image2.png
└── utf8dok/                     # Custom folder — Word ignores this
    ├── manifest.json            # Element mappings & metadata
    ├── source.adoc              # Original AsciiDoc (optional)
    ├── config.toml              # Style configuration
    └── diagrams/
        ├── image1.puml          # PlantUML source for image1.png
        └── image2.mmd           # Mermaid source for image2.png
```

### Benefits of Self-Contained Architecture

| Benefit | Description |
|---------|-------------|
| **Self-contained** | Single file has everything needed to reconstruct |
| **No external dependencies** | Share the .docx, recipient has full source |
| **Diagram regeneration** | Change PlantUML source, re-render, image updates |
| **Lossless round-trip** | Extract → edit → render produces identical structure |
| **Git-friendly option** | Can still extract to files for version control |
| **Word-compatible** | Opens normally in Word — custom folder is ignored |

---

## Architecture: Two Workflows

### Workflow A: Extract (DOCX → AsciiDoc)

Bootstrap AsciiDoc authoring from existing documents or templates.

```
┌─────────────────────────┐      ┌─────────────────────────┐
│  existing.docx          │      │  (preserved as)         │
│  (corporate document)   │      │  template structure     │
└─────────────────────────┘      └─────────────────────────┘
          │                                  │
          ▼                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                      utf8dok extract                            │
│                                                                 │
│  1. Parse document structure (word/document.xml)               │
│  2. Analyze styles → AsciiDoc mapping                          │
│  3. Extract content as AsciiDoc                                │
│  4. Detect diagrams, preserve sources if embedded              │
│  5. Generate manifest.json with element IDs                    │
│  6. Generate utf8dok.toml configuration                        │
└─────────────────────────────────────────────────────────────────┘
          │                                  │
          ▼                                  ▼
┌─────────────────────────┐      ┌─────────────────────────┐
│  document.adoc          │      │  utf8dok.toml           │
│  (editable source)      │      │  (style mappings)       │
└─────────────────────────┘      └─────────────────────────┘
```

### Workflow B: Render (AsciiDoc → DOCX)

Generate corporate-compliant documents from AsciiDoc source.

```
┌─────────────────────────┐      ┌─────────────────────────┐
│  document.adoc          │      │  utf8dok.toml           │
│  (authored content)     │      │  (configuration)        │
└─────────────────────────┘      └─────────────────────────┘
          │                                  │
          ▼                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                      utf8dok render                             │
│                                                                 │
│  1. Parse AsciiDoc → AST                                       │
│  2. Render diagrams via Kroki (cache by content-hash)          │
│  3. Load template (.dotx)                                      │
│  4. Map AST nodes → template styles                            │
│  5. Replace metadata placeholders                              │
│  6. Inject body content with stable element IDs                │
│  7. Embed sources in /utf8dok/ folder                          │
│  8. Update TOC, fields                                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        output.docx                              │
│                                                                 │
│  word/           ← Standard OOXML (opens in Word)              │
│  utf8dok/        ← Embedded sources (ignored by Word)          │
│    manifest.json                                                │
│    source.adoc                                                  │
│    diagrams/*.puml                                              │
└─────────────────────────────────────────────────────────────────┘
```

### Round-Trip Flow

```
                    EXTRACT                              RENDER
    ┌─────────────────────────────────────┐  ┌─────────────────────────────────────┐
    │                                     │  │                                     │
    │  ┌─────────────┐                    │  │                    ┌─────────────┐  │
    │  │ corporate   │                    │  │                    │  output     │  │
    │  │ .docx       │                    │  │                    │  .docx      │  │
    │  │             │                    │  │                    │             │  │
    │  │ word/       │──────extract──────▶│  │◀────render────────│  word/      │  │
    │  │ utf8dok/    │                    │  │                    │  utf8dok/   │  │
    │  │  └manifest  │                    │  │                    │   └manifest │  │
    │  │  └diagrams/ │                    │  │                    │   └diagrams/│  │
    │  └─────────────┘                    │  │                    └─────────────┘  │
    │         │                           │  │                           ▲         │
    │         ▼                           │  │                           │         │
    │  ┌─────────────┐                    │  │                    ┌──────┴──────┐  │
    │  │ document    │◀───────────────────┼──┼────edit as text───▶│ document    │  │
    │  │ .adoc       │                    │  │                    │ .adoc       │  │
    │  └─────────────┘                    │  │                    └─────────────┘  │
    │                                     │  │                                     │
    └─────────────────────────────────────┘  └─────────────────────────────────────┘
```

---

## Bidirectional Element Mapping

For lossless round-trips, elements need stable identifiers that survive both directions.

### Naming Convention

```
{type}-{semantic-name}[-{sequence}]
```

| Type Prefix | Element | Example |
|-------------|---------|---------|
| `sec-` | Section/Heading | `sec-architecture`, `sec-requirements` |
| `tbl-` | Table | `tbl-revision-history`, `tbl-requirements` |
| `fig-` | Figure/Diagram | `fig-system-context`, `fig-dataflow` |
| `lst-` | Code listing | `lst-config-example` |
| `req-` | Requirement | `req-FR-001` |

### AsciiDoc Anchors

```asciidoc
[[sec-architecture]]
== Architecture Overview

[[tbl-requirements]]
.Requirements Matrix
|===
|ID |Description
|FR-001 |OAuth support
|===

[[fig-system-context]]
[mermaid]
----
graph TD
    A[Client] --> B[API Gateway]
----
```

### OOXML Mapping

```xml
<w:p w14:paraId="sec-architecture">
  <w:bookmarkStart w:name="sec-architecture"/>
  <w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
  <w:r><w:t>Architecture Overview</w:t></w:r>
  <w:bookmarkEnd/>
</w:p>
```

### Manifest Structure

```json
{
  "version": "1.0",
  "generator": "utf8dok",
  "generated_at": "2025-01-15T10:30:00Z",
  "elements": {
    "sec-architecture": {
      "type": "heading",
      "level": 2,
      "paraId": "4A3B2C1D"
    },
    "fig-system-context": {
      "type": "figure",
      "media": "word/media/image1.png",
      "source": "utf8dok/diagrams/system-context.mmd",
      "source_type": "mermaid",
      "hash": "sha256:abc123..."
    },
    "tbl-requirements": {
      "type": "table",
      "style": "Requirements"
    }
  },
  "config": {
    "styles": {
      "heading1": "Heading 1",
      "heading2": "Heading 2"
    }
  }
}
```

---

## Diagram Integration

### The Challenge

DOCX files need actual images (PNG/SVG), not text-based diagram code. We render diagrams at build time using Kroki.

### Supported Diagram Types (via Kroki)

| Type | Syntax Block | Use Case |
|------|--------------|----------|
| Mermaid | `[mermaid]` | Flowcharts, sequence diagrams, ER diagrams |
| PlantUML | `[plantuml]` | UML diagrams, architecture |
| Graphviz | `[graphviz]` | Network diagrams, graphs |
| Ditaa | `[ditaa]` | ASCII art diagrams |
| D2 | `[d2]` | Modern diagramming |
| BlockDiag | `[blockdiag]` | Block diagrams |

### AsciiDoc Syntax (Asciidoctor-Compatible)

```asciidoc
= Architecture Document

== System Overview

[[fig-system-overview]]
.System Architecture
[mermaid]
----
graph TD
    A[Load Balancer] --> B[API Gateway]
    B --> C[Auth Service]
    B --> D[Business Service]
    C --> E[(Database)]
    D --> E
----

== Sequence Flow

[[fig-auth-flow]]
.Authentication Sequence
[plantuml]
----
@startuml
Client -> Gateway: Request
Gateway -> Auth: Validate
Auth --> Gateway: OK
Gateway -> Service: Process
Service --> Gateway: Response
Gateway --> Client: Response
@enduml
----
```

### Diagram Rendering Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DIAGRAM RENDERING PIPELINE                           │
│                                                                          │
│  AsciiDoc Source                                                        │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ [[fig-api-flow]]                                             │       │
│  │ [mermaid]                                                    │       │
│  │ ----                                                         │       │
│  │ graph LR                                                     │       │
│  │     A[Client] --> B[API Gateway]                             │       │
│  │ ----                                                         │       │
│  └─────────────────────────────────────────────────────────────┘       │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │                    utf8dok-diagrams                          │       │
│  │                                                              │       │
│  │  1. Detect diagram blocks (mermaid, plantuml, graphviz...)  │       │
│  │  2. Compute content-hash for caching                        │       │
│  │  3. Check cache — if hit, use cached image                  │       │
│  │  4. If miss: send to Kroki (self-hosted or kroki.io)        │       │
│  │  5. Receive PNG, store in cache                             │       │
│  │  6. Replace block with image reference                       │       │
│  └─────────────────────────────────────────────────────────────┘       │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │                    Output DOCX                               │       │
│  │                                                              │       │
│  │  word/media/fig-api-flow.png   ← Rendered image             │       │
│  │  utf8dok/diagrams/fig-api-flow.mmd  ← Source preserved      │       │
│  │  <w:drawing>...</w:drawing>    ← Image reference in doc     │       │
│  └─────────────────────────────────────────────────────────────┘       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Diagram Workflow Benefits

| Aspect | Benefit |
|--------|---------|
| **Kroki unified API** | One integration supports 20+ diagram types |
| **Self-hostable** | Corporate environments can run `docker-compose up` |
| **Content-hash caching** | Deterministic, no redundant renders |
| **Asciidoctor-compatible syntax** | Users already know `[mermaid]` blocks |
| **Offline mode** | CI/CD can pre-warm cache, build without network |
| **Source preservation** | Diagram code embedded in DOCX for round-trip |

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

[anchors]
# Strategy for generating anchors during extraction
strategy = "semantic"  # or "preserve-ids" or "sequential"
prefix_headings = "sec"
prefix_tables = "tbl"
prefix_figures = "fig"

[diagrams]
# Kroki server (default: public server)
server = "https://kroki.io"
# Or self-hosted:
# server = "http://localhost:8000"

# Output format for DOCX
format = "png"  # "png" or "svg"

# Cache directory (relative to project)
cache = ".utf8dok/diagram-cache"

# Offline mode (use cache only)
offline = false

# Timeout in seconds
timeout = 30

# Default diagram width in DOCX
default_width = "6in"

# Diagram types to recognize
source_types = ["mermaid", "plantuml", "graphviz", "ditaa", "d2"]

[package]
# What to embed in the docx
embed_manifest = true
embed_diagram_sources = true
embed_original_adoc = false  # Optional — might want for "source of truth"
embed_config = true
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

[[tbl-revision-history]]
[.revision-history]
|===
|Version |Date |Author |Changes |Status

|0.1 |2025-01-10 |J. Smith |Initial draft |DRAFT
|1.0 |2025-01-15 |J. Smith |Incorporated review feedback |IN REVIEW
|===
```

### Requirements Tables

```asciidoc
[[tbl-functional-requirements]]
[.requirements, type=functional]
|===
|ID |Description |Priority

|FR-001 |Support OAuth 2.0 authentication |0
|FR-002 |Rate limiting per API endpoint |1
|===
```

---

## CLI Usage

```bash
# Extract: Bootstrap AsciiDoc from existing document
utf8dok extract document.docx --output project/
# Creates:
#   project/document.adoc      (content as AsciiDoc)
#   project/template.dotx      (template with styles, no content)
#   project/utf8dok.toml       (auto-detected style mappings)

# Extract from self-contained utf8dok DOCX (uses embedded sources)
utf8dok extract document.docx --use-embedded
# Extracts from utf8dok/ folder if present

# Render: Generate docx from AsciiDoc
utf8dok render project/document.adoc --output final.docx
# Uses project/utf8dok.toml for configuration
# Embeds sources in utf8dok/ folder

# Render with explicit config
utf8dok render document.adoc --config custom.toml --output final.docx

# Render without embedding sources (smaller file, no round-trip)
utf8dok render document.adoc --no-embed --output final.docx

# Validate: Check AsciiDoc against template requirements
utf8dok validate document.adoc --config utf8dok.toml

# Diagram cache management
utf8dok diagrams warm --config utf8dok.toml  # Pre-render all diagrams
utf8dok diagrams clear                        # Clear cache
```

---

## Implementation Phases

### Phase 0: Extraction (DOCX → AsciiDoc) ✓ In Progress

- [x] OOXML parser (unpack .docx/.dotx)
- [x] Style analyzer (detect heading levels)
- [x] Content extractor (paragraphs, tables → AsciiDoc)
- [ ] Config generator (create utf8dok.toml from detected styles)
- [ ] Template preservation (strip content, keep structure)
- [ ] Manifest generation with element IDs

### Phase 1: Core Rendering Infrastructure

- [ ] Template loader (.dotx parsing)
- [ ] Style registry (map AsciiDoc → Word styles)
- [ ] Placeholder replacement engine
- [ ] Basic content injection
- [ ] Element ID preservation (anchors ↔ bookmarks)

### Phase 2: Diagram Integration

- [ ] Kroki client (HTTP API)
- [ ] Content-hash caching
- [ ] Mermaid + PlantUML support
- [ ] Image embedding in word/media/
- [ ] Source preservation in utf8dok/diagrams/

### Phase 3: Content Rendering

- [ ] Paragraphs with style mapping
- [ ] Headings (auto-numbered)
- [ ] Tables (simple and styled)
- [ ] Lists (bulleted, numbered)
- [ ] Inline formatting (bold, italic, code)

### Phase 4: Document Features

- [ ] Table of Contents update
- [ ] Header/footer field updates
- [ ] Cross-references
- [ ] Images (non-diagram)

### Phase 5: Advanced

- [ ] Metadata tables (auto-generated)
- [ ] Admonitions → styled callouts
- [ ] Code blocks with formatting
- [ ] Conditional content
- [ ] Full Kroki diagram type support

---

## Crate Structure

```
utf8dok/
├── Cargo.toml                    # Workspace
├── crates/
│   ├── utf8dok-cli/              # Command-line interface
│   │   └── src/
│   │       ├── main.rs
│   │       ├── extract.rs
│   │       ├── render.rs
│   │       └── validate.rs
│   │
│   ├── utf8dok-parser/           # AsciiDoc parser
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grammar.pest
│   │       └── ast.rs
│   │
│   ├── utf8dok-ooxml/            # DOCX reading/writing
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── archive.rs        # ZIP handling
│   │       ├── document.rs       # word/document.xml
│   │       ├── styles.rs         # word/styles.xml
│   │       ├── manifest.rs       # utf8dok/manifest.json
│   │       └── extract.rs        # DOCX → AsciiDoc
│   │
│   ├── utf8dok-diagrams/         # Diagram rendering
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── kroki.rs          # Kroki API client
│   │       ├── cache.rs          # Content-hash caching
│   │       └── types.rs          # DiagramType enum
│   │
│   └── utf8dok-render/           # AsciiDoc → DOCX
│       └── src/
│           ├── lib.rs
│           ├── template.rs       # Template loading
│           ├── inject.rs         # Content injection
│           └── package.rs        # Final DOCX assembly
```

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
| **Diagrams-as-code** | ❌ | ❌ | ✅ |
| **Self-contained DOCX** | ❌ | ❌ | ✅ |
| WASM | ❌ | ✅ | Planned |
| TCK compliance | ✅ | Partial | Goal |

---

## Why Build This?

1. **Unmet need**: No existing tool produces template-compliant DOCX with round-trip capability
2. **Real-world requirement**: Organizations have mandatory templates and version control needs
3. **Diagrams-as-code**: Technical documentation requires version-controlled diagrams
4. **Self-contained documents**: Share a single file that contains everything needed to reconstruct
5. **AI-assisted development**: Clean architecture without legacy constraints
6. **Learning opportunity**: Parser construction, OOXML, Rust patterns
7. **Contribution potential**: Fills a gap in the AsciiDoc ecosystem
