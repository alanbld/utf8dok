//! Minimal location-aware AsciiDoc parser for the ASG adapter.
//!
//! Current tiers:
//! - **document/paragraph/text** — blank lines separate paragraphs; consecutive
//!   non-blank lines form one paragraph whose text value joins the lines with
//!   `\n` (the ASG preserves hard line breaks in the source — note this differs
//!   from [`crate::parser`], which joins with a space for DOCX output).
//! - **header** — a leading `= Title` line, optionally followed by contiguous
//!   `:name: value` / `:name:` attribute entries.
//! - **sections** — `==`+ headings nesting recursively.
//! - **unordered lists** — contiguous `*` items.
//! - **delimited blocks** — `----` listing (verbatim) and `****` sidebar
//!   (recursively-parsed body).
//! - **inline markup** (inline mode only) — constrained strong spans `*…*`.
//!
//! Location rules (recursive, derived from the TCK fixtures):
//! - A `text` leaf spans `[{first_line, 1}, {last_line, char_count}]`.
//! - A container (paragraph, document) spans from its first child's start to its
//!   last child's end.
//! - The `header` spans from the `=` at `{1,1}` through the end of its last
//!   attribute entry (or the end of the title if there are none).

use std::collections::BTreeMap;
use std::ops::Range;

use super::{
    Block, Document, Header, Inline, InlineSpan, List, ListItem, Listing, Location, Paragraph,
    Section, Sidebar, Text,
};

/// Parse `source` into a `document` ASG node (block mode).
pub fn parse_document(source: &str) -> Document {
    let lines: Vec<&str> = source.split('\n').map(strip_cr).collect();

    // Try to peel off a document header (`= Title` + attribute entries).
    let parsed_header = parse_header(&lines);
    let body_start = parsed_header.as_ref().map_or(0, |h| h.next_line_index);
    let blocks = parse_blocks_range(&lines, body_start, lines.len());

    // Document span: start at the header (or first block), end at the last block
    // (or the header when there is no body).
    let start = match &parsed_header {
        Some(_) => Location::new(1, 1),
        None => blocks
            .first()
            .map_or(Location::new(1, 1), |b| b.location()[0]),
    };
    let end = match blocks.last() {
        Some(b) => b.location()[1],
        None => parsed_header
            .as_ref()
            .map_or(Location::new(1, 1), |h| h.header.location[1]),
    };
    let location = [start, end];

    match parsed_header {
        Some(h) => Document::with_header(h.header, h.attributes, blocks, location),
        None => Document::new(blocks, location),
    }
}

/// Parse `source` as inline content and return its inline nodes (inline mode).
///
/// The TCK's `inline/*` tests assert against a bare array of inline nodes rather
/// than a wrapping document, so inline mode parses markup directly. The current
/// tier handles a single line with optional constrained strong spans (`*…*`).
///
/// Note: block-level paragraphs still emit a single `text` inline (see
/// [`make_text`]); applying markup parsing inside multi-line paragraphs is a
/// later tier.
pub fn parse_inlines(source: &str) -> Vec<Inline> {
    let line = strip_cr(source.split('\n').next().unwrap_or(""));
    let positioned: Vec<(char, Location)> = line
        .chars()
        .enumerate()
        .map(|(ci, ch)| (ch, Location::new(1, ci + 1)))
        .collect();
    parse_inline_markup(&positioned)
}

/// A document header peeled off the front of the source.
struct ParsedHeader {
    header: Header,
    attributes: BTreeMap<String, String>,
    /// 0-based index of the first line *after* the header.
    next_line_index: usize,
}

/// Parse a leading `= Title` header plus any contiguous attribute entries.
/// Returns `None` when the first line is not a document title.
fn parse_header(lines: &[&str]) -> Option<ParsedHeader> {
    let first = lines.first()?;
    if !is_doc_title(first) {
        return None;
    }

    let title = heading_title(first, 1);
    // Header span ends at the title unless attribute entries extend it.
    let mut header_end = title.location[1];
    let title_inlines = vec![Inline::Text(title)];

    let mut attributes = BTreeMap::new();
    let mut idx = 1;
    while let Some(line) = lines.get(idx) {
        match parse_attribute_entry(line) {
            Some((name, value)) => {
                attributes.insert(name, value);
                header_end = Location::new(idx + 1, char_count(line));
                idx += 1;
            }
            None => break,
        }
    }

    let header = Header::new(title_inlines, [Location::new(1, 1), header_end]);
    Some(ParsedHeader {
        header,
        attributes,
        next_line_index: idx,
    })
}

