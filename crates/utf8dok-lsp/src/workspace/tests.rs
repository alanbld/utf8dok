//! TDD Tests for Workspace Intelligence (Phase 11)
//!
//! Tests the knowledge graph for cross-file navigation, validation, and refactoring.

// ==================== GRAPH CONSTRUCTION TESTS ====================

mod graph_tests {
    use crate::workspace::graph::WorkspaceGraph;

    /// Test 1: Indexing a Multi-File Project
    #[test]
    fn test_workspace_graph_construction() {
        let mut graph = WorkspaceGraph::new();

        // File A: Defines a section
        graph.add_document("file:///a.adoc", "[[sys-arch]]\n== System Architecture");

        // File B: References it
        graph.add_document("file:///b.adoc", "See <<sys-arch>> for details.");

        // Check Definition
        let def = graph.resolve_id("sys-arch");
        assert!(def.is_some(), "Should find definition for sys-arch");
        assert_eq!(def.unwrap().uri.as_str(), "file:///a.adoc");

        // Check Reference (Reverse lookup)
        let refs = graph.find_references("sys-arch");
        assert_eq!(refs.len(), 1, "Should find one reference");
        assert_eq!(refs[0].uri.as_str(), "file:///b.adoc");
    }

    /// Test 2: Multiple Definitions and References
    #[test]
    fn test_multiple_definitions_and_references() {
        let mut graph = WorkspaceGraph::new();

        // Multiple definitions
        graph.add_document(
            "file:///a.adoc",
            "[[intro]]\n== Introduction\n\n[[arch]]\n== Architecture",
        );

        // Multiple references in one file
        graph.add_document(
            "file:///b.adoc",
            "See <<intro>> and <<arch>> for more info.\nAlso <<intro>> again.",
        );

        // Check definitions
        assert!(graph.resolve_id("intro").is_some());
        assert!(graph.resolve_id("arch").is_some());

        // Check references - intro should have 2 refs
        let intro_refs = graph.find_references("intro");
        assert_eq!(intro_refs.len(), 2, "intro should have 2 references");

        // arch should have 1 ref
        let arch_refs = graph.find_references("arch");
        assert_eq!(arch_refs.len(), 1, "arch should have 1 reference");
    }

    /// Test 3: Document Update (Re-indexing)
    #[test]
    fn test_document_update() {
        let mut graph = WorkspaceGraph::new();

        // Initial content
        graph.add_document("file:///a.adoc", "[[old-id]]\n== Old Section");
        assert!(graph.resolve_id("old-id").is_some());

        // Update content (same URI, new content)
        graph.add_document("file:///a.adoc", "[[new-id]]\n== New Section");

        // Old ID should be gone
        assert!(
            graph.resolve_id("old-id").is_none(),
            "Old ID should be removed"
        );

        // New ID should exist
        assert!(graph.resolve_id("new-id").is_some(), "New ID should exist");
    }

    /// Test 4: Document Removal
    #[test]
    fn test_document_removal() {
        let mut graph = WorkspaceGraph::new();

        graph.add_document("file:///a.adoc", "[[to-remove]]\n== Section");
        assert!(graph.resolve_id("to-remove").is_some());

        graph.remove_document("file:///a.adoc");
        assert!(
            graph.resolve_id("to-remove").is_none(),
            "ID should be removed when document is removed"
        );
    }

    /// Test 5: Header-based ID Generation
    #[test]
    fn test_header_id_generation() {
        let mut graph = WorkspaceGraph::new();

        // Header without explicit anchor should generate an ID
        graph.add_document("file:///a.adoc", "= Document Title\n\n== Getting Started");

        // Should be able to find by generated ID
        let symbols = graph.query_symbols("Getting");
        assert!(!symbols.is_empty(), "Should find header by name");
    }
}

// ==================== WORKSPACE SYMBOL TESTS ====================

mod symbol_tests {
    use crate::workspace::graph::WorkspaceGraph;

