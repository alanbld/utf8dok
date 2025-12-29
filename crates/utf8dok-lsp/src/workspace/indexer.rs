//! Workspace Indexer
//!
//! Scans document content to extract definitions, references, and symbols.

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Workspace indexer for extracting structural information from documents
pub struct WorkspaceIndexer;

impl WorkspaceIndexer {
    /// Extract all anchor definitions ([[id]]) from content
    /// Returns Vec<(id, line, column)>
    pub fn extract_definitions(content: &str) -> Vec<(String, usize, usize)> {
        static ANCHOR_RE: OnceLock<Regex> = OnceLock::new();
        let anchor_re = ANCHOR_RE.get_or_init(|| Regex::new(r"\[\[([\w\-]+)\]\]").unwrap());

        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in anchor_re.captures_iter(line) {
                if let Some(id_match) = cap.get(1) {
                    let id = id_match.as_str().to_string();
                    let col = id_match.start();
                    results.push((id, line_num, col));
                }
            }
        }

        results
    }

    /// Extract all cross-references (<<id>>) from content
    /// Returns Vec<(id, line, column)>
    pub fn extract_references(content: &str) -> Vec<(String, usize, usize)> {
        static XREF_RE: OnceLock<Regex> = OnceLock::new();
        let xref_re = XREF_RE.get_or_init(|| Regex::new(r"<<([\w\-]+)(?:,[^>]*)?>?>").unwrap());

        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in xref_re.captures_iter(line) {
                if let Some(id_match) = cap.get(1) {
                    let id = id_match.as_str().to_string();
                    // Column is the start of the << plus 2 for the <<
                    let col = cap.get(0).map(|m| m.start() + 2).unwrap_or(0);
                    results.push((id, line_num, col));
                }
            }
        }

        results
    }

    /// Extract all file-based cross-references (<<path/to/file.adoc#,...>>) from content
    /// Returns Vec<(relative_path, line, column)>
    pub fn extract_file_references(content: &str) -> Vec<(String, usize, usize)> {
        static FILE_XREF_RE: OnceLock<Regex> = OnceLock::new();
        // Match <<path/to/file.adoc#anchor,label>> or <<path/to/file.adoc#,label>>
        let file_xref_re = FILE_XREF_RE.get_or_init(|| {
            Regex::new(r"<<([^#,>\s]+\.adoc)(?:#[^,>]*)?(?:,[^>]*)?\s*>>").unwrap()
        });

        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in file_xref_re.captures_iter(line) {
                if let Some(path_match) = cap.get(1) {
                    let path = path_match.as_str().to_string();
                    let col = cap.get(0).map(|m| m.start() + 2).unwrap_or(0);
                    results.push((path, line_num, col));
                }
            }
        }

        results
    }

    /// Extract all headers from content
    /// Returns Vec<(title, line, level)> where level is the number of '=' signs
    pub fn extract_headers(content: &str) -> Vec<(String, usize, usize)> {
        static HEADER_RE: OnceLock<Regex> = OnceLock::new();
        let header_re = HEADER_RE.get_or_init(|| Regex::new(r"^(=+)\s+(.+)$").unwrap());

        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = header_re.captures(line) {
                let level = cap.get(1).map(|m| m.as_str().len()).unwrap_or(1);
                let title = cap
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                results.push((title, line_num, level));
            }
        }

        results
    }

    /// Extract document attributes (:name: value)
    /// Returns HashMap<name, value>
    pub fn extract_attributes(content: &str) -> HashMap<String, String> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let attr_re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w\-]+):\s*(.*)$").unwrap());

        let mut attrs = HashMap::new();

        for line in content.lines() {
            if let Some(cap) = attr_re.captures(line) {
                let name = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let value = cap
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                if !name.is_empty() {
                    attrs.insert(name, value);
                }
            }
        }

        attrs
    }

    /// Extract both definitions and headers combined
    /// Useful for building a complete symbol table
    #[allow(dead_code)]
    pub fn extract_all_symbols(content: &str) -> Vec<(String, usize, SymbolType)> {
        let mut symbols = Vec::new();

        // Add anchors
        for (id, line, _) in Self::extract_definitions(content) {
            symbols.push((id, line, SymbolType::Anchor));
        }

        // Add headers
        for (title, line, level) in Self::extract_headers(content) {
            symbols.push((title, line, SymbolType::Header(level)));
        }

        // Sort by line number
        symbols.sort_by_key(|(_, line, _)| *line);

        symbols
    }
}

/// Type of symbol extracted
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    /// An anchor definition [[id]]
    Anchor,
    /// A header with level (number of '=' signs)
    Header(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_anchor() {
        let content = "[[my-id]]\n== Section";
        let defs = WorkspaceIndexer::extract_definitions(content);

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].0, "my-id");
        assert_eq!(defs[0].1, 0); // line 0
    }

    #[test]
    fn test_extract_multiple_anchors() {
        let content = "[[first]]\n== One\n\n[[second]]\n== Two";
        let defs = WorkspaceIndexer::extract_definitions(content);

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].0, "first");
        assert_eq!(defs[1].0, "second");
    }

    #[test]
    fn test_extract_references() {
        let content = "See <<ref-one>> and <<ref-two,label>>";
        let refs = WorkspaceIndexer::extract_references(content);

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].0, "ref-one");
        assert_eq!(refs[1].0, "ref-two");
    }

    #[test]
    fn test_extract_headers() {
        let content = "= Title\n\n== Section\n\n=== Subsection";
        let headers = WorkspaceIndexer::extract_headers(content);

        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0], ("Title".to_string(), 0, 1));
        assert_eq!(headers[1], ("Section".to_string(), 2, 2));
        assert_eq!(headers[2], ("Subsection".to_string(), 4, 3));
    }

    #[test]
    fn test_anchor_with_hyphen() {
        let content = "[[my-long-id]]";
        let defs = WorkspaceIndexer::extract_definitions(content);

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].0, "my-long-id");
    }

    #[test]
    fn test_inline_anchor() {
        let content = "Some text [[inline]] more text";
        let defs = WorkspaceIndexer::extract_definitions(content);

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].0, "inline");
    }

    #[test]
    fn test_extract_file_references() {
        let content = "* <<adr/0001-architecture.adoc#,ADR 0001: Architecture>>\n* <<adr/0002-lsp-first.adoc#,ADR 0002: LSP-First>>";
        let refs = WorkspaceIndexer::extract_file_references(content);

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].0, "adr/0001-architecture.adoc");
        assert_eq!(refs[0].1, 0); // line 0
        assert_eq!(refs[1].0, "adr/0002-lsp-first.adoc");
        assert_eq!(refs[1].1, 1); // line 1
    }

    #[test]
    fn test_extract_file_references_with_anchor() {
        let content = "See <<docs/guide.adoc#section-one,Section One>> for details.";
        let refs = WorkspaceIndexer::extract_file_references(content);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].0, "docs/guide.adoc");
    }
}
