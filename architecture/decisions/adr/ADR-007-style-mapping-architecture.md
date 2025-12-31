# ADR-007: Style Mapping Architecture for DOCX ↔ AsciiDoc Round-Trip

## Status
Accepted

## Context

### The Fundamental Impedance Mismatch

DOCX and AsciiDoc represent documents with fundamentally different philosophies:

| Aspect | DOCX | AsciiDoc |
|--------|------|----------|
| Paradigm | Presentation-first | Semantic-first |
| Layout | Implicit, context-dependent | Backend-delegated |
| Styles | Inheritance chains, themes, direct formatting | Roles, attributes |
| Mutability | Highly mutable by users | Deterministic |

A direct DOCX → AsciiDoc → DOCX round-trip will always have fidelity gaps because:

1. **DOCX styles are not just named styles** - they include inheritance chains, conditional formatting, theme-dependent fonts, section-scoped overrides, and layout rules resolved at render time.

2. **Many DOCX "styles" are direct formatting artifacts** - applied opportunistically by users or templates, not semantic constructs.

3. **DOCX documents often lack semantic information** - insufficient to reconstruct a clean text model.

### Why Extending AsciiDoc Syntax Is Wrong

The temptation: "If AsciiDoc were more typesetting-aware, I could preserve DOCX fidelity."

The reality: Pushing layout intelligence into AsciiDoc syntax would:
- Destroy multi-output portability
- Recreate troff/LaTeX complexity
- Lock to DOCX semantics
- Make documents brittle and unreadable

This is the trap that troff fell into. We must avoid it.

### The Real Gap

AsciiDoc already has sufficient expressive power (roles, attributes, block types, structural nodes). The gap is:

**There is no canonical, lossless semantic model for DOCX styles.**

We cannot round-trip what was never semantically well-defined.

## Decision

### Introduce a Style Mapping Layer

Replace the mental model:

```
DOCX → AsciiDoc → DOCX
```

With:

```
DOCX → (Semantic AST + StyleMap) → AsciiDoc → (StyleMap) → DOCX
```

### StyleMap as First-Class Artifact

The StyleMap:
- Lives alongside the AST
- Is serializable (TOML)
- Survives the full round trip unchanged
- Is part of document identity, not a rendering detail

### StyleMap Responsibilities

**Must capture:**
- Word paragraph styles (`w:pStyle`) → semantic roles
- Character styles (`w:rStyle`) → semantic roles
- TOC/heading hierarchy with stable anchors
- Hyperlink intent (external vs internal)
- Bookmark and anchor identity mapping
- Table style references
- Theme-derived defaults (font family class, size class, spacing class)

**Must NOT capture:**
- Absolute measurements (points, inches)
- Page geometry
- Word-specific auto-layout quirks

### Concrete Mapping Rules

#### Paragraph Styles

```
DOCX:           AsciiDoc:           StyleMap:
─────────────────────────────────────────────────
Heading 1   →   == Title            paragraph_styles.Heading1 = "h1"
Heading 2   →   === Section         paragraph_styles.Heading2 = "h2"
Body Text   →   Normal paragraph    paragraph_styles.BodyText = "body"
Quote       →   [role=quote]        paragraph_styles.Quote = "quote"
Code        →   [source]            paragraph_styles.Code = "code"
```

#### Bookmark/Anchor Normalization

```
DOCX bookmark:              Semantic anchor:        AsciiDoc:
────────────────────────────────────────────────────────────
_Toc192197374           →   introduction        →   [[introduction]]
_Toc192197375           →   purpose-and-scope   →   [[purpose-and-scope]]
_Ref123456              →   figure-1            →   [[figure-1]]
custom_bookmark         →   custom-bookmark     →   [[custom-bookmark]]
```

#### Hyperlink Intent

```
DOCX:                           StyleMap:                       AsciiDoc:
──────────────────────────────────────────────────────────────────────────
w:hyperlink r:id="rId5"     →   external: "https://..."     →   link:url[text]
w:hyperlink w:anchor="_Toc" →   internal: toc-entry         →   <<anchor,text>>
HYPERLINK field \l "ref"    →   internal: ref               →   <<ref>>
```

### StyleMap Schema

```rust
/// Style mapping contract between DOCX and AsciiDoc
pub struct StyleMap {
    /// Document identity
    pub source_file: Option<String>,
    pub created: DateTime<Utc>,

    /// Paragraph style mappings (Word style ID → semantic role)
    pub paragraph_styles: HashMap<String, ParagraphStyleMapping>,

    /// Character style mappings (Word style ID → semantic role)
    pub character_styles: HashMap<String, CharacterStyleMapping>,

    /// Anchor registry (Word bookmark → semantic anchor)
    pub anchors: HashMap<String, AnchorMapping>,

    /// Table style mappings
    pub table_styles: HashMap<String, TableStyleMapping>,

    /// Theme defaults extracted from document
    pub theme: ThemeDefaults,
}

pub struct ParagraphStyleMapping {
    pub word_style_id: String,
    pub semantic_role: String,
    pub heading_level: Option<u8>,
    pub is_list: bool,
    pub list_type: Option<ListType>,
}

pub struct AnchorMapping {
    pub word_bookmark: String,
    pub semantic_id: String,
    pub anchor_type: AnchorType,  // Toc, Ref, Heading, UserDefined
    pub target_heading: Option<String>,
}
```