    /// Test 6: Workspace Symbols (Ctrl+T)
    #[test]
    fn test_workspace_symbol_search() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "== System Architecture");

        let symbols = graph.query_symbols("Arch");

        assert!(!symbols.is_empty(), "Should find matching symbols");
        assert_eq!(symbols[0].name, "System Architecture");
        assert_eq!(symbols[0].location.uri.as_str(), "file:///a.adoc");
    }

    /// Test 7: Case-Insensitive Symbol Search
    #[test]
    fn test_symbol_search_case_insensitive() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "== API Reference\n\n== Database Schema");

        // Search with different cases
        let upper = graph.query_symbols("API");
        let lower = graph.query_symbols("api");
        let mixed = graph.query_symbols("Api");

        assert!(!upper.is_empty());
        assert!(!lower.is_empty());
        assert!(!mixed.is_empty());
    }

    /// Test 8: Symbol Search Across Multiple Files
    #[test]
    fn test_symbol_search_multi_file() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "== Architecture Overview");
        graph.add_document("file:///b.adoc", "== System Architecture");
        graph.add_document("file:///c.adoc", "== Database Design");

        let results = graph.query_symbols("Architecture");

        assert_eq!(results.len(), 2, "Should find 2 matches for 'Architecture'");
    }

    /// Test 9: Empty Query Returns All Symbols
    #[test]
    fn test_empty_query_returns_all() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "== Section One\n\n== Section Two");

        let all_symbols = graph.query_symbols("");

        assert!(
            all_symbols.len() >= 2,
            "Empty query should return all symbols"
        );
    }

    /// Test 10: Symbol Location Has Correct Line Numbers
    #[test]
    fn test_symbol_location_accuracy() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document(
            "file:///a.adoc",
            "= Title\n\n== First Section\n\n== Second Section",
        );

        let symbols = graph.query_symbols("Second");

        assert!(!symbols.is_empty());
        // "Second Section" is on line 4 (0-indexed)
        assert_eq!(symbols[0].location.range.start.line, 4);
    }
}

// ==================== BROKEN LINK DETECTION TESTS ====================

mod validation_tests {
    use crate::workspace::graph::WorkspaceGraph;

    /// Test 11: Broken Link Detection
    #[test]
    fn test_broken_link_detection() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///b.adoc", "See <<missing-id>>");

        let diagnostics = graph.validate_links("file:///b.adoc");

        assert!(!diagnostics.is_empty(), "Should detect broken link");
        assert!(
            diagnostics[0].message.contains("Broken reference")
                || diagnostics[0].message.contains("missing-id"),
            "Message should mention broken reference: {}",
            diagnostics[0].message
        );
    }

    /// Test 12: Valid Links Should Pass
    #[test]
    fn test_valid_links_pass() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "[[valid-id]]\n== Valid Section");
        graph.add_document("file:///b.adoc", "See <<valid-id>>");

        let diagnostics = graph.validate_links("file:///b.adoc");

        assert!(
            diagnostics.is_empty(),
            "Valid links should not produce diagnostics"
        );
    }

    /// Test 13: Multiple Broken Links in One File
    #[test]
    fn test_multiple_broken_links() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "See <<missing1>> and <<missing2>>");

        let diagnostics = graph.validate_links("file:///a.adoc");

        assert_eq!(diagnostics.len(), 2, "Should find 2 broken links");
    }

    /// Test 14: Cross-File Link Validation
    #[test]
    fn test_cross_file_link_validation() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "[[defined-here]]\n== Section");
        graph.add_document(
            "file:///b.adoc",
            "Links to <<defined-here>> and <<not-defined>>",
        );

        let diagnostics = graph.validate_links("file:///b.adoc");

        assert_eq!(
            diagnostics.len(),
            1,
            "Should only flag the undefined reference"
        );
        assert!(diagnostics[0].message.contains("not-defined"));
    }

    /// Test 15: Validate All Documents
    #[test]
    fn test_validate_all_documents() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "See <<missing-a>>");
        graph.add_document("file:///b.adoc", "See <<missing-b>>");

        let all_diagnostics = graph.validate_all_links();

        assert!(
            all_diagnostics.len() >= 2,
            "Should find broken links in multiple files"
        );
    }
}

// ==================== INDEXER TESTS ====================

mod indexer_tests {
    use crate::workspace::indexer::WorkspaceIndexer;

    /// Test 16: Indexer Extracts IDs from Content
    #[test]
    fn test_indexer_extracts_ids() {
        let content = "[[anchor-id]]\n== Header Title\n\n[[another-id]]\n=== Sub Section";

        let ids = WorkspaceIndexer::extract_definitions(content);

        assert!(ids.iter().any(|(id, _, _)| id == "anchor-id"));
        assert!(ids.iter().any(|(id, _, _)| id == "another-id"));
    }

