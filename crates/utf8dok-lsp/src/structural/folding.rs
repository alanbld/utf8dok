//! Folding range generation for AsciiDoc documents
//!
//! Generates LSP folding ranges for:
//! - Attribute groups (consecutive :name: value lines)
//! - Header sections (based on hierarchy)
//! - Delimited blocks (----, ...., etc.)

use super::scanner::{LineType, StructuralScanner};
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

/// Analyzer for generating folding ranges
pub struct FoldingAnalyzer;

impl FoldingAnalyzer {
    /// Generate folding ranges for the entire document
    pub fn generate_ranges(text: &str) -> Vec<FoldingRange> {
        let lines: Vec<&str> = text.lines().collect();
        let mut ranges = Vec::new();

        if lines.is_empty() {
            return ranges;
        }

        // Scan all lines once
        let line_types: Vec<LineType> = lines
            .iter()
            .map(|line| StructuralScanner::scan(line))
            .collect();

        // Track state for different fold types
        let mut attr_group_start: Option<usize> = None;
        let mut header_stack: Vec<(u8, usize)> = Vec::new(); // (level, start_line)
        let mut block_start: Option<usize> = None;

        for (i, line_type) in line_types.iter().enumerate() {
            // ----- ATTRIBUTE GROUP LOGIC -----
            match line_type {
                LineType::Attribute => {
                    if attr_group_start.is_none() {
                        attr_group_start = Some(i);
                    }
                }
                _ => {
                    if let Some(start) = attr_group_start {
                        // Only create fold if we have at least 2 consecutive attributes
                        if i - start >= 2 {
                            ranges.push(FoldingRange {
                                start_line: start as u32,
                                end_line: (i - 1) as u32,
                                kind: Some(FoldingRangeKind::Imports),
                                start_character: None,
                                end_character: None,
                                collapsed_text: None,
                            });
                        }
                        attr_group_start = None;
                    }
                }
            }

            // ----- HEADER HIERARCHY LOGIC -----
            if let LineType::Header(level) = line_type {
                // Close any headers at this level or deeper
                while let Some((stack_level, start_line)) = header_stack.last() {
                    if *stack_level >= *level {
                        // Only create fold if there's content between start and current line
                        if i > *start_line + 1 {
                            ranges.push(FoldingRange {
                                start_line: *start_line as u32,
                                end_line: (i - 1) as u32,
                                kind: Some(FoldingRangeKind::Region),
                                start_character: None,
                                end_character: None,
                                collapsed_text: None,
                            });
                        }
                        header_stack.pop();
                    } else {
                        break;
                    }
                }
                header_stack.push((*level, i));
            }

            // ----- BLOCK DELIMITER LOGIC -----
            if let LineType::BlockDelimiter = line_type {
                if let Some(start) = block_start {
                    // Closing delimiter
                    ranges.push(FoldingRange {
                        start_line: start as u32,
                        end_line: i as u32,
                        kind: Some(FoldingRangeKind::Region),
                        start_character: None,
                        end_character: None,
                        collapsed_text: None,
                    });
                    block_start = None;
                } else {
                    // Opening delimiter
                    block_start = Some(i);
                }
            }
        }

        // ----- CLEANUP AT DOCUMENT END -----
        let last_line = lines.len() - 1;

        // Close trailing attribute group
        if let Some(start) = attr_group_start {
            if last_line > start {
                ranges.push(FoldingRange {
                    start_line: start as u32,
                    end_line: last_line as u32,
                    kind: Some(FoldingRangeKind::Imports),
                    start_character: None,
                    end_character: None,
                    collapsed_text: None,
                });
            }
        }

        // Close trailing headers
        while let Some((_, start_line)) = header_stack.pop() {
            if last_line > start_line {
                ranges.push(FoldingRange {
                    start_line: start_line as u32,
                    end_line: last_line as u32,
                    kind: Some(FoldingRangeKind::Region),
                    start_character: None,
                    end_character: None,
                    collapsed_text: None,
                });
            }
        }

        // Note: Trailing block delimiters left open intentionally
        // (incomplete blocks shouldn't create misleading folds)

        ranges
    }
}
