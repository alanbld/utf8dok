//! Workspace Knowledge Graph
//!
//! Stores definitions, references, and symbols across all workspace documents.
//! Enables cross-file navigation, validation, and refactoring.

use std::collections::{HashMap, HashSet};
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

    /// Map from document URI to file-based references (relative paths to .adoc files)
    document_file_refs: HashMap<String, Vec<String>>,

    /// All symbols (headers, anchors) for workspace symbol search
    symbols: Vec<WorkspaceSymbol>,

    /// Map from document URI to symbol indices (for cleanup)
    document_symbols: HashMap<String, Vec<usize>>,

    /// Map from document URI to its attributes (:name: value)
    document_attributes: HashMap<String, HashMap<String, String>>,

    /// Map from document URI to its full text content (for code actions)
    document_texts: HashMap<String, String>,
}

impl WorkspaceGraph {
    /// Create a new empty workspace graph
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            references: HashMap::new(),
            document_ids: HashMap::new(),
            document_refs: HashMap::new(),
            document_file_refs: HashMap::new(),
            symbols: Vec::new(),
            document_symbols: HashMap::new(),
            document_attributes: HashMap::new(),
            document_texts: HashMap::new(),
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

        // Extract file-based references (<<path/to/file.adoc#,...>>)
        let file_refs = WorkspaceIndexer::extract_file_references(content);
        let file_ref_paths: Vec<String> = file_refs.into_iter().map(|(path, _, _)| path).collect();
        if !file_ref_paths.is_empty() {
            self.document_file_refs
                .insert(uri.to_string(), file_ref_paths);
        }

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

        // Extract document attributes
        let attrs = WorkspaceIndexer::extract_attributes(content);
        if !attrs.is_empty() {
            self.document_attributes.insert(uri.to_string(), attrs);
        }

        // Store document text for code actions
        self.document_texts
            .insert(uri.to_string(), content.to_string());
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

        // Remove file references
        self.document_file_refs.remove(uri);

        // Remove attributes
        self.document_attributes.remove(uri);

        // Remove text
        self.document_texts.remove(uri);
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
                                    message: format!("Broken reference: '{}' is not defined", id),
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

    /// Get the number of documents in the graph
    #[allow(dead_code)]
    pub fn document_count(&self) -> usize {
        self.document_ids.len()
    }

    // ==================== COMPLIANCE ENGINE ACCESSORS ====================

    /// Get all document URIs in the graph
    #[allow(dead_code)]
    pub fn document_uris(&self) -> Vec<&String> {
        self.document_ids.keys().collect()
    }

    /// Get attributes for a specific document
    #[allow(dead_code)]
    pub fn get_document_attributes(&self, uri: &str) -> Option<&HashMap<String, String>> {
        self.document_attributes.get(uri)
    }

    /// Get a specific attribute value for a document
    #[allow(dead_code)]
    pub fn get_document_attribute(&self, uri: &str, attr_name: &str) -> Option<&String> {
        self.document_attributes
            .get(uri)
            .and_then(|attrs| attrs.get(attr_name))
    }

    /// Get all IDs defined in a document
    #[allow(dead_code)]
    pub fn get_document_ids(&self, uri: &str) -> Option<&Vec<String>> {
        self.document_ids.get(uri)
    }

    /// Get all reference IDs in a document
    #[allow(dead_code)]
    pub fn get_document_refs(&self, uri: &str) -> Option<&Vec<String>> {
        self.document_refs.get(uri)
    }

    /// Check if an ID is defined anywhere in the workspace
    #[allow(dead_code)]
    pub fn is_id_defined(&self, id: &str) -> bool {
        self.definitions.contains_key(id)
    }

    /// Get the URI where an ID is defined
    #[allow(dead_code)]
    pub fn get_definition_uri(&self, id: &str) -> Option<&Url> {
        self.definitions.get(id).map(|loc| &loc.uri)
    }

    /// Get all documents that reference a given ID
    #[allow(dead_code)]
    pub fn get_referencing_documents(&self, id: &str) -> Vec<&Url> {
        self.references
            .get(id)
            .map(|locs| locs.iter().map(|loc| &loc.uri).collect())
            .unwrap_or_default()
    }

    /// Get the full text content of a document
    #[allow(dead_code)]
    pub fn get_document_text(&self, uri: &str) -> Option<&String> {
        self.document_texts.get(uri)
    }

    /// Find all documents reachable from entry points via references
    /// Returns the set of reachable document URIs
    #[allow(dead_code)]
    pub fn find_reachable_documents(&self, entry_points: &[&str]) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue: Vec<String> = Vec::new();

