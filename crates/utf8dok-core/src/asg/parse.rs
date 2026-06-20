//! Minimal location-aware AsciiDoc parser for the ASG adapter.
//!
//! Current tier: documents made of paragraphs of plain text. Blank lines
//! separate paragraphs; consecutive non-blank lines form one paragraph whose
//! text value joins the lines with `\n` (the ASG preserves hard line breaks in
//! the source — note this differs from [`crate::parser`], which joins with a
//! space for DOCX output).
//!
//! Location rules (recursive, derived from the TCK fixtures):
//! - A `text` leaf spans `[{first_line, 1}, {last_line, char_count}]`.
//! - A container (paragraph, document) spans from its first child's start to its
//!   last child's end.

use super::{Block, Document, Inline, Location, Paragraph, Span, Text};

/// Parse `source` into a `document` ASG node (block mode).
pub fn parse_document(source: &str) -> Document {
    let blocks = parse_blocks(source);
    let location = span_of_blocks(&blocks).unwrap_or([Location::new(1, 1), Location::new(1, 1)]);
    Document::new(blocks, location)
}

/// Parse `source` and return the inline nodes of its first block (inline mode).
///
/// The TCK's `inline/*` tests assert against a bare array of inline nodes rather
/// than a wrapping document, so the adapter projects out the first block's
/// inlines here.
pub fn parse_inlines(source: &str) -> Vec<Inline> {
    match parse_blocks(source).into_iter().next() {
        Some(Block::Paragraph(p)) => p.inlines,
        None => Vec::new(),
    }
}

/// Group lines into paragraph blocks, separated by blank lines.
fn parse_blocks(source: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    // Lines accumulated for the paragraph currently being built: (line_no, text).
    let mut current: Vec<(usize, &str)> = Vec::new();

    for (idx, raw_line) in source.split('\n').enumerate() {
        let line_no = idx + 1;
        // Tolerate CRLF input even though callers normally pre-normalize.
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        if line.trim().is_empty() {
            flush_paragraph(&mut blocks, &mut current);
        } else {
            current.push((line_no, line));
        }
    }
    flush_paragraph(&mut blocks, &mut current);

    blocks
}

fn flush_paragraph(blocks: &mut Vec<Block>, current: &mut Vec<(usize, &str)>) {
    if current.is_empty() {
        return;
    }
    let text = make_text(current);
    // A single-text paragraph spans exactly its text inline.
    let location = text.location;
    blocks.push(Block::Paragraph(Paragraph::new(
        vec![Inline::Text(text)],
        location,
    )));
    current.clear();
}

/// Build the `text` node for an accumulated run of paragraph lines.
fn make_text(lines: &[(usize, &str)]) -> Text {
    let value = lines
        .iter()
        .map(|(_, content)| *content)
        .collect::<Vec<_>>()
        .join("\n");

    let (first_line, _) = lines[0];
    let (last_line, last_content) = lines[lines.len() - 1];
    let start = Location::new(first_line, 1);
    // End col points at the last character (inclusive) = its 1-based index,
    // which equals the character count of the final line.
    let end = Location::new(last_line, char_count(last_content));
    Text::new(value, [start, end])
}

/// The span covering all blocks: first block's start to last block's end.
fn span_of_blocks(blocks: &[Block]) -> Option<Span> {
    let first = blocks.first()?.location();
    let last = blocks.last()?.location();
    Some([first[0], last[1]])
}

/// Number of Unicode scalar values in `s` (ASG columns count characters, not
/// bytes).
fn char_count(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(line: usize, col: usize) -> Location {
        Location::new(line, col)
    }

    #[test]
    fn single_line_paragraph_spans_the_line() {
        let doc = parse_document("body only");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Paragraph(p) = &doc.blocks[0];
        assert_eq!(p.location, [loc(1, 1), loc(1, 9)]);
        assert_eq!(p.inlines.len(), 1);
        let Inline::Text(t) = &p.inlines[0];
        assert_eq!(t.value, "body only");
        assert_eq!(t.location, [loc(1, 1), loc(1, 9)]);
        assert_eq!(doc.location, [loc(1, 1), loc(1, 9)]);
    }

    #[test]
    fn multiple_lines_join_with_newline_and_span_to_last_line() {
        let doc = parse_document("first line\nsecond longer line");
        let Block::Paragraph(p) = &doc.blocks[0];
        let Inline::Text(t) = &p.inlines[0];
        assert_eq!(t.value, "first line\nsecond longer line");
        // "second longer line" is 18 chars.
        assert_eq!(t.location, [loc(1, 1), loc(2, 18)]);
        assert_eq!(doc.location, [loc(1, 1), loc(2, 18)]);
    }

    #[test]
    fn blank_line_separates_sibling_paragraphs() {
        let doc = parse_document("one\n\nthree");
        assert_eq!(doc.blocks.len(), 2);
        let Block::Paragraph(p2) = &doc.blocks[1];
        assert_eq!(p2.location, [loc(3, 1), loc(3, 5)]);
        // Document spans from first paragraph start to last paragraph end.
        assert_eq!(doc.location, [loc(1, 1), loc(3, 5)]);
    }

    #[test]
    fn multiple_blank_lines_preserve_line_numbers() {
        let doc = parse_document("one\n\n\nfour para");
        assert_eq!(doc.blocks.len(), 2);
        let Block::Paragraph(p2) = &doc.blocks[1];
        assert_eq!(p2.location, [loc(4, 1), loc(4, 9)]);
    }

    #[test]
    fn parse_inlines_returns_bare_text_nodes() {
        let inlines = parse_inlines("hello");
        assert_eq!(inlines.len(), 1);
        let Inline::Text(t) = &inlines[0];
        assert_eq!(t.value, "hello");
        assert_eq!(t.location, [loc(1, 1), loc(1, 5)]);
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        // "café" is 4 chars but 5 bytes; end col must be 4.
        let inlines = parse_inlines("café");
        let Inline::Text(t) = &inlines[0];
        assert_eq!(t.location, [loc(1, 1), loc(1, 4)]);
    }

    #[test]
    fn empty_source_yields_no_blocks() {
        let doc = parse_document("");
        assert!(doc.blocks.is_empty());
        assert_eq!(doc.location, [loc(1, 1), loc(1, 1)]);
    }

    #[test]
    fn serializes_to_expected_asg_shape() {
        let doc = parse_document("hi");
        let v = serde_json::to_value(&doc).unwrap();
        assert_eq!(v["name"], "document");
        assert_eq!(v["type"], "block");
        assert_eq!(v["blocks"][0]["name"], "paragraph");
        assert_eq!(v["blocks"][0]["inlines"][0]["name"], "text");
        assert_eq!(v["blocks"][0]["inlines"][0]["type"], "string");
        assert_eq!(v["blocks"][0]["inlines"][0]["value"], "hi");
        // location is [ {line,col}, {line,col} ]
        assert_eq!(v["location"][0]["line"], 1);
        assert_eq!(v["location"][1]["col"], 2);
    }
}
