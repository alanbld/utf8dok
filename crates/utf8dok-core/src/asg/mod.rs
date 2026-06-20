//! Eclipse AsciiDoc ASG (Abstract Semantic Graph) emitter.
//!
//! This module is the foundation of utf8dok's Eclipse AsciiDoc TCK adapter
//! (see ADR-004). The TCK validates a processor by comparing the JSON-encoded
//! ASG it produces against expected fixtures, so the node types here are shaped
//! to serialize to the *exact* ASG JSON form, e.g.:
//!
//! ```json
//! {
//!   "name": "document", "type": "block",
//!   "blocks": [
//!     { "name": "paragraph", "type": "block",
//!       "inlines": [ { "name": "text", "type": "string", "value": "hi",
//!                      "location": [{"line":1,"col":1},{"line":1,"col":2}] } ],
//!       "location": [{"line":1,"col":1},{"line":1,"col":2}] }
//!   ],
//!   "location": [{"line":1,"col":1},{"line":1,"col":2}]
//! }
//! ```
//!
//! It is deliberately **separate** from the OOXML-oriented [`crate::parser`]:
//! that parser discards source positions (it joins paragraph lines with `" "`),
//! which the ASG cannot tolerate — every ASG node carries a `location`. Rather
//! than retrofit positions and a section subtree onto the DOCX pipeline, the ASG
//! adapter has its own minimal, location-aware parser (see [`parse`]). The two
//! may converge later; for now isolation keeps the DOCX path stable.
//!
//! Scope is grown fixture-by-fixture against the vendored TCK suite (see
//! `tests/tck/`). The current tier covers: `document`, `paragraph`, and `text`
//! (no header, sections, lists, or inline markup yet).

mod parse;

pub use parse::{parse_document, parse_inlines};

use std::collections::BTreeMap;

use serde::Serialize;

/// A 1-based source position (`{ "line": L, "col": C }`).
///
/// `col` counts Unicode scalar values, 1-based, and for the end of a span
/// points at the **last** character (inclusive), matching the ASG fixtures
/// (e.g. the 9-char `"body only"` ends at `col 9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// A source span: `[start, end]`, both inclusive. Serializes as a 2-element
/// JSON array exactly as the ASG schema expects.
pub type Span = [Location; 2];

/// A `text` node: `{ "name": "text", "type": "string", "value", "location" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Text {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    pub value: String,
    pub location: Span,
}

impl Text {
    pub fn new(value: impl Into<String>, location: Span) -> Self {
        Self {
            name: "text",
            node_type: "string",
            value: value.into(),
            location,
        }
    }
}

/// An inline node. Serialized untagged so the JSON is the inner node's fields
/// directly (the `name`/`type` fields are the discriminator, per the ASG).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Inline {
    Text(Text),
}

impl Inline {
    /// The node's source span.
    pub fn location(&self) -> Span {
        match self {
            Inline::Text(t) => t.location,
        }
    }
}

/// A `paragraph` node: `{ "name", "type": "block", "inlines", "location" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Paragraph {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    pub inlines: Vec<Inline>,
    pub location: Span,
}

impl Paragraph {
    pub fn new(inlines: Vec<Inline>, location: Span) -> Self {
        Self {
            name: "paragraph",
            node_type: "block",
            inlines,
            location,
        }
    }
}

/// A `section` node: `{ "name": "section", "type": "block", "title", "level",
/// "location", "blocks" }`. `level` is the heading depth (`==` → 1, `===` → 2);
/// the section spans from its heading marker through the end of its last child
/// block. `blocks` is omitted when the section has no body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    pub title: Vec<Inline>,
    pub level: usize,
    pub location: Span,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
}

impl Section {
    pub fn new(title: Vec<Inline>, level: usize, location: Span, blocks: Vec<Block>) -> Self {
        Self {
            name: "section",
            node_type: "block",
            title,
            level,
            location,
            blocks,
        }
    }
}

/// A `listItem` node: `{ "name": "listItem", "type": "block", "marker",
/// "principal", "location" }`. `principal` is the item's inline content; the
/// item spans from its marker through the end of that content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    pub marker: String,
    pub principal: Vec<Inline>,
    pub location: Span,
}

impl ListItem {
    pub fn new(marker: impl Into<String>, principal: Vec<Inline>, location: Span) -> Self {
        Self {
            name: "listItem",
            node_type: "block",
            marker: marker.into(),
            principal,
            location,
        }
    }
}

/// A `list` node: `{ "name": "list", "type": "block", "variant", "marker",
/// "items", "location" }`. `variant` is e.g. `"unordered"`; the list spans from
/// its first item to its last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct List {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    pub variant: &'static str,
    pub marker: String,
    pub items: Vec<ListItem>,
    pub location: Span,
}

impl List {
    pub fn new(
        variant: &'static str,
        marker: impl Into<String>,
        items: Vec<ListItem>,
        location: Span,
    ) -> Self {
        Self {
            name: "list",
            node_type: "block",
            variant,
            marker: marker.into(),
            items,
            location,
        }
    }
}

/// A block node. Serialized untagged (see [`Inline`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Block {
    Paragraph(Paragraph),
    Section(Section),
    List(List),
}

impl Block {
    /// The node's source span.
    pub fn location(&self) -> Span {
        match self {
            Block::Paragraph(p) => p.location,
            Block::Section(s) => s.location,
            Block::List(l) => l.location,
        }
    }
}

/// A document `header`: the title inlines plus a span covering the title line
/// through the last attribute-entry line. Note a header is *not* itself a node
/// (it carries no `name`/`type`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Header {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    pub location: Span,
}

impl Header {
    pub fn new(title: Vec<Inline>, location: Span) -> Self {
        Self { title, location }
    }
}

/// The root `document` node.
///
/// Without a header it serializes as
/// `{ "name": "document", "type": "block", "blocks": [...], "location": [...] }`.
/// With a `= Title` header it also carries `attributes` (present even when
/// empty, i.e. `{}`) and a `header` object. `blocks` is omitted when empty
/// (e.g. a header-only document), matching the TCK fixtures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Document {
    name: &'static str,
    #[serde(rename = "type")]
    node_type: &'static str,
    /// Present iff the document has a header (then `{}` when no attribute
    /// entries), absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Header>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    pub location: Span,
}

impl Document {
    /// A header-less document (e.g. body-only input).
    pub fn new(blocks: Vec<Block>, location: Span) -> Self {
        Self {
            name: "document",
            node_type: "block",
            attributes: None,
            header: None,
            blocks,
            location,
        }
    }

    /// A document with a `= Title` header. `attributes` is always serialized
    /// (as `{}` when empty) once a header is present.
    pub fn with_header(
        header: Header,
        attributes: BTreeMap<String, String>,
        blocks: Vec<Block>,
        location: Span,
    ) -> Self {
        Self {
            name: "document",
            node_type: "block",
            attributes: Some(attributes),
            header: Some(header),
            blocks,
            location,
        }
    }
}