/// Parse lines `[start..end)` into blocks (paragraphs and nested sections).
///
/// A section heading opens a section whose body runs until the next heading of
/// the same or higher rank (i.e. `level <= this level`), or the end of the
/// range; that body is parsed recursively so sub-sections nest.
fn parse_blocks_range(lines: &[&str], start: usize, end: usize) -> Vec<Block> {
    let mut blocks = Vec::new();
    // Lines accumulated for the paragraph currently being built: (line_no, text).
    let mut current: Vec<(usize, &str)> = Vec::new();

    let mut i = start;
    while i < end {
        let line = lines[i];
        if let Some(delim_char) = block_delimiter(line) {
            flush_paragraph(&mut blocks, &mut current);
            // Find the matching closing delimiter (an identical line).
            let mut close = i + 1;
            while close < end && lines[close] != line {
                close += 1;
            }
            let closed = close < end;
            let content = i + 1..close;
            let block = match delim_char {
                '-' => make_listing(lines, content, line, i, closed.then_some(close)),
                _ => make_sidebar(lines, content, line, i, closed.then_some(close)),
            };
            blocks.push(block);
            i = if closed { close + 1 } else { end };
        } else if let Some(level) = section_level(line) {
            flush_paragraph(&mut blocks, &mut current);
            // Find where this section's body ends.
            let mut j = i + 1;
            while j < end {
                if matches!(section_level(lines[j]), Some(other) if other <= level) {
                    break;
                }
                j += 1;
            }
            let children = parse_blocks_range(lines, i + 1, j);
            blocks.push(make_section(lines, i, level, children));
            i = j;
        } else if let Some(marker) = unordered_marker(line) {
            flush_paragraph(&mut blocks, &mut current);
            // Consume contiguous items sharing this marker into one list.
            let mut items = Vec::new();
            while i < end {
                match unordered_marker(lines[i]) {
                    Some(m) if m == marker => {
                        items.push(make_list_item(lines[i], i, m));
                        i += 1;
                    }
                    _ => break,
                }
            }
            blocks.push(make_list(marker, items));
        } else if line.trim().is_empty() {
            flush_paragraph(&mut blocks, &mut current);
            i += 1;
        } else {
            current.push((i + 1, line));
            i += 1;
        }
    }
    flush_paragraph(&mut blocks, &mut current);

    blocks
}

/// Build a `section` block from its heading line and already-parsed children.
fn make_section(lines: &[&str], heading_idx: usize, level: usize, children: Vec<Block>) -> Block {
    let line_no = heading_idx + 1;
    let title = heading_title(lines[heading_idx], line_no);
    let start = Location::new(line_no, 1);
    // The section spans through its last child block; with no body it ends at
    // the title.
    let end = children
        .last()
        .map_or(title.location[1], |b| b.location()[1]);
    Block::Section(Section::new(
        vec![Inline::Text(title)],
        level,
        [start, end],
        children,
    ))
}

fn flush_paragraph(blocks: &mut Vec<Block>, current: &mut Vec<(usize, &str)>) {
    if current.is_empty() {
        return;
    }
    // Parse inline markup across the paragraph's lines. Markup-free text still
    // collapses to a single `text` node (possibly spanning lines via `\n`).
    let inlines = parse_block_inlines(current);
    let location = inline_span(&inlines, current);
    blocks.push(Block::Paragraph(Paragraph::new(inlines, location)));
    current.clear();
}

/// The span covering a run of inlines: first inline's start to last's end.
fn inline_span(inlines: &[Inline], lines: &[(usize, &str)]) -> [Location; 2] {
    match (inlines.first(), inlines.last()) {
        (Some(first), Some(last)) => [first.location()[0], last.location()[1]],
        _ => {
            let (line_no, _) = lines[0];
            [Location::new(line_no, 1), Location::new(line_no, 1)]
        }
    }
}

