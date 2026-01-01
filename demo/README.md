# UTF8DOK Demo

This directory contains a demonstration of UTF8DOK's dual-nature documentation capability.

## Demo Files

| File | Description |
|------|-------------|
| `utf8dok-overview.adoc` | Source AsciiDoc with dual-nature annotations |
| `utf8dok-overview.docx` | Generated DOCX (1.2MB, self-contained) |
| `template.dotx` | Open source template (Liberation Sans font, Rust logo) |

The DOCX was generated from the AsciiDoc source and contains the embedded source for lossless round-trips.

### Template Modifications

The template was derived from a corporate template with these open-source substitutions:
- **Fonts**: CocoEng → Liberation Sans
- **Logos**: Corporate logos → Rust logo (rust-lang.org)

## Try It

### Analyze Dual-Nature Structure

```bash
# Show both slide and document views
utf8dok dual-nature demo/utf8dok-overview.adoc

# Show only slide view (what would go into PPTX)
utf8dok dual-nature demo/utf8dok-overview.adoc --target slide

# Show only document view (what would go into DOCX)
utf8dok dual-nature demo/utf8dok-overview.adoc --target document

# JSON output for tooling
utf8dok dual-nature demo/utf8dok-overview.adoc --format json

# Validate dual-nature consistency
utf8dok dual-nature demo/utf8dok-overview.adoc --validate-only
```

### Check Document Quality

```bash
# Run validation checks
utf8dok check demo/utf8dok-overview.adoc
```

## Dual-Nature Annotations Used

The demo document uses these annotations:

| Annotation | Purpose |
|------------|---------|
| `[.slide]` | Section appears in both, optimized for slides |
| `[.document-only]` | Content appears only in DOCX |
| `:slide-bullets: 4` | Limit bullet points per slide |
| `:slide-layout: Title-And-Content` | Specify PowerPoint layout |
| `:slide-master: Corporate` | Template selection |

## Output Formats

### Currently Available

- **Dual-Nature Analysis**: Analyze and validate slide/document views
- **DOCX Rendering**: Generate Word documents (requires template)

### Coming Soon

- **PPTX Rendering**: Native PowerPoint generation from `[.slide]` content
- **HTML Export**: Web-ready documentation

## Rendering DOCX

The demo DOCX was generated using the included template:

```bash
# Render AsciiDoc to DOCX
utf8dok render demo/utf8dok-overview.adoc \
  --template demo/template.dotx \
  --output demo/utf8dok-overview.docx

# Verify round-trip (extracts embedded source)
utf8dok extract demo/utf8dok-overview.docx --output roundtrip/
diff demo/utf8dok-overview.adoc roundtrip/document.adoc
# ✓ Source preserved exactly!
```

### Creating Your Own Templates

```bash
# Extract from existing corporate document
utf8dok extract corporate-doc.docx --output my-template/

# Use the extracted template
utf8dok render my-doc.adoc --template my-template/template.dotx
```

## Sample Output

### Slide View Summary (8 slides)

1. Executive Summary - 4 key points
2. Corporate Documentation Challenges - 3 pain points
3. How UTF8DOK Works - 4-step workflow
4. Key Features - 4 capabilities
5. Architecture Overview - 4 components
6. CLI Commands - 4 main commands
7. Validation & Compliance - 4 features
8. What's Next - 4 roadmap items
9. Get Started Today - 3 action items

### Document View

Full detailed documentation with:
- Extended explanations for each section
- Code examples and command references
- Architecture tables and diagrams
- Complete quick-start guide

---

*This demo showcases UTF8DOK's vision: one source, multiple outputs.*
