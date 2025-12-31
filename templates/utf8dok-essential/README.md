# utf8dok Essential Template

An open-source Word/PowerPoint template for utf8dok, derived from a real corporate template.

## Overview

This template provides:
- **Professional typography** using [Inter](https://rsms.me/inter/) font (OFL license)
- **WCAG-compliant colors** with a vibrant Fuchsia/Cyan/Violet palette
- **Complete heading hierarchy** (9 levels)
- **Rich table styles** with color variants
- **Quote, caption, and list styles**

## Files

| File | Description |
|------|-------------|
| `open_template.dotx` | Word template file |
| `style-contract.toml` | StyleContract mapping (Italian IDs → semantic roles) |

### Cover Images

| File | Description | Use Case |
|------|-------------|----------|
| `cover-screen.png` | Full-color 3D geometric design | Screen, digital PDFs |
| `cover-print.svg` | Light background, outline geometry | Print (saves ink) |
| `cover-print-dark.svg` | Dark header band with wave accents | Print (minimal ink) |

**Design Philosophy**:
- **Screen**: Vibrant 3D glass cubes with Fuchsia/Cyan/Violet edge highlights on purple gradient
- **Print**: White background with outline-only geometry using same color accents (90%+ ink savings)

## Style IDs

The template uses Italian style IDs from its corporate heritage:

| Style ID | English Name | Semantic Role |
|----------|--------------|---------------|
| `Titolo1` | Heading 1 | h1 |
| `Titolo2` | Heading 2 | h2 |
| `Normale` | Normal | body |
| `Citazione` | Quote | quote |
| `Didascalia` | Caption | caption |

See `style-contract.toml` for the complete mapping.

## Usage

### Rendering AsciiDoc to DOCX

```bash
utf8dok render document.adoc \
  --template templates/utf8dok-essential/open_template.dotx \
  --contract templates/utf8dok-essential/style-contract.toml \
  --output document.docx
```

### Extracting from DOCX

```bash
utf8dok extract document.docx \
  --contract templates/utf8dok-essential/style-contract.toml \
  --output extracted/
```

## Theme Colors

| Role | Color | Hex |
|------|-------|-----|
| Primary | Fuchsia | `#D946EF` |
| Links | Cyan | `#06B6D4` |
| Accents | Violet | `#8B5CF6` |
| Warnings | Amber | `#F59E0B` |
| Alerts | Orange | `#EA580C` |

## License

- **Template structure**: Apache 2.0 (same as utf8dok)
- **Fonts**: SIL Open Font License
- **Colors**: Public domain

## See Also

- [ADR-008: Essential Platform Template](../../architecture/decisions/adr/ADR-008-essential-platform-template.md)
- [ADR-007: Style Mapping Architecture](../../architecture/decisions/adr/ADR-007-style-mapping-architecture.md)
