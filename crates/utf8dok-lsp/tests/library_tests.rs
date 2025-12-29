//! Library integration tests for utf8dok-lsp
//!
//! These tests verify that the LSP crate works correctly as a library,
//! proving that the binary/library refactor was successful.

use utf8dok_lsp::compliance::ComplianceEngine;
use utf8dok_lsp::workspace::graph::WorkspaceGraph;

#[test]
fn test_library_graph_access() {
    // This proves we can use the LSP as a library
    let graph = WorkspaceGraph::new();
    assert_eq!(graph.document_count(), 0);
}

#[test]
fn test_library_compliance_engine() {
    // Test that the compliance engine can be used as a library
    let engine = ComplianceEngine::new();
    let descriptions = engine.rule_descriptions();

    // Should have at least StatusRule and OrphanRule
    assert!(descriptions.len() >= 2);
    assert!(descriptions.iter().any(|(code, _)| *code == "BRIDGE001"));
    assert!(descriptions.iter().any(|(code, _)| *code == "BRIDGE003"));
}

#[test]
fn test_library_compliance_result() {
    // Test that ComplianceResult can be used
    let engine = ComplianceEngine::new();
    let graph = WorkspaceGraph::new();

    let result = engine.run_with_stats(&graph);

    // Empty graph should have no violations
    assert!(result.is_clean());
    assert!(!result.has_critical());
    assert_eq!(result.compliance_score, 100);
}

#[test]
fn test_workspace_graph_document_operations() {
    let mut graph = WorkspaceGraph::new();

    // Add documents
    graph.add_document("file:///test1.adoc", "= Title\n\nContent here.");
    graph.add_document("file:///test2.adoc", "= Another\n\n<<test1>>");

    assert_eq!(graph.document_count(), 2);

    // Remove a document
    graph.remove_document("file:///test1.adoc");
    assert_eq!(graph.document_count(), 1);
}

#[test]
fn test_compliance_with_documents() {
    let engine = ComplianceEngine::new();
    let mut graph = WorkspaceGraph::new();

    // Add a well-formed ADR structure
    graph.add_document(
        "file:///index.adoc",
        "[[index]]\n= ADR Index\n\n<<adr-001>>",
    );
    graph.add_document(
        "file:///adr-001.adoc",
        "[[adr-001]]\n= ADR 001\n:status: Accepted",
    );

    let result = engine.run_with_stats(&graph);

    // Should have no critical errors for well-formed structure
    assert!(!result.has_critical());
    assert!(result.compliance_score >= 80);
}
