# utf8dok Cover Page Specification

**Version:** 1.0.0
**Status:** Draft
**Date:** 2025-12-31
**Related ADR:** ADR-009

## 1. Overview

This specification defines how utf8dok handles cover pages in document generation and extraction. The design aligns with Asciidoctor PDF conventions for AsciiDoc compatibility while leveraging StyleContract for DOCX-specific styling.

## 2. Terminology

| Term | Definition |
|------|------------|
| **Cover Page** | The first page of a document containing title, author, and branding |
| **Title Page** | Synonym for cover page in some contexts |
| **Background Image** | Image placed behind text (z-order: behind) |
| **Cover Image** | Full-page image used as cover (may or may not have text overlay) |
| **StyleContract** | TOML configuration mapping semantic roles to Word styles |

## 3. AsciiDoc Document Attributes

### 3.1 Supported Attributes

utf8dok recognizes the following document attributes (compatible with Asciidoctor PDF):

| Attribute | Type | Description |
|-----------|------|-------------|
| `:front-cover-image:` | Image macro | Front cover image |
| `:back-cover-image:` | Image macro | Back cover image |
| `:title-page-background-image:` | Image macro | Title page background |
| `:title-logo-image:` | Image macro | Logo on title page |
| `:title-page:` | Flag | Force title page generation |
| `:notitle:` | Flag | Suppress title page |

### 3.2 Image Macro Syntax

```asciidoc
:front-cover-image: image:path/to/cover.png[attributes]
```

**Supported attributes:**

| Attribute | Values | Default | Description |
|-----------|--------|---------|-------------|
| `fit` | `cover`, `contain`, `fill`, `none` | `cover` | How image scales |
| `position` | `center`, `top`, `bottom` | `center` | Vertical alignment |
| `pdfwidth` | Length (e.g., `100%`, `6in`) | `100%` | Image width |

### 3.3 Examples

```asciidoc
= Document Title
:author: Jane Doe
:revnumber: 1.0.0
:revdate: 2025-12-31
:front-cover-image: image:covers/front.png[fit=cover]
:title-page-background-image: image:backgrounds/title-bg.png[]
:title-logo-image: image:logos/company.png[pdfwidth=2in,align=center,top=5%]
```

## 4. StyleContract Cover Section

### 4.1 Schema

The `[cover]` section in `style-contract.toml` defines cover page styling:

```toml
[cover]
# Layout mode
layout = "background"      # "background" | "block"

# Image configuration
image_fit = "cover"        # "cover" | "contain" | "fill" | "none"
image_position = "center"  # "center" | "top" | "bottom"

# Title element
[cover.title]
style = "TitoloCover"      # Word style ID (optional)
color = "FFFFFF"           # Hex RGB (overrides style)
font_size = 72             # Half-points (overrides style)
font_family = "Inter"      # Font name (overrides style)
bold = true                # Bold text
italic = false             # Italic text
top = "35%"                # Position from page top
align = "center"           # "left" | "center" | "right"

# Subtitle element
[cover.subtitle]
style = "SottotitoloCover"
color = "FFFFFF"
font_size = 32
italic = true
top = "45%"
align = "center"

# Authors element
[cover.authors]
style = "AutoreCover"
color = "FFFFFF"
font_size = 28
top = "75%"
align = "center"
content = "{author}"       # Template string

# Revision element
[cover.revision]
style = "RevisioneCover"
color = "FFFFFF"
font_size = 24
top = "80%"
align = "center"
delimiter = " | "
content = "Version {revnumber}{delimiter}{revdate}"
```

### 4.2 Template Variables

The `content` field supports these template variables:

| Variable | Source | Description |
|----------|--------|-------------|
| `{title}` | `:doctitle:` / `= Title` | Document title |
| `{subtitle}` | `:subtitle:` / `:description:` | Document subtitle |
| `{author}` | `:author:` | Author name(s) |
| `{email}` | `:email:` | Author email |
| `{revnumber}` | `:revnumber:` | Revision number |
| `{revdate}` | `:revdate:` | Revision date |
| `{revremark}` | `:revremark:` | Revision remark |
| `{delimiter}` | Local config | Delimiter string |

### 4.3 Position Units

The `top` field accepts:

| Format | Example | Description |
|--------|---------|-------------|
| Percentage | `35%` | Percentage of page height |
| Points | `200pt` | Absolute points from top |
| Inches | `2in` | Absolute inches from top |
| Centimeters | `5cm` | Absolute centimeters from top |
| EMU | `914400emu` | English Metric Units (OOXML native) |

## 5. utf8dok.toml Configuration

### 5.1 Project-Level Cover Defaults

```toml
[cover]
enabled = true
default_image = "assets/cover.png"
layout = "background"

[cover.title]
color = "FFFFFF"
font_size = 72
top = "35%"
```

### 5.2 Configuration Precedence

