//! Workspace Knowledge Graph
//!
//! Stores definitions, references, and symbols across all workspace documents.
//! Enables cross-file navigation, validation, and refactoring.

use std::collections::HashMap;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Location, Position, Range, Url};

use super::indexer::WorkspaceIndexer;

/// A symbol in the workspace (header, section, etc.)
#[derive(Debug, Clone)]
pub struct WorkspaceSymbol {
    /// The symbol name (e.g., "System Architecture")
    pub name: String,
    /// The symbol kind (header level, etc.)
    pub kind: SymbolKind,
    /// Where this symbol is defined
    pub location: Location,
}

/// Kind of workspace symbol
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum SymbolKind {
    /// Document title (= Title)
    Title,
    /// Level 1 header (== Header)
    Header1,
    /// Level 2 header (=== Header)
    Header2,
    /// Level 3+ header
    Header3Plus,
    /// Anchor definition [[id]]
    Anchor,
}

impl SymbolKind {
    /// Convert from header level (number of '=' signs)
    pub fn from_level(level: usize) -> Self {
        match level {
            1 => SymbolKind::Title,
            2 => SymbolKind::Header1,
            3 => SymbolKind::Header2,
            _ => SymbolKind::Header3Plus,
        }
    }
}

/// The main workspace knowledge graph
pub struct WorkspaceGraph {
    /// Map from ID to its definition location
    definitions: HashMap<String, Location>,

    /// Map from ID to all locations that reference it
    references: HashMap<String, Vec<Location>>,

    /// Map from document URI to all IDs defined in it (for cleanup on update)
    document_ids: HashMap<String, Vec<String>>,

    /// Map from document URI to all references in it (for cleanup on update)
    document_refs: HashMap<String, Vec<String>>,

    /// All symbols (headers, anchors) for workspace symbol search
    symbols: Vec<WorkspaceSymbol>,

    /// Map from document URI to symbol indices (for cleanup)
    document_symbols: HashMap<String, Vec<usize>>,
}

impl WorkspaceGraph {
    /// Create a new empty workspace graph
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            references: HashMap::new(),
            document_ids: HashMap::new(),
            document_refs: HashMap::new(),
            symbols: Vec::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add or update a document in the graph
    pub fn add_document(&mut self, uri: &str, content: &str) {
        // First, remove old data for this document
        self.remove_document(uri);

        let parsed_uri = match Url::parse(uri) {
            Ok(u) => u,
            Err(_) => return,
        };

        // Extract definitions (anchors)
        let defs = WorkspaceIndexer::extract_definitions(content);
        let mut doc_ids = Vec::new();

        for (id, line, _col) in defs {
            let location = Location {
                uri: parsed_uri.clone(),
                range: Range {
                    start: Position {
                        line: line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line as u32,
                        character: 0,
                    },
                },
            };
            self.definitions.insert(id.clone(), location);
            doc_ids.push(id);
        }
        self.document_ids.insert(uri.to_string(), doc_ids);

        // Extract references
        let refs = WorkspaceIndexer::extract_references(content);
        let mut doc_refs = Vec::new();

        for (id, line, col) in refs {
            let location = Location {
                uri: parsed_uri.clone(),
                range: Range {
                    start: Position {
                        line: line as u32,
                        character: col as u32,
                    },
                    end: Position {
                        line: line as u32,
                        character: (col + id.len()) as u32,
                    },
                },
            };
            self.references
                .entry(id.clone())
                .or_default()
                .push(location);
            doc_refs.push(id);
        }
        self.document_refs.insert(uri.to_string(), doc_refs);

        // Extract headers as symbols
        let headers = WorkspaceIndexer::extract_headers(content);
        let mut symbol_indices = Vec::new();

        for (name, line, level) in headers {
            let symbol = WorkspaceSymbol {
                name,
                kind: SymbolKind::from_level(level),
                location: Location {
                    uri: parsed_uri.clone(),
                    range: Range {
                        start: Position {
                            line: line as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line as u32,
                            character: 0,
                        },
                    },
                },
            };
            self.symbols.push(symbol);
            symbol_indices.push(self.symbols.len() - 1);
        }

        if !symbol_indices.is_empty() {
            self.document_symbols
                .insert(uri.to_string(), symbol_indices);
        }
    }

