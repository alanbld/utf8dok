# ADR-006: Round-Trip Rendition Fidelity Improvements

## Status
Proposed

## Context

Round-trip testing of `SWP Application Architecture.docx` revealed fidelity gaps between extraction and regeneration:

| Element     | Original | Regenerated | Fidelity |
|-------------|----------|-------------|----------|
| Paragraphs  | 933      | 833         | 89.2%    |
| Tables      | 3        | 3           | 100%     |
| Drawings    | 37       | 27          | 72.9%    |
| Hyperlinks  | 69       | 52          | 75.3%    |
| Media       | 40       | 40          | 100%     |
| Bold        | 83       | 83          | 100%     |

### Recent Improvements

1. **Run Merging** (a4eea5a): Fixed malformed AsciiDoc markup from consecutive runs
2. **Text Box Extraction** (7f6ffa5): Extract text from `<w:txbxContent>` in DrawingML
3. **Bookmark Support** (7f6ffa5): Preserve anchors via `[[name]]` syntax

### Remaining Gaps Analysis

#### 1. Paragraph Loss (10.8% = 100 paragraphs)

Root causes:
- **Text in SmartArt diagrams**: `<dgm:cxnSp>` elements with `<a:t>` content
- **Text in grouped shapes**: `<wpg:wgp>` (WordprocessingML groups)
- **Text in chart labels**: `<c:txPr>` elements
- **Equation content**: OMML `<m:oMath>` blocks render as paragraphs

#### 2. Drawing Loss (27.1% = 10 drawings)

Root causes:
- **SmartArt diagrams**: Complex diagram markup not fully parsed
- **Charts**: `<c:chart>` references to external chart XML
- **Shape groups**: `<wpg:wgp>` with multiple nested shapes
- **Inline shapes**: `<w:pict>` VML fallback content

#### 3. Hyperlink Loss (24.7% = 17 hyperlinks)

Root causes:
- **TOC internal links**: `w:anchor="_Toc..."` links to heading bookmarks
- **Cross-reference links**: `w:anchor="_Ref..."` for internal references
- **Field codes**: `HYPERLINK` field codes with `\l` switch for internal links

### Architectural Constraints

1. **AsciiDoc Semantic Mapping**: Not all OOXML elements have AsciiDoc equivalents
2. **Template Injection Model**: We inject content, not structure; some elements must remain as placeholders
3. **Lossless vs Lossy**: Some transformations are inherently lossy (e.g., SmartArt → text)

## Decision

Implement a phased approach to achieve 95%+ fidelity on text content:

### Phase 1: Enhanced Text Extraction (Target: 95% paragraph fidelity)

Extract text from all DrawingML containers:
```
<wps:wsp>     → Shape text boxes (already done via txbxContent)
<wpg:wgp>     → Group shapes (recurse into children)
<a:graphic>   → Generic DrawingML graphics
<dgm:*>       → Diagram elements
<c:chart>     → Chart title/labels only
```

### Phase 2: Hyperlink Preservation (Target: 90% hyperlink fidelity)

1. **TOC Link Handling**: Extract `w:anchor` values and generate AsciiDoc cross-references
2. **Internal Link Syntax**: Map `w:anchor="_Toc..."` → `<<section-title>>`
3. **External Link Preservation**: Already working via relationship IDs

### Phase 3: Structural Fidelity (Target: 85% drawing fidelity)

1. **SmartArt Text**: Extract text content as admonition blocks or nested lists
2. **Chart Integration**: Extract chart data, regenerate via Kroki/Mermaid
3. **Shape Placeholders**: Emit `[shape:type]` markers for manual review

### Implementation Markers

```asciidoc
// Text from SmartArt diagram
[NOTE]
====
Process step 1
Process step 2
====

// Chart placeholder (data extracted)
[chart,type=bar]
----
Category A, 10
Category B, 20
----

// Shape with text (round-trip preserved)
[[shape-1]]
[.shape]
Text content from shape
```

## Consequences

### Positive

1. **95%+ text fidelity**: All readable text preserved in AsciiDoc
2. **Semantic preservation**: Diagrams become structured AsciiDoc blocks
3. **Round-trip safety**: Placeholders prevent data loss on regeneration
4. **Progressive enhancement**: Each phase adds value independently

### Negative

1. **Visual fidelity loss**: SmartArt layout not preserved (text only)
2. **Implementation complexity**: DrawingML parsing is deeply nested
3. **Test coverage burden**: Each element type needs dedicated tests
4. **Performance impact**: Additional XML traversal per document

## TDD Test Specification

### Unit Tests (utf8dok-ooxml)

```rust
// test_extract_shape_group_text
#[test]
fn test_shape_group_text_extraction() {
    // Given: DOCX with <wpg:wgp> containing multiple shapes with text
    // When: Extract document
    // Then: All shape text appears in output paragraphs
}

// test_extract_smartart_text
#[test]
fn test_smartart_text_extraction() {
    // Given: DOCX with SmartArt diagram containing text nodes
    // When: Extract document
    // Then: Text extracted as structured content (list or admonition)
}

// test_extract_chart_labels
#[test]
fn test_chart_label_extraction() {
    // Given: DOCX with embedded chart having title and axis labels
    // When: Extract document
    // Then: Chart title and labels appear in output
}

// test_toc_hyperlink_to_xref
#[test]
fn test_toc_hyperlink_to_crossref() {
    // Given: DOCX with TOC containing hyperlinks to headings
    // When: Extract document
    // Then: Links converted to <<anchor>> cross-references
}

// test_internal_anchor_preservation
#[test]
fn test_internal_anchor_roundtrip() {
    // Given: DOCX with internal bookmarks and links
    // When: Extract → Render → Extract
    // Then: All anchors and links preserved
}
```

### Integration Tests (utf8dok-cli)

```rust
// test_swp_roundtrip_fidelity
#[test]
fn test_swp_architecture_roundtrip() {
    // Given: SWP Application Architecture.docx
    // When: extract → render
    // Then: Paragraph fidelity >= 95%
    // And: Hyperlink fidelity >= 90%
    // And: Media fidelity == 100%
}

// test_complex_document_roundtrip
#[test]
fn test_complex_document_with_diagrams() {
    // Given: Document with SmartArt, charts, and grouped shapes
    // When: extract → render
    // Then: All text content preserved
    // And: Placeholders generated for complex graphics
}
```

### Property-Based Tests

```rust
// test_text_preservation_invariant
#[proptest]
fn text_never_lost_in_roundtrip(doc: ArbitraryDocx) {
    // Given: Any valid DOCX
    // When: extract → render
    // Then: extracted_text ⊆ roundtrip_text
    // (All original text is present in roundtrip)
}
```

## References

- [ADR-005: Image Support](./ADR-005-image-support.md)
- [ECMA-376 Part 1: DrawingML](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/)
- [Office Open XML Structure](https://docs.microsoft.com/en-us/office/open-xml/)
- Related commits: a4eea5a, 7f6ffa5, 77a8b03
