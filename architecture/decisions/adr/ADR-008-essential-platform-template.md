# ADR-008: utf8dok Essential Platform Template

## Status
Accepted

## Context

### The Template Challenge

utf8dok aims to produce corporate-compliant DOCX documents through template injection. However:

1. **Corporate templates are proprietary** - Fonts, logos, and branding cannot be open-sourced
2. **Template styles vary by locale** - Italian templates use `Titolo1`, English use `Heading1`
3. **Round-trip fidelity requires style stability** - Arbitrary style names break reproducibility

### The Corporate Heritage

The project originated with Italian corporate templates. When anonymizing for open-source:
- **Fonts**: Replaced CocoEng with Inter (OFL licensed)
- **Logos**: Replaced corporate logos with generic placeholders
- **Colors**: Adopted a vibrant, WCAG-compliant palette
- **Style IDs**: Retained Italian names (`Titolo1`, `Normale`, etc.)

The Italian style IDs are not a limitation - they are an asset that validates utf8dok's locale-independence.

### The StyleContract Bridge

ADR-007 introduced the StyleContract as a first-class artifact that separates semantic content from presentation. This enables:

```
Any Template (any locale) + StyleContract → Semantic AsciiDoc → StyleContract → Identical DOCX
```

The template's internal naming is irrelevant; the contract provides the mapping.

## Decision

### Define "utf8dok Essential" as the Canonical Open-Source Template

**utf8dok Essential** is:
- An open-source Word/PowerPoint template for development and demonstration
- Derived from a real corporate template, anonymized for public use
- The reference implementation for StyleContract-based round-trips

### Preserve Locale-Heritage Style IDs

Keep the Italian style IDs in `open_template.dotx`:

| Style ID | English Name | Semantic Role |
|----------|--------------|---------------|
| `Normale` | Normal | body |
| `Titolo1` | Heading 1 | h1 |
| `Titolo2` | Heading 2 | h2 |
| `Titolo3` | Heading 3 | h3 |
| `Titolo4` | Heading 4 | h4 |
| `Titolo5` | Heading 5 | h5 |
| `Titolo6` | Heading 6 | h6 |
| `Titolo7` | Heading 7 | h7 |
| `Titolo8` | Heading 8 | h8 |
| `Titolo9` | Heading 9 | h9 |
| `Didascalia` | Caption | caption |
| `Intestazione` | Header | header |
| `Pidipagina` | Footer | footer |

This validates that utf8dok handles real-world templates, not just idealized English ones.

### Create a Canonical StyleContract

The file `templates/utf8dok-essential.toml` defines the authoritative mapping:

```toml
# utf8dok Essential - StyleContract for open_template.dotx
# This is the canonical mapping between Italian-heritage styles and AsciiDoc semantics

[meta]
template = "open_template.dotx"
template_name = "utf8dok Essential"
locale = "it-IT"
version = "1.0.0"
created = "2025-12-31T12:00:00Z"

[paragraph_styles]
# Heading hierarchy (Italian style IDs)
Titolo1 = { role = "h1", heading_level = 1 }
Titolo2 = { role = "h2", heading_level = 2 }
Titolo3 = { role = "h3", heading_level = 3 }
Titolo4 = { role = "h4", heading_level = 4 }
Titolo5 = { role = "h5", heading_level = 5 }
Titolo6 = { role = "h6", heading_level = 6 }
Titolo7 = { role = "h7", heading_level = 7 }
Titolo8 = { role = "h8", heading_level = 8 }
Titolo9 = { role = "h9", heading_level = 9 }

# Body text
Normale = { role = "body" }
Nessunaspaziatura = { role = "body-compact" }

# Structural
Intestazione = { role = "header" }
Pidipagina = { role = "footer" }
Didascalia = { role = "caption" }

# Lists (if present)
# Elencoapallini = { role = "list-bullet" }
# Elenconumerato = { role = "list-ordered" }

[character_styles]
# Bold/Italic/Code handled by direct formatting detection
# Custom character styles can be mapped here

[table_styles]
Tabellanormale = { role = "default" }
# Add custom table styles as discovered

[theme]
major_font = "Inter"
minor_font = "Inter"
accent1 = "#D946EF"  # Fuchsia - Primary
accent2 = "#06B6D4"  # Cyan - Links
hyperlink = "#0891B2"
```

