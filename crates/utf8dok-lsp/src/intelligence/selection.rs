//! Smart selection for AsciiDoc documents
//!
//! Provides hierarchy-aware selection expansion:
//! - Word → Line → Paragraph/Block → Section → Document
//!
//! Integrates with Phase 7 folding ranges for structural awareness.

use crate::structural::FoldingAnalyzer;
use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{Position, Range, SelectionRange};

/// Represents a selection level with metadata
#[derive(Debug, Clone)]
pub struct SelectionLevel {
    pub range: Range,
    #[allow(dead_code)]
    pub kind: SelectionKind,
}

/// Classification of selection scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Word,
    Line,
    LogicalBlock,    // Paragraph, attribute group
    StructuralBlock, // Code block, section
    Document,
}

/// Analyzer for generating selection range hierarchies
pub struct SelectionAnalyzer<'a> {
    #[allow(dead_code)]
    text: &'a str,
    lines: Vec<&'a str>,
    folding_ranges: Vec<Range>,
}

impl<'a> SelectionAnalyzer<'a> {
    /// Create a new selection analyzer for the given text
    pub fn new(text: &'a str) -> Self {
        let lines: Vec<&str> = text.lines().collect();

        // Use Phase 7 folding analyzer to get structural blocks
        let folding_ranges = FoldingAnalyzer::generate_ranges(text)
            .into_iter()
            .map(|fr| {
                let end_char = if (fr.end_line as usize) < lines.len() {
                    lines[fr.end_line as usize].len() as u32
                } else {
                    0
                };
                Range {
                    start: Position {
                        line: fr.start_line,
                        character: 0,
                    },
                    end: Position {
                        line: fr.end_line,
                        character: end_char,
                    },
                }
            })
            .collect();

        Self {
            text,
            lines,
            folding_ranges,
        }
    }

    /// Get hierarchy of selection ranges from cursor position
    /// Returns ranges from most specific (word) to least specific (document)
    pub fn get_selection_hierarchy(&self, cursor: Position) -> Vec<SelectionLevel> {
        let mut hierarchy = Vec::new();
        let line_idx = cursor.line as usize;

        if line_idx >= self.lines.len() {
            // Return just document range for out-of-bounds
            hierarchy.push(SelectionLevel {
                range: self.get_document_range(),
                kind: SelectionKind::Document,
            });
            return hierarchy;
        }

        // Level 1: Word at cursor (if any)
        if let Some(word_range) = self.get_word_at(cursor) {
            hierarchy.push(SelectionLevel {
                range: word_range,
                kind: SelectionKind::Word,
            });
        }

        // Level 2: Xref or attribute usage (special inline elements)
        if let Some(inline_range) = self.get_inline_element_at(cursor) {
            // Only add if different from word
            if hierarchy.is_empty() || hierarchy.last().unwrap().range != inline_range {
                hierarchy.push(SelectionLevel {
                    range: inline_range,
                    kind: SelectionKind::Word,
                });
            }
        }

        // Level 3: Line containing cursor
        let line_range = self.get_line_range(cursor.line);
        if hierarchy.is_empty() || hierarchy.last().unwrap().range != line_range {
            hierarchy.push(SelectionLevel {
                range: line_range,
                kind: SelectionKind::Line,
            });
        }

        // Level 4: Logical block (paragraph, attribute group, etc.)
        if let Some(logical_range) = self.get_logical_block(cursor) {
            if hierarchy
                .last()
                .map(|h| h.range != logical_range)
                .unwrap_or(true)
            {
                hierarchy.push(SelectionLevel {
                    range: logical_range,
                    kind: SelectionKind::LogicalBlock,
                });
            }
        }

        // Level 5+: Structural blocks (from folding ranges), sorted by size
        let mut structural_ranges = self.get_structural_blocks_containing(cursor);
        structural_ranges.sort_by_key(|r| self.range_size(r));

        for range in structural_ranges {
            if hierarchy.last().map(|h| h.range != range).unwrap_or(true) {
                hierarchy.push(SelectionLevel {
                    range,
                    kind: SelectionKind::StructuralBlock,
                });
            }
        }

        // Final level: Entire document
        let doc_range = self.get_document_range();
        if hierarchy
            .last()
            .map(|h| h.range != doc_range)
            .unwrap_or(true)
        {
            hierarchy.push(SelectionLevel {
                range: doc_range,
                kind: SelectionKind::Document,
            });
        }

        hierarchy
    }

