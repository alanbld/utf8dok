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

### 8. Tables

Tables use `|===` as delimiters (start and end).

```asciidoc
|===
| Header 1
| Header 2

| Cell A1
| Cell A2

| Cell B1
| Cell B2
|===
```

**Syntax Rules:**

- **Delimiter**: `|===` marks the start and end of a table
- **Cells**: Lines starting with `|` (pipe) followed by content
- **Rows**: For MVP, each cell line becomes one cell; cells are grouped into rows
  - Simple logic: Blank lines or consistent cell count determines row boundaries
  - Fallback: Each `|` line is one cell, grouped sequentially

**AST Mapping**: `Block::Table { rows: [...], ... }`

```rust
Block::Table {
    rows: [
        TableRow { cells: [TableCell { content: [...] }], is_header: true },
        TableRow { cells: [TableCell { content: [...] }], is_header: false },
    ],
    ...
}
```

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

### 9. Literal Blocks and Diagrams

Literal blocks are delimited by `----` (4 or more dashes) and preserve content verbatim.

```asciidoc
----
This is literal text.
No formatting is applied.
----
```

**AST Mapping**: `Block::Literal { content, language: None, ... }`

#### Block Attributes

Blocks can have attributes specified in square brackets on the preceding line.
Attributes accumulate until a block starts.

```asciidoc
[source,rust]
----
fn main() {
    println!("Hello");
}
----
```

**AST Mapping**: `Block::Literal { content, language: Some("rust"), ... }`

#### Diagram Blocks

Diagram blocks use block attributes to specify the diagram type.
Supported diagram types include: `mermaid`, `plantuml`, `graphviz`, `d2`, etc.

```asciidoc
[mermaid]
----
graph TD;
    A-->B;
    B-->C;
----
```

**Syntax Rules:**

1. **Block Attributes**: Lines matching `[...]` (square brackets) are block attributes
2. **Attribute Accumulation**: Attributes accumulate until the next block starts
3. **Delimiter**: `----` (4+ dashes) delimits a literal block
4. **Style Mapping**: First attribute value becomes the `style_id` or `language`

**AST Mapping**: `Block::Literal { content, language: Some("mermaid"), style_id: Some("mermaid"), ... }`

**Parsing Rules:**

| Attribute | AST Field |
|-----------|-----------|
| `[mermaid]` | `style_id: Some("mermaid")` |
| `[plantuml]` | `style_id: Some("plantuml")` |
| `[source,rust]` | `language: Some("rust")` |
| `[source]` | `language: None` |

### 10. Cross-References

Internal cross-references link to anchors within the document.

```asciidoc
See <<section-id,Section Title>> for more details.
```

**AST Mapping**: `Inline::Link { url: "#section-id", text: [Text("Section Title")] }`

Shorthand form (uses anchor as text):

```asciidoc
See <<section-id>> for details.
```

## Out of Scope (MVP)

The following features are **not** in scope for the MVP:

- Admonitions (NOTE, WARNING, etc.)
- Images
- External Links
- Includes
- Conditionals (ifdef)
- Footnotes

These will be added in subsequent iterations.

## Implementation Notes

1. Use `pest` (PEG parser) as specified in ADR-003
2. Parser should be streaming-friendly for large documents
3. Line numbers should be tracked for error reporting