### Establish Style Conventions for Round-Trip Elegance

#### Heading Anchor Generation

When extracting from DOCX:
```
Heading text: "3.1.4 API Gateway Configuration"
Word bookmark: _Toc192197374
Semantic ID: api-gateway-configuration  (normalized)
AsciiDoc: [[api-gateway-configuration]]
          === API Gateway Configuration
```

When rendering to DOCX:
```
AsciiDoc: === API Gateway Configuration
StyleContract lookup: api-gateway-configuration → _Toc192197374
DOCX: <w:bookmarkStart w:name="_Toc192197374"/>
      <w:pStyle w:val="Titolo3"/>
```

#### Direct Formatting Rules

| DOCX Formatting | StyleContract Behavior | AsciiDoc Output |
|-----------------|------------------------|-----------------|
| Bold (`<w:b/>`) | Pass-through | `**bold**` |
| Italic (`<w:i/>`) | Pass-through | `_italic_` |
| Monospace font | Detect by font name | `` `code` `` |
| Underline | Warn (non-semantic) | `[.underline]#text#` |
| Color/highlight | Ignore or warn | Plain text |

#### Unmapped Style Handling

```
Extraction: Unknown style "CustomParagraph" → Warning, map to "body"
Rendering: Role "quote" has no mapping → Error, abort
```

This asymmetry is intentional: extraction is permissive (discover new styles), rendering is strict (enforce contract).

### Template Package Structure

```
templates/
├── utf8dok-essential/
│   ├── open_template.dotx       # Word template
│   ├── open_template.potx       # PowerPoint template (future)
│   ├── style-contract.toml      # Canonical StyleContract
│   ├── README.md                # Usage instructions
│   └── assets/
│       ├── logo-placeholder.svg
│       └── theme-colors.md
└── corporate/                   # .gitignored, user's private templates
    └── my-company.dotx
```

## Consequences

### Positive

1. **Real-world validation** - Template with Italian IDs proves locale-independence
2. **Single source of truth** - `style-contract.toml` is the authoritative mapping
3. **Round-trip testable** - Can verify DOCX → AsciiDoc → DOCX identity
4. **Corporate-ready** - Pattern works for any proprietary template
5. **Elegant degradation** - Unknown styles warn, don't crash

### Negative

1. **Italian naming may confuse** - Mitigated by StyleContract abstraction
2. **Additional artifact** - Must ship template + contract together
3. **Contract maintenance** - Template updates require contract updates

### Migration Path for Corporate Users

1. Extract StyleContract from corporate template: `utf8dok extract --contract corporate.docx`
2. Review and customize the generated `style-contract.toml`
3. Use for all future rendering: `utf8dok render doc.adoc --template corporate.dotx --contract style-contract.toml`

## Implementation Plan

### Phase 1: Canonical Contract
1. Create `templates/utf8dok-essential/` directory structure
2. Move `demo/open_template.dotx` to templates
3. Create `style-contract.toml` with complete mappings
4. Add validation tests

### Phase 2: Round-Trip Testing
1. Create sample AsciiDoc document
2. Render to DOCX with template + contract
3. Extract back to AsciiDoc
4. Verify semantic equivalence

### Phase 3: Contract Auto-Generation
1. Implement `utf8dok extract --contract <file.docx>`
2. Generate initial StyleContract from document analysis
3. User reviews and commits contract

## References

- [ADR-007: Style Mapping Architecture](./ADR-007-style-mapping-architecture.md)
- [TEMPLATE_SPEC.md](../../../demo/open_template/TEMPLATE_SPEC.md)
- [ECMA-376: Office Open XML](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/)