    /// Calculate approximate size of a range (for sorting)
    fn range_size(&self, range: &Range) -> u32 {
        let line_diff = range.end.line.saturating_sub(range.start.line);
        line_diff * 1000 + range.end.character.saturating_sub(range.start.character)
    }

    /// Get word boundaries at cursor position
    fn get_word_at(&self, cursor: Position) -> Option<Range> {
        let line_idx = cursor.line as usize;
        if line_idx >= self.lines.len() {
            return None;
        }

        let line = self.lines[line_idx];
        let char_idx = cursor.character as usize;

        if char_idx > line.len() || line.is_empty() {
            return None;
        }

        let bytes = line.as_bytes();
        let char_idx = char_idx.min(bytes.len().saturating_sub(1));

        // Check if we're on a word character
        if !Self::is_word_char(bytes[char_idx]) {
            return None;
        }

        // Find word start
        let mut start = char_idx;
        while start > 0 && Self::is_word_char(bytes[start - 1]) {
            start -= 1;
        }

        // Find word end
        let mut end = char_idx;
        while end < bytes.len() && Self::is_word_char(bytes[end]) {
            end += 1;
        }

        if start < end {
            Some(Range {
                start: Position {
                    line: cursor.line,
                    character: start as u32,
                },
                end: Position {
                    line: cursor.line,
                    character: end as u32,
                },
            })
        } else {
            None
        }
    }

    /// Get inline element (xref, attribute usage) at cursor
    fn get_inline_element_at(&self, cursor: Position) -> Option<Range> {
        let line_idx = cursor.line as usize;
        if line_idx >= self.lines.len() {
            return None;
        }

        let line = self.lines[line_idx];
        let char_idx = cursor.character as usize;

        // Check for xref <<id>>
        if let Some(range) = self.find_xref_at(line, cursor.line, char_idx) {
            return Some(range);
        }

        // Check for attribute usage {name}
        if let Some(range) = self.find_attr_usage_at(line, cursor.line, char_idx) {
            return Some(range);
        }

        None
    }

    /// Find xref at position
    fn find_xref_at(&self, line: &str, line_num: u32, char_idx: usize) -> Option<Range> {
        static XREF_RE: OnceLock<Regex> = OnceLock::new();
        let re = XREF_RE.get_or_init(|| Regex::new(r"<<([\w\-]+)(?:,[^>]*)?\s*>>").unwrap());

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let id_match = cap.get(1).unwrap();

            // Check if cursor is on the ID part
            if id_match.start() <= char_idx && char_idx < id_match.end() {
                return Some(Range {
                    start: Position {
                        line: line_num,
                        character: id_match.start() as u32,
                    },
                    end: Position {
                        line: line_num,
                        character: id_match.end() as u32,
                    },
                });
            }

            // Check if cursor is anywhere in the xref
            if full_match.start() <= char_idx && char_idx < full_match.end() {
                return Some(Range {
                    start: Position {
                        line: line_num,
                        character: full_match.start() as u32,
                    },
                    end: Position {
                        line: line_num,
                        character: full_match.end() as u32,
                    },
                });
            }
        }

