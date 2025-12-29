//! TDD Tests for the Universal Platform Architecture (Phase 10)
//!
//! Tests the plugin-based domain registry and semantic highlighting system.

use tower_lsp::lsp_types::{Position, SemanticTokenType};

fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

// ==================== REGISTRY TESTS ====================

mod registry_tests {
    use crate::domain::registry::DomainRegistry;

    /// Test 1: Core Registry Functionality
    #[test]
    fn test_registry_detects_and_selects_domain() {
        // GIVEN: A registry with multiple domains
        let registry = DomainRegistry::default();

        // WHEN: Detecting domain for different document types
        let bridge_doc = "= ADR 001: Title\n:status: Draft\n\n== Context\nSome context.\n\n== Decision\nWe decided.";
        let rfc_doc = "= RFC 1234: Some Protocol\n:category: standards-track\n\n== Abstract\nThis document describes...";

        // THEN: Correct domain is identified with confidence score
        let (bridge_domain, bridge_score) = registry
            .detect_domain(bridge_doc)
            .expect("Should detect Bridge");
        let (rfc_domain, rfc_score) = registry.detect_domain(rfc_doc).expect("Should detect RFC");

        assert_eq!(bridge_domain.name(), "bridge");
        assert_eq!(rfc_domain.name(), "rfc");
        assert!(
            bridge_score > 0.5 && bridge_score <= 1.0,
            "Bridge score {} should be > 0.5",
            bridge_score
        );
        assert!(
            rfc_score > 0.5 && rfc_score <= 1.0,
            "RFC score {} should be > 0.5",
            rfc_score
        );
    }

    /// Test 2: Registry Fallback to "Generic" Domain
    #[test]
    fn test_registry_fallback_to_generic() {
        // GIVEN: A plain document that doesn't match any specific domain
        let registry = DomainRegistry::default();
        let plain_doc = "Just a regular AsciiDoc paragraph.\n\nNo special headers or attributes.";

        // WHEN: Detecting domain
        let result = registry.detect_domain(plain_doc);

        // THEN: Should return a generic/fallback domain
        assert!(result.is_some());
        let (domain, score) = result.unwrap();
        assert_eq!(domain.name(), "generic");
        assert!(
            score > 0.0 && score < 0.5,
            "Generic score {} should be < 0.5",
            score
        );
    }

    /// Test 3: Registry can retrieve domain by name
    #[test]
    fn test_registry_get_domain_by_name() {
        let registry = DomainRegistry::default();

        let bridge = registry.get_domain("bridge");
        let rfc = registry.get_domain("rfc");
        let generic = registry.get_domain("generic");
        let unknown = registry.get_domain("unknown");

        assert!(bridge.is_some(), "Should find bridge domain");
        assert!(rfc.is_some(), "Should find rfc domain");
        assert!(generic.is_some(), "Should find generic domain");
        assert!(unknown.is_none(), "Should not find unknown domain");
    }

    /// Test 4: Registry handles empty documents
    #[test]
    fn test_registry_handles_empty_document() {
        let registry = DomainRegistry::default();
        let empty_doc = "";

        let result = registry.detect_domain(empty_doc);
        assert!(result.is_some());
        let (domain, _) = result.unwrap();
        assert_eq!(domain.name(), "generic");
    }
}

// ==================== PLUGIN TESTS ====================

mod plugin_tests {
    use super::*;
    use crate::domain::plugins::{BridgePlugin, GenericPlugin, RfcPlugin};
    use crate::domain::traits::DocumentDomain;

    /// Test 5: Bridge Plugin Correctly Implements Domain Trait
    #[test]
    fn test_bridge_plugin_implements_trait() {
        let plugin = BridgePlugin::new();

        // Test core trait methods
        assert_eq!(plugin.name(), "bridge");

        // Should score ADR documents highly
        let adr_doc = "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.\n\n== Decision\nTest.";
        assert!(
            plugin.score_document(adr_doc) > 0.8,
            "ADR doc should score > 0.8"
        );

        // Should score non-ADR documents low
        let plain_doc = "= Regular Document\n\nSome content.";
        assert!(
            plugin.score_document(plain_doc) < 0.3,
            "Plain doc should score < 0.3"
        );
    }

    /// Test 6: Bridge Plugin Validates ADR Documents
    #[test]
    fn test_bridge_plugin_validation() {
        let plugin = BridgePlugin::new();

        // Invalid status should produce diagnostic
        let invalid_doc =
            "= ADR 001: Test\n:status: Invalid\n\n== Context\nTest.\n\n== Decision\nTest.";
        let diagnostics = plugin.validate(invalid_doc);
        assert!(!diagnostics.is_empty(), "Should flag invalid status");

        // Valid document should have no diagnostics
        let valid_doc = "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.\n\n== Decision\nTest.\n\n== Consequences\nTest.";
        let diagnostics = plugin.validate(valid_doc);
        assert!(
            diagnostics.is_empty(),
            "Valid ADR should have no diagnostics"
        );
    }

