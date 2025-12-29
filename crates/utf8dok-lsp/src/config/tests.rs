//! TDD Tests for Configuration Engine (Phase 14)
//!
//! Tests the configuration system for enterprise customization.

use super::*;
use crate::workspace::graph::WorkspaceGraph;

// ==================== SETTINGS PARSING TESTS ====================

mod parsing_tests {
    use super::*;

    /// Test 1: Load configuration from TOML
    #[test]
    fn test_load_config_from_toml() {
        let toml = r#"
[compliance.bridge]
orphans = "error"
superseded_status = "warning"
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Error);
        assert_eq!(
            settings.compliance.bridge.superseded_status,
            RuleSeverity::Warning
        );
    }

    /// Test 2: Default fallback when no config
    #[test]
    fn test_default_settings() {
        let settings = Settings::default();

        // Defaults should be warnings (not errors, not ignored)
        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Warning);
        assert_eq!(
            settings.compliance.bridge.superseded_status,
            RuleSeverity::Error
        );
    }

    /// Test 3: Parse severity enum correctly
    #[test]
    fn test_severity_parsing() {
        let toml = r#"
[compliance.bridge]
orphans = "ignore"
superseded_status = "info"
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Ignore);
        assert_eq!(
            settings.compliance.bridge.superseded_status,
            RuleSeverity::Info
        );
    }

    /// Test 4: Partial config uses defaults for missing fields
    #[test]
    fn test_partial_config_uses_defaults() {
        let toml = r#"
[compliance.bridge]
orphans = "error"
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Error);
        // Missing field uses default
        assert_eq!(
            settings.compliance.bridge.superseded_status,
            RuleSeverity::Error
        );
    }

    /// Test 5: Empty config uses all defaults
    #[test]
    fn test_empty_config_uses_defaults() {
        let toml = "";

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Warning);
    }

    /// Test 6: Plugin configuration parsing
    #[test]
    fn test_plugin_config_parsing() {
        let toml = r#"
[plugins]
api_docs = true
writing_quality = false
custom_rules = ["rules/custom.rhai"]
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert!(settings.plugins.api_docs);
        assert!(!settings.plugins.writing_quality);
        assert_eq!(settings.plugins.custom_rules.len(), 1);
        assert_eq!(settings.plugins.custom_rules[0], "rules/custom.rhai");
    }

    /// Test 7: Workspace configuration parsing
    #[test]
    fn test_workspace_config_parsing() {
        let toml = r#"
[workspace]
root = "docs/"
entry_points = ["index.adoc", "README.adoc"]
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.workspace.root, Some("docs/".to_string()));
        assert_eq!(settings.workspace.entry_points.len(), 2);
    }
}

// ==================== LOADER TESTS ====================

mod loader_tests {
    use super::*;

    /// Test 8: Load from file path
    #[test]
    fn test_load_from_string() {
        let toml = r#"
[compliance.bridge]
orphans = "error"
"#;

        let settings = Settings::from_toml_str(toml).unwrap();
        assert_eq!(settings.compliance.bridge.orphans, RuleSeverity::Error);
    }

    /// Test 9: Invalid TOML returns error
    #[test]
    fn test_invalid_toml_returns_error() {
        let invalid_toml = "this is not valid { toml";

        let result = Settings::from_toml_str(invalid_toml);
        assert!(result.is_err());
    }

    /// Test 10: Unknown fields are ignored (forward compatibility)
    #[test]
    fn test_unknown_fields_ignored() {
        let toml = r#"
[compliance.bridge]
orphans = "error"
future_field = "value"

[future_section]
something = true
"#;

        // Should not panic, unknown fields ignored
        let result = Settings::from_toml_str(toml);
        assert!(result.is_ok());
    }
}

// ==================== COMPLIANCE INTEGRATION TESTS ====================

mod compliance_integration_tests {
    use super::*;
    use crate::compliance::bridge::OrphanRule;
    use crate::compliance::ComplianceRule;