/// Parse a paragraph's accumulated lines into inline nodes, joining the lines
/// with `\n` and tracking each character's source `(line, col)`.
fn parse_block_inlines(lines: &[(usize, &str)]) -> Vec<Inline> {
    let mut positioned: Vec<(char, Location)> = Vec::new();
    for (idx, (line_no, content)) in lines.iter().enumerate() {
        if idx > 0 {
            // Newline join between source lines; its position is interior to a
            // text run (never a run's start/end for the cases we handle).
            let (prev_line, prev_content) = lines[idx - 1];
            positioned.push(('\n', Location::new(prev_line, char_count(prev_content) + 1)));
        }
        for (ci, ch) in content.chars().enumerate() {
            positioned.push((ch, Location::new(*line_no, ci + 1)));
        }
    }
    parse_inline_markup(&positioned)
}

/// Build a verbatim `text` node for a run of lines (used by listing blocks).
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

/// Is `line` a level-0 document title (`= Title`, but not `== Section`)?
fn is_doc_title(line: &str) -> bool {
    line.starts_with("= ")
}

/// If `line` is a section heading (`==`+ followed by a space), return its level
/// (`==` → 1, `===` → 2, …). A single `=` is the document title, not a section.
fn section_level(line: &str) -> Option<usize> {
    let equals = line.chars().take_while(|c| *c == '=').count();
    if equals >= 2 && line[equals..].starts_with(' ') {
        Some(equals - 1)
    } else {
        None
    }
}

/// Parse the title text of a heading line (`=`+ Title) into a `text` node.
///
/// The title begins after the `=` marker run; its column span counts
/// characters, so `= Document Title` on line 1 yields `[{1,3},{1,16}]` and
/// `== Section Title` yields `[{1,4},{1,16}]`.
fn heading_title(line: &str, line_no: usize) -> Text {
    let marker_len = line.chars().take_while(|c| *c == '=').count();
    marked_text(line, line_no, marker_len)
}

/// If `line` is an unordered list item (`*`+ followed by a space), return its
/// marker (the run of `*`). Requires the trailing space so inline `*bold*`
/// text is not mistaken for a list.
fn unordered_marker(line: &str) -> Option<&str> {
    let stars = line.chars().take_while(|c| *c == '*').count();
    if stars >= 1 && line[stars..].starts_with(' ') {
        Some(&line[..stars])
    } else {
        None
    }
}

/// Build a `listItem` from one item line. The principal content follows the
/// marker; the item spans from the marker (col 1) through the principal's end.
fn make_list_item(line: &str, idx: usize, marker: &str) -> ListItem {
    let line_no = idx + 1;
    let principal = marked_text(line, line_no, marker.chars().count());
    let location = [Location::new(line_no, 1), principal.location[1]];
    ListItem::new(marker, vec![Inline::Text(principal)], location)
}

/// Build an unordered `list` from its items. The list spans from its first item
/// to its last.
fn make_list(marker: &str, items: Vec<ListItem>) -> Block {
    let start = items
        .first()
        .map_or(Location::new(1, 1), |it| it.location[0]);
    let end = items
        .last()
        .map_or(Location::new(1, 1), |it| it.location[1]);
    Block::List(List::new("unordered", marker, items, [start, end]))
}

/// If `line` is a delimited-block fence (4+ identical `-` or `*`), return that
/// character. `-` opens a listing, `*` a sidebar.
fn block_delimiter(line: &str) -> Option<char> {
    let first = line.chars().next()?;
    if (first == '-' || first == '*')
        && line.chars().count() >= 4
        && line.chars().all(|c| c == first)
    {
        Some(first)
    } else {
        None
    }
}

/// The span of a delimited block: from its opening fence (col 1) through the
/// closing fence (or, when unterminated, the end of its content).
fn delimited_span(
    delimiter: &str,
    open_idx: usize,
    close_idx: Option<usize>,
    content_end: Location,
) -> [Location; 2] {
    let start = Location::new(open_idx + 1, 1);
    let end = match close_idx {
        Some(c) => Location::new(c + 1, char_count(delimiter)),
        None => content_end,
    };
    [start, end]
}

/// Build a `listing` block: verbatim content between the fences as one `text`
/// inline.
fn make_listing(
    lines: &[&str],
    content: Range<usize>,
    delimiter: &str,
    open_idx: usize,
    close_idx: Option<usize>,
) -> Block {
    let content_lines: Vec<(usize, &str)> = content.map(|k| (k + 1, lines[k])).collect();
    let inlines = if content_lines.is_empty() {
        Vec::new()
    } else {
        vec![Inline::Text(make_text(&content_lines))]
    };
    let content_end = inlines
        .last()
        .map_or(Location::new(open_idx + 1, char_count(delimiter)), |inl| {
            inl.location()[1]
        });
    let location = delimited_span(delimiter, open_idx, close_idx, content_end);
    Block::Listing(Listing::new(delimiter, inlines, location))
}

