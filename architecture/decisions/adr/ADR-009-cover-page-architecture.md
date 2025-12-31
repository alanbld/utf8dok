# ADR-009: Cover Page Architecture

## Status
Accepted

## Context

utf8dok generates DOCX documents with cover pages, but the current implementation hardcodes:
- Text colors (white `#FFFFFF`)
- Font sizes (36pt, 16pt, 14pt, 12pt)
- Vertical positioning (spacer paragraph counts)
- Image dimensions (fixed EMU values)

This approach violates the principle of configuration-over-code and makes cover pages inflexible across different templates. Corporate templates require different layouts, colors, and positioning.

### Existing Standards Review

| Standard | Mechanism | Relevance |
|----------|-----------|-----------|
| **Asciidoctor PDF** | Document attributes + Theme YAML | De facto AsciiDoc standard for cover configuration |
| **OOXML (ISO/IEC 29500)** | `CoverPageProperties` Custom XML | Native DOCX cover metadata storage |
| **DocBook 5.1** | `<cover>` element with `mediaobject` | Semantic cover structure |
| **Eclipse AsciiDoc WG** | Specification in progress | Future standard (not yet finalized) |

### Asciidoctor PDF Attributes (Adopted)

```asciidoc
:front-cover-image: image:cover.png[]
:back-cover-image: image:back.png[]
:title-page-background-image: image:bg.png[]
:title-logo-image: image:logo.png[top=25%,align=center]
```

### Asciidoctor PDF Theme Keys (Reference)

```yaml
cover:
  front:
    image: image:cover.pdf[]
  back:
    image: image:back.pdf[]

title-page:
  background-image: image:bg.png[]
  logo:
    image: image:logo.png[]
    top: 10%
    align: center
  title:
    top: 40%
    font-size: 36
    font-color: FFFFFF
  subtitle:
    font-size: 18
    font-color: CCCCCC
  authors:
    font-size: 14
    content: "{author}"
  revision:
    font-size: 12
    delimiter: " | "
```

## Decision

### 1. Adopt Asciidoctor PDF Attribute Names

For AsciiDoc source files, use the same attribute names as Asciidoctor PDF:

```asciidoc
= Document Title
:front-cover-image: image:cover.png[]
:title-page-background-image: image:title-bg.png[]
:title-logo-image: image:logo.png[top=10%,align=center]
```

### 2. Extend StyleContract for Cover Styling

Add `[cover]` section to `style-contract.toml`:

```toml
[cover]
# Layout mode: "background" (image behind text) or "block" (image above text)
layout = "background"

# Image positioning
image_fit = "cover"        # cover, contain, fill, none
image_position = "center"  # center, top, bottom

[cover.title]
style = "TitoloCover"      # Word style ID (optional)
color = "FFFFFF"           # Hex color (overrides style)
font_size = 72             # Half-points (overrides style)
top = "35%"                # Vertical position from top
align = "center"           # left, center, right

[cover.subtitle]
style = "SottotitoloCover"
color = "FFFFFF"
font_size = 32
top = "45%"
align = "center"

[cover.authors]
style = "AutoreCover"
color = "FFFFFF"
font_size = 28
top = "75%"
align = "center"
content = "{author}"       # Template: {author}, {email}, {name}

[cover.revision]
style = "RevisioneCover"
color = "FFFFFF"
font_size = 24
top = "80%"
align = "center"
delimiter = " | "          # Between version and date
content = "Version {revnumber} | {revdate}"
```

### 3. Support OOXML CoverPageProperties

Store cover metadata in OOXML-compliant format:

```
docx.zip/
  customXml/
    item1.xml              # CoverPageProperties
    itemProps1.xml         # Properties
```

Schema: `http://schemas.microsoft.com/office/2006/coverPageProps`

### 4. Configuration Hierarchy (Precedence)

1. **Document attributes** (`:front-cover-image:`) - highest priority
2. **utf8dok.toml** `[cover]` section - project defaults
3. **StyleContract** `[cover]` section - template-specific
4. **Built-in defaults** - fallback

### 5. CLI Integration

```bash
# Explicit cover image (overrides document attribute)
utf8dok render doc.adoc --cover cover.png --output out.docx

# Use document attributes and config
utf8dok render doc.adoc --output out.docx
```

## Consequences

### Positive

1. **Standards Compliance**: Adopts Asciidoctor PDF naming convention
2. **Configuration-Driven**: Cover layout defined in TOML, not code
3. **Template Flexibility**: Different templates can define different cover styles
4. **Round-Trip Safe**: Cover configuration preserved in extraction
5. **OOXML Compliant**: Uses standard `CoverPageProperties` for metadata

### Negative

1. **Learning Curve**: Users must understand StyleContract cover section
2. **Migration**: Existing documents may need updated configurations
3. **Complexity**: More configuration options to maintain

## References

- [Asciidoctor PDF Cover Configuration](https://docs.asciidoctor.org/pdf-converter/latest/theme/cover/)
- [Asciidoctor PDF Title Page Keys](https://docs.asciidoctor.org/pdf-converter/latest/theme/title-page/)
- [MS-OI29500 Cover Page Properties](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/f13db469-a762-4a03-9fbb-b4d0fc4affc7)
- [DocBook 5.1 cover Element](https://tdg.docbook.org/tdg/5.1/cover.html)
- [Eclipse AsciiDoc Working Group](https://asciidoc-wg.eclipse.org/)
- ADR-007: Style Mapping Architecture
- ADR-008: Essential Platform Template