        // Start with entry points
        for entry in entry_points {
            if self.document_ids.contains_key(*entry) {
                reachable.insert(entry.to_string());
                queue.push(entry.to_string());
            }
        }

        // BFS traversal
        while let Some(current_uri) = queue.pop() {
            // Get all ID-based references from this document
            if let Some(refs) = self.document_refs.get(&current_uri) {
                for ref_id in refs {
                    // Find which document defines this ID
                    if let Some(def_loc) = self.definitions.get(ref_id) {
                        let def_uri = def_loc.uri.as_str().to_string();
                        if !reachable.contains(&def_uri) {
                            reachable.insert(def_uri.clone());
                            queue.push(def_uri);
                        }
                    }
                }
            }

            // Get all file-based references from this document
            if let Some(file_refs) = self.document_file_refs.get(&current_uri) {
                // Resolve relative paths against current document's URI
                if let Ok(current_url) = Url::parse(&current_uri) {
                    for file_ref in file_refs {
                        // Resolve relative path against parent directory of current doc
                        if let Some(resolved_uri) =
                            self.resolve_file_reference(&current_url, file_ref)
                        {
                            if !reachable.contains(&resolved_uri) {
                                reachable.insert(resolved_uri.clone());
                                queue.push(resolved_uri);
                            }
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Resolve a relative file reference against a base URI
    fn resolve_file_reference(&self, base_uri: &Url, relative_path: &str) -> Option<String> {
        // Get the base directory (parent of the current file)
        let base_path = base_uri.path();
        let base_dir = base_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");

        // Construct the resolved path
        let resolved_path = if relative_path.starts_with('/') {
            relative_path.to_string()
        } else {
            format!("{}/{}", base_dir, relative_path)
        };

        // Normalize the path (handle ../ and ./)
        let normalized = Self::normalize_path(&resolved_path);

        // Create the full URI
        let resolved_uri = format!(
            "{}://{}{}",
            base_uri.scheme(),
            base_uri.host_str().unwrap_or(""),
            normalized
        );

        // Check if this document exists in our graph
        if self.document_ids.contains_key(&resolved_uri) {
            Some(resolved_uri)
        } else {
            None
        }
    }

    /// Normalize a path by resolving . and .. components
    fn normalize_path(path: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();

        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                _ => parts.push(part),
            }
        }

        format!("/{}", parts.join("/"))
    }

    /// Get file references for a document
    #[allow(dead_code)]
    pub fn get_document_file_refs(&self, uri: &str) -> Option<&Vec<String>> {
        self.document_file_refs.get(uri)
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

    #[test]
    fn test_file_reference_tracking() {
        let mut graph = WorkspaceGraph::new();

        // Add index with file references
        graph.add_document(
            "file:///docs/index.adoc",
            "= Index\n\n* <<adr/0001-arch.adoc#,ADR 0001>>\n* <<adr/0002-lsp.adoc#,ADR 0002>>",
        );

        // Check file references were extracted
        let file_refs = graph.get_document_file_refs("file:///docs/index.adoc");
        assert!(file_refs.is_some());
        let refs = file_refs.unwrap();
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"adr/0001-arch.adoc".to_string()));
        assert!(refs.contains(&"adr/0002-lsp.adoc".to_string()));
    }

    #[test]
    fn test_reachable_via_file_refs() {
        let mut graph = WorkspaceGraph::new();

        // Add index with file references
        graph.add_document(
            "file:///docs/index.adoc",
            "[[index]]\n= Index\n\n* <<adr/0001-arch.adoc#,ADR 0001>>",
        );

        // Add the referenced ADR
        graph.add_document(
            "file:///docs/adr/0001-arch.adoc",
            "[[adr-0001]]\n= ADR 0001: Architecture",
        );

        // Find reachable from index
        let reachable = graph.find_reachable_documents(&["file:///docs/index.adoc"]);

        assert!(reachable.contains("file:///docs/index.adoc"));
        assert!(reachable.contains("file:///docs/adr/0001-arch.adoc"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(WorkspaceGraph::normalize_path("/a/b/c"), "/a/b/c");
        assert_eq!(WorkspaceGraph::normalize_path("/a/b/../c"), "/a/c");
        assert_eq!(WorkspaceGraph::normalize_path("/a/./b/c"), "/a/b/c");
        assert_eq!(WorkspaceGraph::normalize_path("/a/b/c/../.."), "/a");
    }
}
