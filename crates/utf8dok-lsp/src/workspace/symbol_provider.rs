//! Workspace Symbol Provider
//!
//! Provides LSP workspace/symbol functionality using the knowledge graph.

use super::graph::{SymbolKind, WorkspaceGraph};
use tower_lsp::lsp_types::{SymbolInformation, SymbolKind as LspSymbolKind, WorkspaceSymbolParams};

/// Provides workspace symbol search functionality
pub struct SymbolProvider<'a> {
    graph: &'a WorkspaceGraph,
}

impl<'a> SymbolProvider<'a> {
    /// Create a new symbol provider backed by a workspace graph
    #[allow(dead_code)]
    pub fn new(graph: &'a WorkspaceGraph) -> Self {
        Self { graph }
    }

    /// Search for symbols matching the query
    #[allow(deprecated)]
    pub fn workspace_symbols(&self, query: &str) -> Vec<SymbolInformation> {
        self.graph
            .query_symbols(query)
            .into_iter()
            .map(|sym| SymbolInformation {
                name: sym.name.clone(),
                kind: Self::convert_symbol_kind(sym.kind),
                tags: None,
                deprecated: None,
                location: sym.location.clone(),
                container_name: None,
            })
            .collect()
    }

    /// Handle LSP workspace/symbol request
    #[allow(dead_code)]
    pub fn handle_request(&self, params: &WorkspaceSymbolParams) -> Vec<SymbolInformation> {
        self.workspace_symbols(&params.query)
    }

    /// Convert our symbol kind to LSP symbol kind
    pub fn convert_symbol_kind(kind: SymbolKind) -> LspSymbolKind {
        match kind {
            SymbolKind::Title => LspSymbolKind::FILE,
            SymbolKind::Header1 => LspSymbolKind::CLASS,
            SymbolKind::Header2 => LspSymbolKind::METHOD,
            SymbolKind::Header3Plus => LspSymbolKind::FUNCTION,
            SymbolKind::Anchor => LspSymbolKind::CONSTANT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_provider_basic() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///test.adoc", "== Test Section");

        let provider = SymbolProvider::new(&graph);
        let symbols = provider.workspace_symbols("Test");

        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name, "Test Section");
    }

    #[test]
    fn test_symbol_kind_conversion() {
        assert_eq!(
            SymbolProvider::convert_symbol_kind(SymbolKind::Title),
            LspSymbolKind::FILE
        );
        assert_eq!(
            SymbolProvider::convert_symbol_kind(SymbolKind::Header1),
            LspSymbolKind::CLASS
        );
    }
}
