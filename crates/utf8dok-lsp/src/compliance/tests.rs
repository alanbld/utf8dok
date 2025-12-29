//! TDD Tests for Compliance Engine (Phase 12)
//!
//! Tests the compliance rules for Bridge Framework (ADRs).

use super::bridge::BridgeRules;
use crate::workspace::graph::WorkspaceGraph;

// ==================== SUPERSEDED STATUS TESTS ====================

mod status_tests {
    use super::*;

    /// Test 1: Cross-File Status Validation (The "Superseded" Rule)
    /// Scenario: ADR 002 claims to "Supersede" ADR 001.
    /// Rule: ADR 001 must have status "Deprecated" or "Superseded".
    /// Expectation: If ADR 001 is still "Accepted", return a Violation.
    #[test]
    fn test_adr_superseded_validation() {
        let mut graph = WorkspaceGraph::new();
        // ADR 001 is Accepted (Invalid state if superseded)
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n:status: Accepted",
        );
        // ADR 002 Supersedes 001
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:supersedes: adr-001",
        );

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        assert!(!violations.is_empty(), "Should detect status violation");
        assert!(
            violations
                .iter()
                .any(|v| v.message.contains("must be Deprecated") || v.message.contains("must be Superseded")),
            "Message should mention required status change"
        );
    }

    /// Test 2: Valid Superseded Chain
    /// Scenario: ADR 002 supersedes ADR 001, and ADR 001 is Deprecated.
    /// Expectation: No violations.
    #[test]
    fn test_valid_superseded_chain() {
        let mut graph = WorkspaceGraph::new();
        // ADR 001 is Deprecated (correct)
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n:status: Deprecated",
        );
        // ADR 002 Supersedes 001
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:supersedes: adr-001",
        );

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let status_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE001")
            .collect();
        assert!(
            status_violations.is_empty(),
            "Should not report status violation for deprecated ADR"
        );
    }

    /// Test 3: Superseded with "Superseded" status (also valid)
    #[test]
    fn test_superseded_status_also_valid() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n:status: Superseded",
        );
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:supersedes: adr-001",
        );

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let status_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE001")
            .collect();
        assert!(status_violations.is_empty());
    }

    /// Test 4: Multiple supersedes declarations
    #[test]
    fn test_multiple_supersedes() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n:status: Accepted",
        );
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:status: Accepted",
        );
        // ADR 003 supersedes both
        graph.add_document(
            "file:///adr-003.adoc",
            "[[adr-003]]\n= ADR 003\n:supersedes: adr-001, adr-002",
        );

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let status_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE001")
            .collect();
        assert_eq!(
            status_violations.len(),
            2,
            "Should report violations for both superseded ADRs"
        );
    }

    /// Test 5: Supersedes non-existent ADR (warning, not error)
    #[test]
    fn test_supersedes_missing_adr() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:supersedes: adr-001",
        );
        // adr-001 doesn't exist

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        // Should warn about missing reference
        assert!(
            violations.iter().any(|v| v.code == "BRIDGE002"),
            "Should warn about superseding non-existent ADR"
        );
    }
}

// ==================== ORPHAN DETECTION TESTS ====================

mod orphan_tests {
    use super::*;

    /// Test 6: Orphaned Decision Detection
    /// Scenario: An ADR exists but is not linked from the "Log" or "Table of Contents".
    /// Expectation: Warning: "ADR is orphaned (not reachable from index)."
    #[test]
    fn test_orphaned_adr_detection() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "[[index]]\n= ADR Log\n\nLink to <<adr-001>>");
        graph.add_document("file:///adr-001.adoc", "[[adr-001]]\n= ADR 001");
        graph.add_document("file:///adr-002.adoc", "[[adr-002]]\n= ADR 002"); // Orphan

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        assert!(
            violations.iter().any(|v| v.message.contains("orphaned") || v.message.contains("Orphan")),
            "Should detect orphaned document"
        );
        assert!(
            violations.iter().any(|v| v.uri.as_str().contains("adr-002")),
            "Should identify adr-002 as orphaned"
        );
    }

    /// Test 7: All ADRs linked (no orphans)
    #[test]
    fn test_no_orphans_when_all_linked() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document(
            "file:///index.adoc",
            "[[index]]\n= ADR Log\n\n<<adr-001>>\n<<adr-002>>",
        );
        graph.add_document("file:///adr-001.adoc", "[[adr-001]]\n= ADR 001");
        graph.add_document("file:///adr-002.adoc", "[[adr-002]]\n= ADR 002");

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let orphan_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE003")
            .collect();
        assert!(
            orphan_violations.is_empty(),
            "Should not report orphans when all linked"
        );
    }

    /// Test 8: Transitive linking (A -> B -> C, C is not orphan)
    #[test]
    fn test_transitive_linking() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "[[index]]\n= Index\n\n<<adr-001>>");
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n\nSee also <<adr-002>>",
        );
        graph.add_document("file:///adr-002.adoc", "[[adr-002]]\n= ADR 002");

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let orphan_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE003")
            .collect();
        assert!(
            orphan_violations.is_empty(),
            "Transitively linked documents should not be orphans"
        );
    }

    /// Test 9: Index itself is not an orphan
    #[test]
    fn test_index_not_orphan() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "[[index]]\n= ADR Log");

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let orphan_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE003")
            .collect();
        assert!(
            orphan_violations.is_empty(),
            "Index should not be reported as orphan"
        );
    }

    /// Test 10: Multiple entry points (index.adoc OR README.adoc)
    #[test]
    fn test_multiple_entry_points() {
        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///README.adoc", "[[readme]]\n= README\n\n<<adr-001>>");
        graph.add_document("file:///adr-001.adoc", "[[adr-001]]\n= ADR 001");

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        let orphan_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE003")
            .collect();
        assert!(
            orphan_violations.is_empty(),
            "README.adoc should also be a valid entry point"
        );
    }
}