### Serialization Format (TOML)

```toml
# style-map.toml - Style contract for round-trip fidelity

[meta]
source = "SWP Application Architecture.docx"
created = "2025-01-15T10:30:00Z"

[paragraph_styles]
Heading1 = { role = "h1", heading_level = 1 }
Heading2 = { role = "h2", heading_level = 2 }
Titolo1 = { role = "h1", heading_level = 1 }  # Italian template
Normale = { role = "body" }
Quote = { role = "quote" }

[character_styles]
Strong = { role = "strong" }
Emphasis = { role = "emphasis" }
CodeChar = { role = "code" }

[anchors]
_Toc192197374 = { semantic_id = "overview", type = "heading" }
_Toc192197375 = { semantic_id = "purpose-and-scope", type = "heading" }
_Ref123456 = { semantic_id = "figure-architecture", type = "reference" }

[table_styles]
TableGrid = { role = "default" }
Elencotab4 = { role = "corporate-table" }

[theme]
heading_font_family = "Calibri Light"
body_font_family = "Calibri"
base_font_size = 11
```

### Integration Points

#### Extraction Flow

```
DOCX Input
    │
    ▼
┌─────────────────────────────────┐
│  Document Parser (document.rs)  │
│  - Extracts content             │
│  - Populates StyleMap           │
│  - References style keys        │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Style Analyzer                 │
│  - Normalizes anchors           │
│  - Derives semantic IDs         │
│  - Detects direct formatting    │
│  - Warns on unsupported styles  │
└─────────────────────────────────┘
    │
    ├──────────────────┐
    ▼                  ▼
document.adoc     style-map.toml
```

#### Rendering Flow

```
document.adoc + style-map.toml
    │
    ▼
┌─────────────────────────────────┐
│  AsciiDoc Parser                │
│  - Builds semantic AST          │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  Style Resolver                 │
│  - Loads StyleMap               │
│  - Maps roles → Word styles     │
│  - Resolves anchors             │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  DOCX Writer                    │
│  - Applies styles deterministically │
│  - Fails fast on missing mappings   │
└─────────────────────────────────┘
    │
    ▼
output.docx
```

## Consequences

### Positive

1. **Clean separation of concerns** - AsciiDoc stays semantic, styles are data
2. **Deterministic round-trips** - Same input always produces same output
3. **Multi-format portability** - StyleMap can target HTML, PDF, DOCX
4. **Corporate compliance** - Style profiles become enforceable contracts
5. **Transparent failures** - Missing mappings fail explicitly, not silently
6. **Template independence** - Different corporate templates = different StyleMaps

### Negative

1. **Additional artifact to manage** - style-map.toml alongside document.adoc
2. **Cannot preserve arbitrary DOCX formatting** - By design, not accident
3. **Initial extraction requires style analysis** - Adds complexity
4. **Users must understand the model** - Documentation burden

### Explicitly Accepted Constraints

1. **Round-trip success = stylistic equivalence under a controlled profile**
   - NOT bit-for-bit DOCX equivalence

2. **Direct formatting is normalized or flagged**
   - Random bold text → either normalize or warn
   - Do not silently absorb chaos

3. **Unsupported constructs fail early**
   - Better to error than to corrupt

## Implementation Plan

### Phase 2a: StyleMap Foundation
1. Create `style_map.rs` module
2. Define core structs
3. Implement TOML serialization
4. Add to extraction output

### Phase 2b: Anchor Normalization
1. Extract all bookmarks during parsing
2. Derive semantic IDs from heading text
3. Build bidirectional anchor registry
4. Emit AsciiDoc cross-references

### Phase 2c: Style-Aware Rendering
1. Load StyleMap during render
2. Map roles back to Word styles
3. Restore bookmarks with original IDs
4. Generate internal hyperlinks

### Phase 2d: Validation & Diagnostics
1. Warn on unmapped styles
2. Error on missing required mappings
3. Report direct formatting violations
4. Suggest style normalization

## References

- [ADR-006: Rendition Fidelity](./ADR-006-rendition-fidelity.md)
- [ECMA-376 Part 1: Styles](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/)
- DocBook/DITA style separation patterns
- Sphinx/Antora theming architecture