/// Build a `sidebar` block: its body is parsed recursively into child blocks.
fn make_sidebar(
    lines: &[&str],
    content: Range<usize>,
    delimiter: &str,
    open_idx: usize,
    close_idx: Option<usize>,
) -> Block {
    let children = parse_blocks_range(lines, content.start, content.end);
    let content_end = children
        .last()
        .map_or(Location::new(open_idx + 1, char_count(delimiter)), |b| {
            b.location()[1]
        });
    let location = delimited_span(delimiter, open_idx, close_idx, content_end);
    Block::Sidebar(Sidebar::new(delimiter, children, location))
}

/// Build a `text` node from the content of `line` after a leading marker of
/// `marker_len` characters plus its following spaces, on `line_no`. Shared by
/// heading titles and list-item principals.
fn marked_text(line: &str, line_no: usize, marker_len: usize) -> Text {
    let after_marker: String = line.chars().skip(marker_len).collect();
    let leading_spaces = after_marker.chars().take_while(|c| *c == ' ').count();
    let text = after_marker.trim();

    // Column of the first content character: marker chars + leading spaces, +1.
    let start_col = marker_len + leading_spaces + 1;
    let len = char_count(text);
    let end_col = start_col + len.saturating_sub(1);
    Text::new(
        text,
        [
            Location::new(line_no, start_col),
            Location::new(line_no, end_col),
        ],
    )
}

/// Parse an attribute entry line (`:name: value` or `:name:`) into a key/value
/// pair. Returns `None` for any non-attribute line.
fn parse_attribute_entry(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix(':')?;
    let closing = rest.find(':')?;
    let name = &rest[..closing];
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    let value = rest[closing + 1..].trim().to_string();
    Some((name.to_string(), value))
}

/// Parse inline markup over a positioned-character sequence (each char carries
/// its source `(line, col)`). Handles constrained strong spans (`*…*`) and plain
/// text runs; a run may span a `\n` join, yielding one multi-line `text` node.
fn parse_inline_markup(chars: &[(char, Location)]) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut run_start = 0;
    let mut i = 0;

    while i < chars.len() {
        if chars[i].0 == '*' && is_constrained_open(chars, i) {
            if let Some(close) = find_constrained_close(chars, i + 1) {
                flush_text_run(&mut inlines, chars, run_start, i);
                let inner_inlines = parse_inline_markup(&chars[i + 1..close]);
                let location = [chars[i].1, chars[close].1];
                inlines.push(Inline::Span(InlineSpan::strong_constrained(
                    inner_inlines,
                    location,
                )));
                i = close + 1;
                run_start = i;
                continue;
            }
        }
        i += 1;
    }
    flush_text_run(&mut inlines, chars, run_start, chars.len());
    inlines
}

/// Emit a `text` node for the plain run `chars[from..to)`, if non-empty.
fn flush_text_run(inlines: &mut Vec<Inline>, chars: &[(char, Location)], from: usize, to: usize) {
    if from >= to {
        return;
    }
    let value: String = chars[from..to].iter().map(|(c, _)| *c).collect();
    let location = [chars[from].1, chars[to - 1].1];
    inlines.push(Inline::Text(Text::new(value, location)));
}

/// A constrained marker opens when it is at the start or follows whitespace, and
/// the next character is not whitespace (AsciiDoc constrained-formatting rule).
fn is_constrained_open(chars: &[(char, Location)], i: usize) -> bool {
    let prev_ok = i == 0 || chars[i - 1].0.is_whitespace();
    let next_ok = chars.get(i + 1).is_some_and(|(c, _)| !c.is_whitespace());
    prev_ok && next_ok
}

/// Find the matching constrained closing `*`: preceded by a non-space and
/// followed by end-of-line or a non-word character.
fn find_constrained_close(chars: &[(char, Location)], from: usize) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j].0 == '*' {
            let prev_ok = j > from && !chars[j - 1].0.is_whitespace();
            let next_ok = chars.get(j + 1).is_none_or(|(c, _)| !c.is_alphanumeric());
            if prev_ok && next_ok {
                return Some(j);
            }
        }
        j += 1;
    }
    None
}

