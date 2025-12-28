//! Line-by-line structural scanner for AsciiDoc documents
//!
//! Classifies each line's structural role for folding and symbol analysis.

use regex::Regex;
use std::sync::OnceLock;

/// Classification of a line's structural role
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineType {
    /// AsciiDoc header with depth (1-6)
    Header(u8),
    /// Attribute definition (:name: value)
    Attribute,
    /// Block delimiter (----, ...., etc.)
    BlockDelimiter,
    /// Anything else (regular text, lists, etc.)
    Other,
}

/// Structural scanner for AsciiDoc lines
pub struct StructuralScanner;

impl StructuralScanner {
    /// Analyze a single line to determine its structural type
    pub fn scan(line: &str) -> LineType {
        static HEADER_RE: OnceLock<Regex> = OnceLock::new();
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        static BLOCK_RE: OnceLock<Regex> = OnceLock::new();

        // Lazy compile regexes once
        let header_re = HEADER_RE.get_or_init(|| {
            // Match = through ====== followed by space and text
            Regex::new(r"^(={1,6})\s+\S").unwrap()
        });

        let attr_re = ATTR_RE.get_or_init(|| {
            // Match :name: or :name:: (escaped colon)
            Regex::new(r"^:([\w][\w-]*):{1,2}\s").unwrap()
        });

        let block_re = BLOCK_RE.get_or_init(|| {
            // Match 4+ repeated chars for block delimiters
            Regex::new(r"^(-{4,}|\.{4,}|={4,}|\*{4,}|_{4,})$").unwrap()
        });

        let trimmed = line.trim();

        if trimmed.is_empty() {
            return LineType::Other;
        }

        // Check header first (most specific)
        if let Some(caps) = header_re.captures(trimmed) {
            let level = caps[1].len();
            if level <= 6 {
                return LineType::Header(level as u8);
            }
        }

        // Check attribute
        if attr_re.is_match(trimmed) {
            return LineType::Attribute;
        }

        // Check block delimiter
        if block_re.is_match(trimmed) {
            return LineType::BlockDelimiter;
        }

        LineType::Other
    }
}