    /// Remove a document from the graph
    pub fn remove_document(&mut self, uri: &str) {
        // Remove definitions
        if let Some(ids) = self.document_ids.remove(uri) {
            for id in ids {
                self.definitions.remove(&id);
            }
        }

        // Remove references
        if let Some(ref_ids) = self.document_refs.remove(uri) {
            // Parse URI once
            let parsed_uri = Url::parse(uri).ok();

            for id in ref_ids {
                if let Some(refs) = self.references.get_mut(&id) {
                    if let Some(ref parsed) = parsed_uri {
                        refs.retain(|loc| &loc.uri != parsed);
                    }
                    if refs.is_empty() {
                        self.references.remove(&id);
                    }
                }
            }
        }

        // Remove symbols (mark as removed by setting empty name)
        if let Some(indices) = self.document_symbols.remove(uri) {
            for idx in indices {
                if idx < self.symbols.len() {
                    self.symbols[idx].name.clear();
                }
            }
        }
    }

    /// Resolve an ID to its definition location
    #[allow(dead_code)]
    pub fn resolve_id(&self, id: &str) -> Option<Location> {
        self.definitions.get(id).cloned()
    }

    /// Find all references to an ID
    #[allow(dead_code)]
    pub fn find_references(&self, id: &str) -> Vec<Location> {
        self.references.get(id).cloned().unwrap_or_default()
    }

    /// Query symbols matching a pattern (case-insensitive substring match)
    pub fn query_symbols(&self, query: &str) -> Vec<&WorkspaceSymbol> {
        let query_lower = query.to_lowercase();

        self.symbols
            .iter()
            .filter(|s| {
                !s.name.is_empty()
                    && (query.is_empty() || s.name.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Validate all links in a specific document
    #[allow(dead_code)]
    pub fn validate_links(&self, uri: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(ref_ids) = self.document_refs.get(uri) {
            // Get unique references with their locations
            let parsed_uri = match Url::parse(uri) {
                Ok(u) => u,
                Err(_) => return diagnostics,
            };

            // Find all references in this document
            for id in ref_ids {
                // Check if definition exists
                if !self.definitions.contains_key(id) {
                    // Find the location of this reference in the document
                    if let Some(refs) = self.references.get(id) {
                        for loc in refs {
                            if loc.uri == parsed_uri {
                                diagnostics.push(Diagnostic {
                                    range: loc.range,
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    code: Some(tower_lsp::lsp_types::NumberOrString::String(
                                        "WS001".to_string(),
                                    )),
                                    source: Some("utf8dok-workspace".to_string()),
                                    message: format!(
                                        "Broken reference: '{}' is not defined",
                                        id
                                    ),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Validate all links across all documents
    #[allow(dead_code)]
    pub fn validate_all_links(&self) -> Vec<(String, Vec<Diagnostic>)> {
        let mut all_diagnostics = Vec::new();

        for uri in self.document_refs.keys() {
            let diagnostics = self.validate_links(uri);
            if !diagnostics.is_empty() {
                all_diagnostics.push((uri.clone(), diagnostics));
            }
        }

        all_diagnostics
    }

    /// Get statistics about the graph
    #[allow(dead_code)]
    pub fn stats(&self) -> GraphStats {
        GraphStats {
            documents: self.document_ids.len(),
            definitions: self.definitions.len(),
            references: self.references.values().map(|v| v.len()).sum(),
            symbols: self.symbols.iter().filter(|s| !s.name.is_empty()).count(),
        }
    }
}

impl Default for WorkspaceGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the workspace graph
#[allow(dead_code)]
pub struct GraphStats {
    pub documents: usize,
    pub definitions: usize,
    pub references: usize,
    pub symbols: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_add_document() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///test.adoc", "[[id]]\n== Title");

        assert!(graph.resolve_id("id").is_some());
    }

    #[test]
    fn test_symbol_kind_from_level() {
        assert_eq!(SymbolKind::from_level(1), SymbolKind::Title);
        assert_eq!(SymbolKind::from_level(2), SymbolKind::Header1);
        assert_eq!(SymbolKind::from_level(3), SymbolKind::Header2);
        assert_eq!(SymbolKind::from_level(4), SymbolKind::Header3Plus);
    }
}