/// Strip a trailing `\r` so CRLF input is tolerated.
fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
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
        assert!(doc.header.is_none());
        assert!(doc.attributes.is_none());
        assert_eq!(doc.blocks.len(), 1);
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p.location, [loc(1, 1), loc(1, 9)]);
        assert_eq!(p.inlines.len(), 1);
        let Inline::Text(t) = &p.inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "body only");
        assert_eq!(t.location, [loc(1, 1), loc(1, 9)]);
        assert_eq!(doc.location, [loc(1, 1), loc(1, 9)]);
    }

    #[test]
    fn multiple_lines_join_with_newline_and_span_to_last_line() {
        let doc = parse_document("first line\nsecond longer line");
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let Inline::Text(t) = &p.inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "first line\nsecond longer line");
        // "second longer line" is 18 chars.
        assert_eq!(t.location, [loc(1, 1), loc(2, 18)]);
        assert_eq!(doc.location, [loc(1, 1), loc(2, 18)]);
    }

    #[test]
    fn blank_line_separates_sibling_paragraphs() {
        let doc = parse_document("one\n\nthree");
        assert_eq!(doc.blocks.len(), 2);
        let Block::Paragraph(p2) = &doc.blocks[1] else {
            panic!("expected paragraph");
        };
        assert_eq!(p2.location, [loc(3, 1), loc(3, 5)]);
        // Document spans from first paragraph start to last paragraph end.
        assert_eq!(doc.location, [loc(1, 1), loc(3, 5)]);
    }

    #[test]
    fn multiple_blank_lines_preserve_line_numbers() {
        let doc = parse_document("one\n\n\nfour para");
        assert_eq!(doc.blocks.len(), 2);
        let Block::Paragraph(p2) = &doc.blocks[1] else {
            panic!("expected paragraph");
        };
        assert_eq!(p2.location, [loc(4, 1), loc(4, 9)]);
    }

    #[test]
    fn parse_inlines_returns_bare_text_nodes() {
        let inlines = parse_inlines("hello");
        assert_eq!(inlines.len(), 1);
        let Inline::Text(t) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "hello");
        assert_eq!(t.location, [loc(1, 1), loc(1, 5)]);
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        // "café" is 4 chars but 5 bytes; end col must be 4.
        let inlines = parse_inlines("café");
        let Inline::Text(t) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.location, [loc(1, 1), loc(1, 4)]);
    }

    #[test]
    fn empty_source_yields_no_blocks() {
        let doc = parse_document("");
        assert!(doc.blocks.is_empty());
        assert_eq!(doc.location, [loc(1, 1), loc(1, 1)]);
    }

    #[test]
    fn header_with_title_only_and_body() {
        let doc = parse_document("= Document Title\n\nbody");
        let header = doc.header.as_ref().expect("header present");
        // Title text starts after "= " at col 3, "Document Title" is 14 chars.
        let Inline::Text(t) = &header.title[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "Document Title");
        assert_eq!(t.location, [loc(1, 3), loc(1, 16)]);
        // No attribute entries -> header span ends at the title.
        assert_eq!(header.location, [loc(1, 1), loc(1, 16)]);
        // attributes present but empty.
        assert_eq!(doc.attributes.as_ref().unwrap().len(), 0);
        // Body paragraph on line 3.
        assert_eq!(doc.blocks.len(), 1);
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p.location, [loc(3, 1), loc(3, 4)]);
        // Document spans header start to body end.
        assert_eq!(doc.location, [loc(1, 1), loc(3, 4)]);
    }

    #[test]
    fn header_attribute_entries_extend_header_span() {
        let doc = parse_document("= Document Title\n:icons: font\n:toc:");
        let header = doc.header.as_ref().expect("header present");
        // Header span runs through the last attribute entry (`:toc:` = 5 chars).
        assert_eq!(header.location, [loc(1, 1), loc(3, 5)]);
        let attrs = doc.attributes.as_ref().unwrap();
        assert_eq!(attrs.get("icons").map(String::as_str), Some("font"));
        assert_eq!(attrs.get("toc").map(String::as_str), Some(""));
        // No body blocks.
        assert!(doc.blocks.is_empty());
        // Document span equals the header span when there is no body.
        assert_eq!(doc.location, [loc(1, 1), loc(3, 5)]);
    }

    #[test]
    fn section_marker_is_not_a_document_title() {
        // `== Section` is a section, not a level-0 document title/header.
        let doc = parse_document("== Section");
        assert!(doc.header.is_none());
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Section(_)));
    }

    #[test]
    fn section_holds_title_level_and_child_blocks() {
        let doc = parse_document("== Section Title\n\nparagraph");
        assert!(doc.header.is_none());
        assert_eq!(doc.blocks.len(), 1);
        let Block::Section(s) = &doc.blocks[0] else {
            panic!("expected section");
        };
        assert_eq!(s.level, 1);
        // Title starts after "== " at col 4; "Section Title" is 13 chars.
        let Inline::Text(t) = &s.title[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "Section Title");
        assert_eq!(t.location, [loc(1, 4), loc(1, 16)]);
        // Section spans its heading through the end of its last child block.
        assert_eq!(s.location, [loc(1, 1), loc(3, 9)]);
        assert_eq!(s.blocks.len(), 1);
        let Block::Paragraph(p) = &s.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p.location, [loc(3, 1), loc(3, 9)]);
        // Document span equals the section span.
        assert_eq!(doc.location, [loc(1, 1), loc(3, 9)]);
    }

    #[test]
    fn unordered_list_single_item() {
        let doc = parse_document("* water");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List(list) = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(list.variant, "unordered");
        assert_eq!(list.marker, "*");
        assert_eq!(list.location, [loc(1, 1), loc(1, 7)]);
        assert_eq!(list.items.len(), 1);
        let item = &list.items[0];
        assert_eq!(item.marker, "*");
        // Item spans from the marker (col 1) through the principal end (col 7).
        assert_eq!(item.location, [loc(1, 1), loc(1, 7)]);
        let Inline::Text(t) = &item.principal[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "water");
        assert_eq!(t.location, [loc(1, 3), loc(1, 7)]);
    }

    #[test]
    fn unordered_list_multiple_items_in_one_list() {
        let doc = parse_document("* one\n* two");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List(list) = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(list.items.len(), 2);
        // List spans first item start to last item end ("two" ends at col 5).
        assert_eq!(list.location, [loc(1, 1), loc(2, 5)]);
    }

    #[test]
    fn listing_block_holds_verbatim_text() {
        let doc = parse_document("----\ndef main\n  puts 'hi'\nend\n----");
        assert_eq!(doc.blocks.len(), 1);
        let Block::Listing(l) = &doc.blocks[0] else {
            panic!("expected listing");
        };
        assert_eq!(l.delimiter, "----");
        // Block spans both fence lines (closing "----" ends at col 4 on line 5).
        assert_eq!(l.location, [loc(1, 1), loc(5, 4)]);
        let Inline::Text(t) = &l.inlines[0] else {
            panic!("expected text");
        };
        // Content is verbatim (leading spaces preserved) and spans the body.
        assert_eq!(t.value, "def main\n  puts 'hi'\nend");
        assert_eq!(t.location, [loc(2, 1), loc(4, 3)]);
    }

    #[test]
    fn sidebar_block_parses_nested_blocks() {
        let doc = parse_document("****\n* phone\n* keys\n****");
        let Block::Sidebar(s) = &doc.blocks[0] else {
            panic!("expected sidebar");
        };
        assert_eq!(s.delimiter, "****");
        assert_eq!(s.location, [loc(1, 1), loc(4, 4)]);
        // Body is recursively parsed: an unordered list of two items.
        assert_eq!(s.blocks.len(), 1);
        let Block::List(list) = &s.blocks[0] else {
            panic!("expected nested list");
        };
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.location, [loc(2, 1), loc(3, 6)]);
    }

    #[test]
    fn inline_constrained_strong_span() {
        let inlines = parse_inlines("*s*");
        assert_eq!(inlines.len(), 1);
        let Inline::Span(span) = &inlines[0] else {
            panic!("expected span");
        };
        assert_eq!(span.variant, "strong");
        assert_eq!(span.form, "constrained");
        assert_eq!(span.location, [loc(1, 1), loc(1, 3)]);
        let Inline::Text(t) = &span.inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "s");
        assert_eq!(t.location, [loc(1, 2), loc(1, 2)]);
    }

    #[test]
    fn inline_text_around_a_strong_span() {
        // "a *b* c" -> text "a ", span "b", text " c".
        let inlines = parse_inlines("a *b* c");
        assert_eq!(inlines.len(), 3);
        let Inline::Text(a) = &inlines[0] else {
            panic!("expected leading text");
        };
        assert_eq!(a.value, "a ");
        assert_eq!(a.location, [loc(1, 1), loc(1, 2)]);
        assert!(matches!(inlines[1], Inline::Span(_)));
        let Inline::Text(c) = &inlines[2] else {
            panic!("expected trailing text");
        };
        assert_eq!(c.value, " c");
        assert_eq!(c.location, [loc(1, 6), loc(1, 7)]);
    }

    #[test]
    fn block_paragraph_parses_inline_markup() {
        // A paragraph now parses markup: "a *b* c" -> text, span, text.
        let doc = parse_document("a *b* c");
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p.inlines.len(), 3);
        assert!(matches!(p.inlines[1], Inline::Span(_)));
        assert_eq!(p.location, [loc(1, 1), loc(1, 7)]);
        let Inline::Span(span) = &p.inlines[1] else {
            panic!("expected span");
        };
        assert_eq!(span.variant, "strong");
        assert_eq!(span.location, [loc(1, 3), loc(1, 5)]);
    }

    #[test]
    fn block_markup_span_across_a_line_break() {
        // A span on the second line of a multi-line paragraph keeps its line.
        let doc = parse_document("intro line\nthen *bold* here");
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        // text "intro line\nthen ", span "bold" (line 2), text " here".
        let Inline::Span(span) = p
            .inlines
            .iter()
            .find(|i| matches!(i, Inline::Span(_)))
            .expect("a span")
        else {
            unreachable!()
        };
        assert_eq!(span.location, [loc(2, 6), loc(2, 11)]);
        // Leading text run spans the line break as a single node.
        let Inline::Text(head) = &p.inlines[0] else {
            panic!("expected leading text");
        };
        assert_eq!(head.value, "intro line\nthen ");
        assert_eq!(head.location, [loc(1, 1), loc(2, 5)]);
    }

    #[test]
    fn markup_free_multiline_paragraph_stays_one_text_node() {
        // Regression: no markup -> still a single text node spanning lines.
        let doc = parse_document("one two\nthree four");
        let Block::Paragraph(p) = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p.inlines.len(), 1);
        let Inline::Text(t) = &p.inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(t.value, "one two\nthree four");
        assert_eq!(t.location, [loc(1, 1), loc(2, 10)]);
    }

    #[test]
    fn inline_star_text_is_not_a_list() {
        // `*bold*` (no space after the marker) is a paragraph, not a list.
        let doc = parse_document("*bold*");
        assert!(matches!(doc.blocks[0], Block::Paragraph(_)));
    }

    #[test]
    fn nested_section_increments_level_and_nests() {
        let doc = parse_document("== Parent\n\n=== Child\n\nbody");
        let Block::Section(parent) = &doc.blocks[0] else {
            panic!("expected parent section");
        };
        assert_eq!(parent.level, 1);
        assert_eq!(parent.blocks.len(), 1);
        let Block::Section(child) = &parent.blocks[0] else {
            panic!("expected nested child section");
        };
        assert_eq!(child.level, 2);
        assert_eq!(child.blocks.len(), 1);
    }

    #[test]
    fn header_only_document_omits_blocks_in_json() {
        let doc = parse_document("= Document Title\n:toc:");
        let v = serde_json::to_value(&doc).unwrap();
        assert!(v.get("blocks").is_none(), "empty blocks should be omitted");
        assert_eq!(v["attributes"]["toc"], "");
        assert_eq!(v["header"]["title"][0]["value"], "Document Title");
    }

    #[test]
    fn serializes_to_expected_asg_shape() {
        let doc = parse_document("hi");
        let v = serde_json::to_value(&doc).unwrap();
        assert_eq!(v["name"], "document");
        assert_eq!(v["type"], "block");
        // No header -> no attributes/header keys.
        assert!(v.get("attributes").is_none());
        assert!(v.get("header").is_none());
        assert_eq!(v["blocks"][0]["name"], "paragraph");
        assert_eq!(v["blocks"][0]["inlines"][0]["name"], "text");
        assert_eq!(v["blocks"][0]["inlines"][0]["type"], "string");
        assert_eq!(v["blocks"][0]["inlines"][0]["value"], "hi");
        assert_eq!(v["location"][0]["line"], 1);
        assert_eq!(v["location"][1]["col"], 2);
    }
}
