//! Document symbol generation for AsciiDoc documents
//!
//! Generates LSP DocumentSymbol hierarchy for outline view.
//! Symbols are extracted from:
//! - Headers (=, ==, ===, etc.) → MODULE, NAMESPACE, CLASS hierarchy
//! - Attributes (:name: value) → VARIABLE with value as detail
//! - Code blocks (----, ....) → OBJECT with line count

use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

/// Main analyzer for extracting document symbols
pub struct SymbolAnalyzer;

impl SymbolAnalyzer {
    /// Extract symbols from document text
    pub fn extract_symbols(text: &str) -> Vec<DocumentSymbol> {
        if text.trim().is_empty() {
            return Vec::new();
        }

        let lines: Vec<&str> = text.lines().collect();
        let mut symbols: Vec<DocumentSymbol> = Vec::new();
        // Stack stores (level, path) where path is indices to navigate to the symbol
        let mut header_stack: Vec<(usize, Vec<usize>)> = Vec::new();

        // First pass: extract attributes at document start
        let mut in_attribute_block = true;
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            // Check if this is an attribute
            if in_attribute_block {
                if let Some((name, value)) = Self::parse_attribute(trimmed) {
                    #[allow(deprecated)]
                    let symbol = DocumentSymbol {
                        name,
                        detail: Some(value),
                        kind: SymbolKind::VARIABLE,
                        tags: None,
                        deprecated: None,
                        range: Self::line_range(line_num, line),
                        selection_range: Self::line_range(line_num, line),
                        children: None,
                    };
                    symbols.push(symbol);
                    continue;
                } else {
                    // Non-attribute line encountered, stop looking for attributes
                    in_attribute_block = false;
                }
            }

            // Check for header
            if let Some((level, title)) = Self::parse_header(trimmed) {
                let kind = Self::header_kind(level);
                #[allow(deprecated)]
                let symbol = DocumentSymbol {
                    name: title,
                    detail: None,
                    kind,
                    tags: None,
                    deprecated: None,
                    range: Self::line_range(line_num, line),
                    selection_range: Self::line_range(line_num, line),
                    children: Some(Vec::new()),
                };

                // Pop all headers at same or deeper level
                while let Some((stack_level, _)) = header_stack.last() {
                    if *stack_level >= level {
                        header_stack.pop();
                    } else {
                        break;
                    }
                }

                if let Some((_, parent_path)) = header_stack.last().cloned() {
                    // Add as child of parent, get new path
                    let new_path = Self::add_child_at_path(&mut symbols, &parent_path, symbol);
                    header_stack.push((level, new_path));
                } else {
                    // Add at root level
                    let idx = symbols.len();
                    symbols.push(symbol);
                    header_stack.push((level, vec![idx]));
                }
                continue;
            }

            // Check for block delimiter
            if Self::is_block_delimiter(trimmed) {
                if let Some(end_line) = Self::find_closing_delimiter(line_num, &lines) {
                    let line_count = end_line - line_num + 1;
                    #[allow(deprecated)]
                    let block_symbol = DocumentSymbol {
                        name: "[listing block]".to_string(),
                        detail: Some(format!("{} lines", line_count)),
                        kind: SymbolKind::OBJECT,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: 0,
                            },
                            end: Position {
                                line: end_line as u32,
                                character: lines[end_line].len() as u32,
                            },
                        },
                        selection_range: Self::line_range(line_num, line),
                        children: None,
                    };

                    // Add block to current section if there is one
                    if let Some((_, parent_path)) = header_stack.last().cloned() {
                        Self::add_child_at_path(&mut symbols, &parent_path, block_symbol);
                    }
                    // Note: We don't add blocks at root level to keep outline clean
                }
            }
        }

        symbols
    }

    /// Add a child symbol at the given path, returning the new child's path
    fn add_child_at_path(
        symbols: &mut [DocumentSymbol],
        path: &[usize],
        child: DocumentSymbol,
    ) -> Vec<usize> {
        if path.is_empty() {
            return Vec::new();
        }

        // Navigate to parent using path
        let mut current = &mut symbols[path[0]];
        for &idx in &path[1..] {
            if let Some(children) = &mut current.children {
                current = &mut children[idx];
            } else {
                return Vec::new();
            }
        }

        // Add child to current node
        if let Some(children) = &mut current.children {
            let child_idx = children.len();
            children.push(child);
            // Return new path = parent_path + [child_idx]
            let mut new_path = path.to_vec();
            new_path.push(child_idx);
            return new_path;
        }

        Vec::new()
    }

    /// Parse attribute line (:name: value)
    fn parse_attribute(line: &str) -> Option<(String, String)> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w][\w-]*):\s*(.*)$").unwrap());

        if let Some(caps) = re.captures(line) {
            let name = caps[1].to_string();
            let value = caps[2].trim().to_string();
            return Some((name, value));
        }
        None
    }

    /// Parse header line (= Title)
    fn parse_header(line: &str) -> Option<(usize, String)> {
        let trimmed = line.trim();
        if !trimmed.starts_with('=') {
            return None;
        }

        let level = trimmed.chars().take_while(|c| *c == '=').count();
        if !(1..=6).contains(&level) {
            return None;
        }

        let title = trimmed[level..].trim().to_string();
        if title.is_empty() {
            return None;
        }

        Some((level, title))
    }

    /// Determine SymbolKind for header level
    fn header_kind(level: usize) -> SymbolKind {
        match level {
            1 => SymbolKind::MODULE,
            2 => SymbolKind::NAMESPACE,
            3 => SymbolKind::CLASS,
            4 => SymbolKind::INTERFACE,
            5 => SymbolKind::ENUM,
            6 => SymbolKind::STRUCT,
            _ => SymbolKind::MODULE,
        }
    }

    /// Check if line is a block delimiter (4+ repeated chars)
    fn is_block_delimiter(line: &str) -> bool {
        if line.len() < 4 {
            return false;
        }
        let chars: Vec<char> = line.chars().collect();
        let first = chars[0];
        if !matches!(first, '-' | '.' | '=' | '*' | '_') {
            return false;
        }
        chars.iter().all(|c| *c == first)
    }

    /// Find closing delimiter for a block
    fn find_closing_delimiter(start_line: usize, lines: &[&str]) -> Option<usize> {
        let opener = lines[start_line].trim();
        if !Self::is_block_delimiter(opener) {
            return None;
        }

        let opener_char = opener.chars().next()?;
        let opener_len = opener.len();

        for (i, &raw_line) in lines.iter().enumerate().skip(start_line + 1) {
            let line = raw_line.trim();
            if Self::is_block_delimiter(line)
                && line.starts_with(opener_char)
                && line.len() >= opener_len
            {
                return Some(i);
            }
        }

        None
    }

    /// Create a range for a single line
    fn line_range(line_num: usize, line: &str) -> Range {
        Range {
            start: Position {
                line: line_num as u32,
                character: 0,
            },
            end: Position {
                line: line_num as u32,
                character: line.len() as u32,
            },
        }
    }
}
