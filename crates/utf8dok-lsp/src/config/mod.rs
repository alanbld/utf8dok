//! Configuration Engine (Phase 14)
//!
//! Provides enterprise-configurable settings for utf8dok-lsp.
//!
//! # Configuration File
//!
//! Settings are loaded from `utf8dok.toml` in the workspace root:
//!
//! ```toml
//! [compliance.bridge]
//! orphans = "warning"
//! superseded_status = "error"
//!
//! [plugins]
//! api_docs = true
//! writing_quality = false
//! custom_rules = ["rules/custom.rhai"]
//!
//! [workspace]
//! root = "docs/"
//! entry_points = ["index.adoc", "README.adoc"]
//! ```

mod settings;

#[cfg(test)]
mod tests;

pub use settings::{
    BridgeSettings, ComplianceSettings, PluginSettings, RuleSeverity, Settings, WorkspaceSettings,
};
