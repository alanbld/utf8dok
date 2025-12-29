//! Configuration Settings (Phase 14)
//!
//! Defines the configuration structures for enterprise customization.

use serde::{Deserialize, Serialize};

/// Rule severity levels for compliance rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    /// Rule violations are errors (blocks build)
    Error,
    /// Rule violations are warnings (default for most)
    #[default]
    Warning,
    /// Rule violations are informational
    Info,
    /// Rule is disabled
    Ignore,
}

impl RuleSeverity {
    /// Convert to ViolationSeverity, returns None if Ignore
    pub fn to_violation_severity(self) -> Option<crate::compliance::ViolationSeverity> {
        match self {
            RuleSeverity::Error => Some(crate::compliance::ViolationSeverity::Error),
            RuleSeverity::Warning => Some(crate::compliance::ViolationSeverity::Warning),
            RuleSeverity::Info => Some(crate::compliance::ViolationSeverity::Info),
            RuleSeverity::Ignore => None,
        }
    }

    /// Check if this severity means the rule is enabled
    pub fn is_enabled(self) -> bool {
        self != RuleSeverity::Ignore
    }
}

/// Top-level settings structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    /// Compliance rule settings
    pub compliance: ComplianceSettings,
    /// Plugin settings
    pub plugins: PluginSettings,
    /// Workspace settings
    pub workspace: WorkspaceSettings,
}

impl Settings {
    /// Parse settings from a TOML string
    pub fn from_toml_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }
}

/// Compliance rule configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ComplianceSettings {
    /// BRIDGE rule settings
    pub bridge: BridgeSettings,
}

/// BRIDGE-specific compliance rule settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BridgeSettings {
    /// Severity for orphan document detection (BRIDGE003)
    pub orphans: RuleSeverity,
    /// Severity for superseded status violations (BRIDGE001)
    pub superseded_status: RuleSeverity,
}

impl Default for BridgeSettings {
    fn default() -> Self {
        Self {
            orphans: RuleSeverity::Warning,
            superseded_status: RuleSeverity::Error,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginSettings {
    /// Enable API documentation analysis
    pub api_docs: bool,
    /// Enable writing quality checks
    pub writing_quality: bool,
    /// Enable diagram syntax highlighting and validation
    pub diagrams: bool,
    /// Custom Rhai rule files
    pub custom_rules: Vec<String>,
    /// Custom weasel words for quality plugin
    pub custom_weasel_words: Vec<String>,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            api_docs: false,
            writing_quality: true,
            diagrams: true,
            custom_rules: Vec::new(),
            custom_weasel_words: default_weasel_words(),
        }
    }
}

/// Default list of weasel words to detect
fn default_weasel_words() -> Vec<String> {
    vec![
        "clearly".to_string(),
        "obviously".to_string(),
        "basically".to_string(),
        "simply".to_string(),
        "just".to_string(),
        "actually".to_string(),
        "really".to_string(),
        "very".to_string(),
        "quite".to_string(),
        "perhaps".to_string(),
        "maybe".to_string(),
        "possibly".to_string(),
    ]
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceSettings {
    /// Root directory for documentation
    pub root: Option<String>,
    /// Entry point documents (index files)
    pub entry_points: Vec<String>,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            root: None,
            entry_points: vec!["index.adoc".to_string(), "README.adoc".to_string()],
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_rule_severity_default() {
        assert_eq!(RuleSeverity::default(), RuleSeverity::Warning);
    }

    #[test]
    fn test_settings_debug() {
        let settings = Settings::default();
        let debug_str = format!("{:?}", settings);
        assert!(debug_str.contains("Settings"));
    }
}