    /// Test 7: Bridge Plugin Provides Completions
    #[test]
    fn test_bridge_plugin_completions() {
        let plugin = BridgePlugin::new();

        // Status value completion
        let completions = plugin.complete(pos(1, 9), ":status: ");
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"Draft"), "Should suggest Draft");
        assert!(labels.contains(&"Accepted"), "Should suggest Accepted");
    }

    /// Test 8: Bridge Plugin Classifies Semantic Elements
    #[test]
    fn test_bridge_plugin_semantic_classification() {
        let plugin = BridgePlugin::new();

        // Status attribute should be an ENUM
        assert_eq!(
            plugin.classify_element("attribute_name", "status"),
            Some(SemanticTokenType::ENUM)
        );

        // Author attribute should be a PROPERTY
        assert_eq!(
            plugin.classify_element("attribute_name", "author"),
            Some(SemanticTokenType::PROPERTY)
        );

        // Status values should be ENUM_MEMBER
        assert_eq!(
            plugin.classify_element("attribute_value", "Draft"),
            Some(SemanticTokenType::ENUM_MEMBER)
        );

        // Headers should be CLASS
        assert_eq!(
            plugin.classify_element("header", "Context"),
            Some(SemanticTokenType::CLASS)
        );
    }

    /// Test 9: RFC Plugin Provides Different Semantics
    #[test]
    fn test_rfc_plugin_differentiation() {
        let rfc_plugin = RfcPlugin::new();
        let bridge_plugin = BridgePlugin::new();

        // RFC should score RFC documents highly
        let rfc_doc = "= RFC 1234: Protocol\n:category: standards-track\n\n== Abstract\nTest.";
        assert!(
            rfc_plugin.score_document(rfc_doc) > 0.8,
            "RFC doc should score > 0.8"
        );

        // Same attribute name, different semantic meaning per domain
        // In Bridge, "category" is a generic property
        assert_eq!(
            bridge_plugin.classify_element("attribute_name", "category"),
            Some(SemanticTokenType::PROPERTY)
        );
        // In RFC, "category" is a keyword (standards-track, informational, etc.)
        assert_eq!(
            rfc_plugin.classify_element("attribute_name", "category"),
            Some(SemanticTokenType::KEYWORD)
        );
    }

    /// Test 10: Generic Plugin Handles All Documents
    #[test]
    fn test_generic_plugin_fallback() {
        let plugin = GenericPlugin::new();

        assert_eq!(plugin.name(), "generic");

        // Should always provide a low but non-zero score
        let score = plugin.score_document("Any document content");
        assert!(
            score > 0.0 && score < 0.3,
            "Generic score {} should be low",
            score
        );

        // Should provide basic classifications
        assert_eq!(
            plugin.classify_element("header", "Any Header"),
            Some(SemanticTokenType::CLASS)
        );
        assert_eq!(
            plugin.classify_element("attribute_name", "any-attr"),
            Some(SemanticTokenType::VARIABLE)
        );
    }
}

// ==================== SEMANTIC INTEGRATION TESTS ====================

mod semantic_tests {
    use super::*;
    use crate::domain::registry::DomainRegistry;
    use crate::domain::semantic::SemanticAnalyzer;

    /// Test 11: Semantic Analyzer Produces Tokens
    #[test]
    fn test_semantic_analyzer_produces_tokens() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = "= ADR 001: Test\n:status: Draft\n\n== Context\nSome context.";
        let tokens = analyzer.analyze(text);

        assert!(!tokens.is_empty(), "Should produce tokens");

        // Should find status attribute
        let status_token = tokens.iter().find(|t| t.text == "status");
        assert!(status_token.is_some(), "Should find 'status' token");