    /// Test 11: Orphan rule respects "ignore" setting
    #[test]
    fn test_orphan_rule_respects_ignore() {
        let mut settings = Settings::default();
        settings.compliance.bridge.orphans = RuleSeverity::Ignore;

        let mut graph = WorkspaceGraph::new();
        // Add an orphan document (not linked from index)
        graph.add_document("file:///index.adoc", "[[index]]\n= Index");
        graph.add_document("file:///orphan.adoc", "[[orphan]]\n= Orphan Doc");

        let rule = OrphanRule::with_settings(&settings);
        let violations = rule.check(&graph);

        // Should be empty because orphans are ignored
        assert!(violations.is_empty(), "Orphans should be ignored");
    }

    /// Test 12: Orphan rule respects "error" setting
    #[test]
    fn test_orphan_rule_respects_error() {
        let mut settings = Settings::default();
        settings.compliance.bridge.orphans = RuleSeverity::Error;

        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "[[index]]\n= Index");
        graph.add_document("file:///orphan.adoc", "[[orphan]]\n= Orphan Doc");

        let rule = OrphanRule::with_settings(&settings);
        let violations = rule.check(&graph);

        // Should have a violation
        assert!(!violations.is_empty(), "Should detect orphan");
        assert_eq!(
            violations[0].severity,
            crate::compliance::ViolationSeverity::Error
        );
    }

    /// Test 13: Orphan rule respects "warning" setting
    #[test]
    fn test_orphan_rule_respects_warning() {
        let mut settings = Settings::default();
        settings.compliance.bridge.orphans = RuleSeverity::Warning;

        let mut graph = WorkspaceGraph::new();
        graph.add_document("file:///index.adoc", "[[index]]\n= Index");
        graph.add_document("file:///orphan.adoc", "[[orphan]]\n= Orphan Doc");

        let rule = OrphanRule::with_settings(&settings);
        let violations = rule.check(&graph);

        assert!(!violations.is_empty());
        assert_eq!(
            violations[0].severity,
            crate::compliance::ViolationSeverity::Warning
        );
    }

    /// Test 14: Status rule respects settings
    #[test]
    fn test_status_rule_respects_settings() {
        use crate::compliance::bridge::StatusRule;

        let mut settings = Settings::default();
        settings.compliance.bridge.superseded_status = RuleSeverity::Ignore;

        let mut graph = WorkspaceGraph::new();
        // ADR 001 is Accepted but superseded
        graph.add_document(
            "file:///adr-001.adoc",
            "[[adr-001]]\n= ADR 001\n:status: Accepted",
        );
        graph.add_document(
            "file:///adr-002.adoc",
            "[[adr-002]]\n= ADR 002\n:supersedes: adr-001",
        );

        let rule = StatusRule::with_settings(&settings);
        let violations = rule.check(&graph);

        // Should be empty because status violations are ignored
        assert!(violations.is_empty());
    }

    /// Test 15: ComplianceEngine respects settings
    #[test]
    fn test_engine_respects_settings() {
        use crate::compliance::ComplianceEngine;

        let mut settings = Settings::default();
        settings.compliance.bridge.orphans = RuleSeverity::Ignore;
        settings.compliance.bridge.superseded_status = RuleSeverity::Ignore;

        let mut graph = WorkspaceGraph::new();
        // Add violations that would normally trigger
        graph.add_document("file:///orphan.adoc", "[[orphan]]\n= Orphan");

        let engine = ComplianceEngine::with_settings(&settings);
        let result = engine.run_with_stats(&graph);

        // All rules ignored, should be clean
        assert!(result.is_clean());
    }
}

// ==================== ENTRY POINTS TESTS ====================

mod entry_points_tests {
    use super::*;

    /// Test 16: Custom entry points from config
    #[test]
    fn test_custom_entry_points() {
        let toml = r#"
[workspace]
entry_points = ["docs/main.adoc", "docs/api.adoc"]
"#;

        let settings: Settings = toml::from_str(toml).unwrap();

        assert_eq!(settings.workspace.entry_points.len(), 2);
        assert!(settings.workspace.entry_points.contains(&"docs/main.adoc".to_string()));
    }

    /// Test 17: Default entry points
    #[test]
    fn test_default_entry_points() {
        let settings = Settings::default();

        // Default entry points should include common names
        assert!(settings.workspace.entry_points.contains(&"index.adoc".to_string()));
        assert!(settings.workspace.entry_points.contains(&"README.adoc".to_string()));
    }
}