        None
    }

    /// Find attribute usage at position
    fn find_attr_usage_at(&self, line: &str, line_num: u32, char_idx: usize) -> Option<Range> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let re = ATTR_RE.get_or_init(|| Regex::new(r"\{([\w\-]+)\}").unwrap());

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();

            if full_match.start() <= char_idx && char_idx < full_match.end() {
                return Some(Range {
                    start: Position {
                        line: line_num,
                        character: full_match.start() as u32,
                    },
                    end: Position {
                        line: line_num,
                        character: full_match.end() as u32,
                    },
                });
            }
        }

        None
    }

    /// Check if character is part of a word
    fn is_word_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
    }

    /// Get entire line range
    fn get_line_range(&self, line: u32) -> Range {
        let line_idx = line as usize;
        let end_char = if line_idx < self.lines.len() {
            self.lines[line_idx].len() as u32
        } else {
            0
        };

        Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: end_char,
            },
        }
    }

    /// Get logical block (paragraph, attribute block)
    fn get_logical_block(&self, cursor: Position) -> Option<Range> {
        let line_idx = cursor.line as usize;
        if line_idx >= self.lines.len() {
            return None;
        }

        // Check if we're in an attribute block
        if self.is_attribute_line(line_idx) {
            return self.get_attribute_block(line_idx);
        }

        // Check if we're in a paragraph
        self.get_paragraph_block(line_idx)
    }

    /// Get attribute block containing line
    fn get_attribute_block(&self, line_idx: usize) -> Option<Range> {
        // Scan up for first attribute
        let mut start = line_idx;
        while start > 0 && self.is_attribute_line(start - 1) {
            start -= 1;
        }

        // Scan down for last attribute
        let mut end = line_idx;
        while end + 1 < self.lines.len() && self.is_attribute_line(end + 1) {
            end += 1;
        }

        // Only return if we have multiple attributes
        if start < end {
            Some(Range {
                start: Position {
                    line: start as u32,
                    character: 0,
                },
                end: Position {
                    line: end as u32,
                    character: self.lines[end].len() as u32,
                },
            })
        } else {
            None
        }
    }

    /// Check if line is an attribute definition
    fn is_attribute_line(&self, line_idx: usize) -> bool {
        if line_idx >= self.lines.len() {
            return false;
        }

        let line = self.lines[line_idx].trim();
        if line.len() < 3 {
            return false;
        }

        // Match :name: pattern
        line.starts_with(':')
            && line
                .chars()
                .nth(1)
                .map(|c| c.is_alphanumeric())
                .unwrap_or(false)
            && line[1..].contains(':')
    }

    /// Get paragraph block containing line
    fn get_paragraph_block(&self, line_idx: usize) -> Option<Range> {
        if self.lines[line_idx].trim().is_empty() {
            return None;
        }

        // Find paragraph start (first non-empty line after a blank line or header)
        let mut start = line_idx;
        while start > 0 {
            let prev_line = self.lines[start - 1].trim();
            if prev_line.is_empty()
                || self.is_header_line(start - 1)
                || self.is_block_delimiter(start - 1)
            {
                break;
            }
            start -= 1;
        }

        // Find paragraph end (last non-empty line before a blank line or header)
        let mut end = line_idx;
        while end + 1 < self.lines.len() {
            let next_line = self.lines[end + 1].trim();
            if next_line.is_empty()
                || self.is_header_line(end + 1)
                || self.is_block_delimiter(end + 1)
            {
                break;
            }
            end += 1;
        }

        // Only return if paragraph spans multiple lines
        if start < end {
            Some(Range {
                start: Position {
                    line: start as u32,
                    character: 0,
                },
                end: Position {
                    line: end as u32,
                    character: self.lines[end].len() as u32,
                },
            })
        } else {
            None
        }
    }

    /// Check if line is a header
    fn is_header_line(&self, line_idx: usize) -> bool {
        if line_idx >= self.lines.len() {
            return false;
        }

        let line = self.lines[line_idx].trim();
        line.starts_with('=')
            && line.len() > 2
            && line
                .chars()
                .find(|c| *c != '=')
                .map(|c| c == ' ')
                .unwrap_or(false)
    }

    /// Check if line is a block delimiter
    fn is_block_delimiter(&self, line_idx: usize) -> bool {
        if line_idx >= self.lines.len() {
            return false;
        }

        let line = self.lines[line_idx].trim();
        if line.len() < 4 {
            return false;
        }

        let first = line.chars().next().unwrap();
        matches!(first, '-' | '.' | '=' | '*' | '_') && line.chars().all(|c| c == first)
    }

    /// Get structural blocks containing cursor (from folding ranges)
    fn get_structural_blocks_containing(&self, cursor: Position) -> Vec<Range> {
        self.folding_ranges
            .iter()
            .filter(|range| range.start.line <= cursor.line && cursor.line <= range.end.line)
            .cloned()
            .collect()
    }

    /// Get entire document range
    fn get_document_range(&self) -> Range {
        if self.lines.is_empty() {
            return Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            };
        }

        let last_line = self.lines.len() - 1;
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: last_line as u32,
                character: self.lines[last_line].len() as u32,
            },
        }
    }

    /// Convert to LSP SelectionRange hierarchy (linked via parent)
    pub fn to_lsp_selection_ranges(&self, cursor: Position) -> Option<SelectionRange> {
        let hierarchy = self.get_selection_hierarchy(cursor);
        if hierarchy.is_empty() {
            return None;
        }

        // Build from outermost to innermost, linking parents
        let mut result: Option<SelectionRange> = None;

        for level in hierarchy.iter().rev() {
            result = Some(SelectionRange {
                range: level.range,
                parent: result.map(Box::new),
            });
        }

        result
    }
}