        // Should find Draft value
        let draft_token = tokens.iter().find(|t| t.text == "Draft");
        assert!(draft_token.is_some(), "Should find 'Draft' token");
    }

    /// Test 12: Semantic Tokens are Domain-Aware
    #[test]
    fn test_semantic_tokens_vary_by_domain() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        // Bridge document
        let bridge_text =
            "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.\n\n== Decision\nTest.";
        let bridge_tokens = analyzer.analyze(bridge_text);

        // The word "Draft" should be an ENUM_MEMBER in Bridge
        let bridge_draft_token = bridge_tokens
            .iter()
            .find(|t| t.text == "Draft")
            .expect("Should find 'Draft' token");
        assert_eq!(
            bridge_draft_token.token_type,
            SemanticTokenType::ENUM_MEMBER
        );

        // RFC document
        let rfc_text = "= RFC 1234: Test\n:category: standards-track\n\n== Abstract\nTest.";
        let rfc_tokens = analyzer.analyze(rfc_text);

        // The phrase "standards-track" should be a KEYWORD in RFC
        let rfc_std_token = rfc_tokens
            .iter()
            .find(|t| t.text == "standards-track")
            .expect("Should find 'standards-track' token");
        assert_eq!(rfc_std_token.token_type, SemanticTokenType::KEYWORD);
    }

    /// Test 13: Semantic Analyzer Provides Token Positions
    #[test]
    fn test_semantic_token_positions() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = ":status: Draft";
        let tokens = analyzer.analyze(text);

        // Find the status token
        let status_token = tokens
            .iter()
            .find(|t| t.text == "status")
            .expect("Should find status");
        assert_eq!(status_token.line, 0);
        assert_eq!(status_token.start_char, 1); // After the ':'
        assert_eq!(status_token.length, 6); // "status" is 6 chars

        // Find the Draft token
        let draft_token = tokens
            .iter()
            .find(|t| t.text == "Draft")
            .expect("Should find Draft");
        assert_eq!(draft_token.line, 0);
        assert_eq!(draft_token.start_char, 9); // After ":status: "
        assert_eq!(draft_token.length, 5); // "Draft" is 5 chars
    }

    /// Test 14: Performance of Semantic Analysis
    #[test]
    fn test_semantic_analysis_performance() {
        use std::time::Instant;

        // Build a large, mixed document
        let mut text = String::new();
        for i in 0..50 {
            text.push_str(&format!("= ADR {:03}: Decision {}\n", i, i));
            text.push_str(":status: Draft\n");
            text.push_str(":author: Team\n\n");
            text.push_str("== Context\nSome context here.\n\n");
            text.push_str("== Decision\nWe decided this.\n\n");
        }

        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let start = Instant::now();
        let tokens = analyzer.analyze(&text);
        let duration = start.elapsed();

        assert!(!tokens.is_empty());
        assert!(
            duration < std::time::Duration::from_millis(150),
            "Semantic analysis took {:?} (should be <150ms)",
            duration
        );
    }

    /// Test 15: Semantic Analyzer Handles Headers
    #[test]
    fn test_semantic_analyzer_headers() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = "= Document Title\n\n== Section One\n\n=== Subsection";
        let tokens = analyzer.analyze(text);

        // Headers should be classified
        let title_token = tokens.iter().find(|t| t.text == "Document Title");
        assert!(title_token.is_some(), "Should find title token");
        assert_eq!(title_token.unwrap().token_type, SemanticTokenType::CLASS);

        let section_token = tokens.iter().find(|t| t.text == "Section One");
        assert!(section_token.is_some(), "Should find section token");
    }

    /// Test 16: Convert to LSP Semantic Tokens Format
    #[test]
    fn test_convert_to_lsp_format() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = ":status: Draft\n:author: Team";
        let tokens = analyzer.analyze(text);

        // Convert to LSP delta format
        let lsp_tokens = analyzer.to_lsp_tokens(&tokens);

        // LSP tokens use delta encoding
        assert!(!lsp_tokens.is_empty());

        // First token should have absolute position
        let first = &lsp_tokens[0];
        assert_eq!(first.delta_line, 0);
    }
}

// ==================== INTEGRATION TESTS ====================

mod integration_tests {
    use super::*;
    use crate::domain::DomainEngine;

    /// Test 17: DomainEngine Coordinates All Features
    #[test]
    fn test_domain_engine_integration() {
        let engine = DomainEngine::new();

        let text = "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.\n\n== Decision\nTest.";

        // Completions should work
        let completions = engine.get_completions(text, pos(1, 9));
        assert!(!completions.is_empty(), "Should provide completions");

        // Semantic tokens should work
        let tokens = engine.get_semantic_tokens(text);
        assert!(!tokens.is_empty(), "Should provide semantic tokens");

        // Code actions should work (from Phase 9)
        // (Code actions are triggered by params, not direct text)
    }

    /// Test 18: DomainEngine Uses Registry for Domain Detection
    #[test]
    fn test_domain_engine_uses_registry() {
        let engine = DomainEngine::new();

        // Bridge document
        let bridge_doc =
            "= ADR 001: Test\n:status: Draft\n\n== Context\nTest.\n\n== Decision\nTest.";
        let bridge_domain = engine.detect_domain(bridge_doc);
        assert_eq!(bridge_domain, "bridge");

        // RFC document
        let rfc_doc = "= RFC 1234: Test\n:category: standards-track\n\n== Abstract\nTest.";
        let rfc_domain = engine.detect_domain(rfc_doc);
        assert_eq!(rfc_domain, "rfc");

        // Generic document
        let plain_doc = "Just some text.";
        let plain_domain = engine.detect_domain(plain_doc);
        assert_eq!(plain_domain, "generic");
    }
}