// ==================== INTEGRATION TESTS ====================

mod integration_tests {
    use super::*;

    /// Test 11: Full compliance check on realistic ADR set
    #[test]
    fn test_full_adr_compliance_check() {
        let mut graph = WorkspaceGraph::new();

        // Index links to ADR 001 and 002
        graph.add_document(
            "file:///decisions/index.adoc",
            "[[adr-index]]\n= Architecture Decision Records\n\n<<adr-001>>\n<<adr-002>>",
        );

        // ADR 001: Deprecated (superseded by 002)
        graph.add_document(
            "file:///decisions/adr-001.adoc",
            "[[adr-001]]\n= Use PostgreSQL\n:status: Deprecated",
        );

        // ADR 002: Accepted, supersedes 001
        graph.add_document(
            "file:///decisions/adr-002.adoc",
            "[[adr-002]]\n= Use CockroachDB\n:status: Accepted\n:supersedes: adr-001",
        );

        // ADR 003: Orphaned (not linked from index)
        graph.add_document(
            "file:///decisions/adr-003.adoc",
            "[[adr-003]]\n= Use Redis\n:status: Proposed",
        );

        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);

        // Should only find the orphan (ADR 003)
        assert_eq!(violations.len(), 1, "Should find exactly one violation");
        assert!(violations[0].uri.as_str().contains("adr-003"));
        assert_eq!(violations[0].code, "BRIDGE003");
    }

    /// Test 12: Performance test with many documents
    #[test]
    fn test_compliance_performance() {
        let mut graph = WorkspaceGraph::new();

        // Create index linking to 50 ADRs
        let mut index_content = "[[adr-index]]\n= ADR Log\n\n".to_string();
        for i in 1..=50 {
            index_content.push_str(&format!("<<adr-{:03}>>\n", i));
        }
        graph.add_document("file:///index.adoc", &index_content);

        // Create 50 ADRs
        for i in 1..=50 {
            let content = format!(
                "[[adr-{:03}]]\n= ADR {:03}\n:status: Accepted",
                i, i
            );
            graph.add_document(&format!("file:///adr-{:03}.adoc", i), &content);
        }

        // Add some supersedes relationships
        for i in 1..=10 {
            let content = format!(
                "[[adr-{:03}]]\n= ADR {:03}\n:status: Accepted\n:supersedes: adr-{:03}",
                50 + i,
                50 + i,
                i
            );
            graph.add_document(&format!("file:///adr-{:03}.adoc", 50 + i), &content);
        }

        let start = std::time::Instant::now();
        let rules = BridgeRules::default();
        let violations = rules.validate(&graph);
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Compliance check took {:?} (should be <100ms)",
            elapsed
        );

        // Should find status violations (ADRs 1-10 should be deprecated)
        let status_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "BRIDGE001")
            .collect();
        assert_eq!(status_violations.len(), 10);
    }
}

// ==================== ATTRIBUTE EXTRACTION TESTS ====================

mod attribute_tests {
    use crate::workspace::indexer::WorkspaceIndexer;

    /// Test 13: Extract status attribute
    #[test]
    fn test_extract_status_attribute() {
        let content = "= Title\n:status: Accepted\n:author: John";
        let attrs = WorkspaceIndexer::extract_attributes(content);

        assert!(attrs.contains_key("status"));
        assert_eq!(attrs.get("status").unwrap(), "Accepted");
    }

    /// Test 14: Extract supersedes attribute (single value)
    #[test]
    fn test_extract_supersedes_single() {
        let content = "= Title\n:supersedes: adr-001";
        let attrs = WorkspaceIndexer::extract_attributes(content);

        assert!(attrs.contains_key("supersedes"));
        assert_eq!(attrs.get("supersedes").unwrap(), "adr-001");
    }

    /// Test 15: Extract supersedes attribute (multiple values)
    #[test]
    fn test_extract_supersedes_multiple() {
        let content = "= Title\n:supersedes: adr-001, adr-002";
        let attrs = WorkspaceIndexer::extract_attributes(content);

        let supersedes = attrs.get("supersedes").unwrap();
        assert!(supersedes.contains("adr-001"));
        assert!(supersedes.contains("adr-002"));
    }

    /// Test 16: No attributes in document
    #[test]
    fn test_no_attributes() {
        let content = "= Title\n\nJust a paragraph.";
        let attrs = WorkspaceIndexer::extract_attributes(content);

        assert!(attrs.is_empty() || !attrs.contains_key("status"));
    }
}