1. **Document attributes** - Highest priority
2. **CLI options** (`--cover`)
3. **utf8dok.toml** `[cover]` section
4. **StyleContract** `[cover]` section
5. **Built-in defaults** - Lowest priority

## 6. OOXML Implementation

### 6.1 Background Image (wp:anchor)

Cover images with `layout = "background"` use anchored drawing:

```xml
<w:p>
  <w:r>
    <w:drawing>
      <wp:anchor behindDoc="1" relativeHeight="0" ...>
        <wp:positionH relativeFrom="page">
          <wp:align>center</wp:align>
        </wp:positionH>
        <wp:positionV relativeFrom="page">
          <wp:posOffset>0</wp:posOffset>
        </wp:positionV>
        <wp:extent cx="{width}" cy="{height}"/>
        <wp:wrapNone/>
        <!-- pic:pic element -->
      </wp:anchor>
    </w:drawing>
  </w:r>
</w:p>
```

Key attributes:
- `behindDoc="1"` - Places image behind text
- `relativeHeight="0"` - Lowest z-order
- `<wp:wrapNone/>` - Text flows over image

### 6.2 Text Positioning

Cover text elements use absolute positioning via frame properties:

```xml
<w:p>
  <w:pPr>
    <w:framePr w:vAnchor="page" w:y="{top_twips}"/>
    <w:jc w:val="{align}"/>
  </w:pPr>
  <w:r>
    <w:rPr>
      <w:color w:val="{color}"/>
      <w:sz w:val="{font_size}"/>
      <w:b/> <!-- if bold -->
    </w:rPr>
    <w:t>{text}</w:t>
  </w:r>
</w:p>
```

### 6.3 CoverPageProperties (Custom XML)

Metadata stored in OOXML-standard format:

**Path:** `customXml/item1.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<CoverPageProperties xmlns="http://schemas.microsoft.com/office/2006/coverPageProps">
  <PublishDate>{revdate}</PublishDate>
  <Abstract>{description}</Abstract>
</CoverPageProperties>
```

**Path:** `customXml/itemProps1.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<ds:datastoreItem xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"
    ds:itemID="{uuid}">
  <ds:schemaRefs>
    <ds:schemaRef ds:uri="http://schemas.microsoft.com/office/2006/coverPageProps"/>
  </ds:schemaRefs>
</ds:datastoreItem>
```

## 7. Default Configuration

When no cover configuration is provided:

```toml
[cover]
layout = "background"
image_fit = "cover"
image_position = "center"

[cover.title]
color = "FFFFFF"
font_size = 72
bold = true
top = "35%"
align = "center"

[cover.subtitle]
color = "FFFFFF"
font_size = 32
italic = true
top = "45%"
align = "center"

[cover.authors]
color = "FFFFFF"
font_size = 28
top = "75%"
align = "center"
content = "{author}"

[cover.revision]
color = "FFFFFF"
font_size = 24
top = "80%"
align = "center"
content = "Version {revnumber} | {revdate}"
```

## 8. CLI Reference

```bash
# Use document attributes for cover
utf8dok render document.adoc --output out.docx

# Override cover image
utf8dok render document.adoc --cover custom-cover.png --output out.docx

# Disable cover page
utf8dok render document.adoc --no-cover --output out.docx

# Specify cover config file
utf8dok render document.adoc --cover-config cover.toml --output out.docx
```

## 9. Round-Trip Behavior

### 9.1 Extraction

When extracting from DOCX:
1. Detect cover page by structure (background image + positioned text)
2. Extract cover image to `media/cover.{ext}`
3. Generate `:front-cover-image:` attribute
4. Capture text positions in StyleContract `[cover]` section

### 9.2 Rendering

When rendering to DOCX:
1. Parse `:front-cover-image:` attribute
2. Load cover configuration from StyleContract
3. Generate OOXML with configured styling
4. Embed cover configuration in `utf8dok/cover-config.toml`

## 10. Compatibility Matrix

| Feature | Asciidoctor PDF | utf8dok | Notes |
|---------|-----------------|---------|-------|
| `:front-cover-image:` | Yes | Yes | Compatible |
| `:back-cover-image:` | Yes | Planned | Not yet implemented |
| `:title-page-background-image:` | Yes | Yes | Compatible |
| `:title-logo-image:` | Yes | Planned | Not yet implemented |
| Theme YAML | Yes | StyleContract TOML | Different format, similar concept |
| PDF output | Yes | No | DOCX only |

## 11. References

- [Asciidoctor PDF Covers](https://docs.asciidoctor.org/pdf-converter/latest/theme/cover/)
- [Asciidoctor PDF Title Page](https://docs.asciidoctor.org/pdf-converter/latest/theme/title-page/)
- [MS-OI29500 CoverPageProperties](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/f13db469-a762-4a03-9fbb-b4d0fc4affc7)
- [OOXML Drawing Anchor](http://officeopenxml.com/drwPicFloating-anchor.php)
- ADR-009: Cover Page Architecture
