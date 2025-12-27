# Render Phase Specification

> MVP specification for the AsciiDoc Parser in utf8dok-core.

## Overview

This document defines the minimum viable AsciiDoc syntax that the parser must support
for the **Render** workflow (AsciiDoc → DOCX).

## Syntax Elements

### 1. Document Title

The document title is a level-0 heading at the start of the document.

```asciidoc
= Document Title
```

**AST Mapping**: Sets `Document.metadata.title`

### 2. Document Attributes

Key-value pairs that set document metadata or configuration.

```asciidoc
:author: John Doe
:version: 1.0
:toc: left
```

**AST Mapping**: Stored in `Document.metadata.attributes`

### 3. Section Headings

Headings use `=` prefix, where the count determines the level.

```asciidoc
== Level 1 Heading
=== Level 2 Heading
==== Level 3 Heading
===== Level 4 Heading
```

**AST Mapping**: `Block::Heading { level, text, ... }`

| Syntax | Level |
|--------|-------|
| `==`   | 1     |
| `===`  | 2     |
| `====` | 3     |
| `=====`| 4     |

### 4. Paragraphs

Plain text separated by one or more blank lines.

```asciidoc
This is the first paragraph.

This is the second paragraph.
```

**AST Mapping**: `Block::Paragraph { inlines, ... }`

### 5. Inline Formatting

| Syntax | Meaning | AST Mapping |
|--------|---------|-------------|
| `*bold*` | Bold text | `Inline::Format(Bold, ...)` |
| `_italic_` | Italic text | `Inline::Format(Italic, ...)` |
| `` `mono` `` | Monospace | `Inline::Format(Monospace, ...)` |

Formatting can be nested:

```asciidoc
This is *bold _and italic_* text.
```

### 6. Unordered Lists

Lines starting with `*` followed by space.

```asciidoc
* First item
* Second item
** Nested item
* Third item
```

**AST Mapping**: `Block::List { list_type: Unordered, items: [...] }`

Nesting is indicated by additional `*` characters.

### 7. Ordered Lists

Lines starting with `.` followed by space.

```asciidoc
. First step
. Second step
.. Sub-step
. Third step
```

**AST Mapping**: `Block::List { list_type: Ordered, items: [...] }`

## Parser Requirements

### Input

- UTF-8 encoded text
- Unix-style line endings (`\n`) preferred, but `\r\n` should be handled

### Output

- `utf8dok_ast::Document` on success
- `anyhow::Error` with context on failure

### Error Handling

- Unknown syntax should be treated as plain paragraph text (graceful degradation)
- Unclosed formatting markers should include the marker as literal text

## Test Cases

### Minimal Document

```asciidoc
= Test Document
:version: 1.0

== Section One

Hello *world*.
```

**Expected AST:**

```rust
Document {
    metadata: DocumentMeta {
        title: Some("Test Document"),
        attributes: { "version": "1.0" },
        ..
    },
    blocks: [
        Block::Heading { level: 1, text: [Text("Section One")] },
        Block::Paragraph {
            inlines: [
                Text("Hello "),
                Format(Bold, Text("world")),
                Text("."),
            ]
        },
    ],
}
```

### List Document

```asciidoc
== Shopping List

* Apples
* Oranges
* Bananas
```

**Expected AST:**

```rust
Document {
    blocks: [
        Block::Heading { level: 1, text: [Text("Shopping List")] },
        Block::List {
            list_type: Unordered,
            items: [
                ListItem { content: [Paragraph([Text("Apples")])] },
                ListItem { content: [Paragraph([Text("Oranges")])] },
                ListItem { content: [Paragraph([Text("Bananas")])] },
            ],
        },
    ],
}
```

## Out of Scope (MVP)

The following features are **not** in scope for the MVP:

- Tables
- Admonitions (NOTE, WARNING, etc.)
- Code blocks with language
- Images
- Links
- Includes
- Conditionals (ifdef)
- Cross-references
- Footnotes

These will be added in subsequent iterations.

## Implementation Notes

1. Use `pest` (PEG parser) as specified in ADR-003
2. Parser should be streaming-friendly for large documents
3. Line numbers should be tracked for error reporting