    /// Test 17: Indexer Extracts References
    #[test]
    fn test_indexer_extracts_references() {
        let content = "See <<ref-one>> and <<ref-two>> for details.\nAlso <<ref-one>>.";

        let refs = WorkspaceIndexer::extract_references(content);

        // Should find 3 references total (ref-one twice, ref-two once)
        assert_eq!(refs.len(), 3);
        assert_eq!(refs.iter().filter(|(id, _, _)| id == "ref-one").count(), 2);
        assert_eq!(refs.iter().filter(|(id, _, _)| id == "ref-two").count(), 1);
    }

    /// Test 18: Indexer Extracts Headers as Symbols
    #[test]
    fn test_indexer_extracts_headers() {
        let content = "= Document Title\n\n== First Section\n\n=== Subsection\n\n== Second Section";

        let headers = WorkspaceIndexer::extract_headers(content);

        assert_eq!(headers.len(), 4);
        assert!(headers.iter().any(|(name, _, _)| name == "Document Title"));
        assert!(headers.iter().any(|(name, _, _)| name == "First Section"));
        assert!(headers.iter().any(|(name, _, _)| name == "Subsection"));
        assert!(headers.iter().any(|(name, _, _)| name == "Second Section"));
    }

    /// Test 19: Indexer Handles Empty Content
    #[test]
    fn test_indexer_handles_empty() {
        let ids = WorkspaceIndexer::extract_definitions("");
        let refs = WorkspaceIndexer::extract_references("");
        let headers = WorkspaceIndexer::extract_headers("");

        assert!(ids.is_empty());
        assert!(refs.is_empty());
        assert!(headers.is_empty());
    }
}

// ==================== PERFORMANCE TESTS ====================

mod performance_tests {
    use crate::workspace::graph::WorkspaceGraph;
    use std::time::Instant;

    /// Test 20: Large Workspace Performance
    #[test]
    fn test_large_workspace_performance() {
        let mut graph = WorkspaceGraph::new();

        // Simulate 100 files with multiple sections each
        for i in 0..100 {
            let mut content = String::new();
            for j in 0..10 {
                content.push_str(&format!(
                    "[[file{}-section{}]]\n== Section {} of File {}\n\n",
                    i, j, j, i
                ));
            }
            // Add some cross-references
            if i > 0 {
                content.push_str(&format!("See <<file{}-section0>> for previous.\n", i - 1));
            }
            graph.add_document(&format!("file:///file{}.adoc", i), &content);
        }

        // Test query performance
        let start = Instant::now();
        let _symbols = graph.query_symbols("Section");
        let query_time = start.elapsed();

        // Test validation performance
        let start = Instant::now();
        let _diagnostics = graph.validate_all_links();
        let validate_time = start.elapsed();

        assert!(
            query_time < std::time::Duration::from_millis(100),
            "Symbol query took {:?} (should be <100ms)",
            query_time
        );
        assert!(
            validate_time < std::time::Duration::from_millis(200),
            "Validation took {:?} (should be <200ms)",
            validate_time
        );
    }
}

// ==================== INTEGRATION TESTS ====================

mod integration_tests {
    use crate::workspace::graph::WorkspaceGraph;
    use crate::workspace::symbol_provider::SymbolProvider;

    /// Test 21: Symbol Provider Integration
    #[test]
    fn test_symbol_provider_integration() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "== Architecture\n\n== Design");
        graph.add_document("file:///b.adoc", "== Implementation");

        let provider = SymbolProvider::new(&graph);
        let symbols = provider.workspace_symbols("Arch");

        assert!(!symbols.is_empty());
        // Should return proper LSP WorkspaceSymbol format
    }

    /// Test 22: Go To Definition
    #[test]
    fn test_go_to_definition() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "[[target]]\n== Target Section");
        graph.add_document("file:///b.adoc", "Link to <<target>>");

        // User clicks on <<target>> in file B
        let location = graph.resolve_id("target");

        assert!(location.is_some());
        let loc = location.unwrap();
        assert_eq!(loc.uri.as_str(), "file:///a.adoc");
        assert_eq!(loc.range.start.line, 0);
    }

    /// Test 23: Find All References
    #[test]
    fn test_find_all_references() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///a.adoc", "[[api]]\n== API Reference");
        graph.add_document("file:///b.adoc", "See <<api>> for the API.");
        graph.add_document("file:///c.adoc", "The <<api>> documentation.");

        let refs = graph.find_references("api");

        assert_eq!(refs.len(), 2, "Should find 2 references across files");
    }
}
