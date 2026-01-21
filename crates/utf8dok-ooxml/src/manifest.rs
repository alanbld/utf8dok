//! Manifest handling for utf8dok container architecture
//!
//! The manifest tracks metadata about embedded elements within a DOCX file,
//! enabling round-trip fidelity and drift detection.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utf8dok_ast::DocumentIntent;

use crate::error::Result;

/// The manifest file path within the DOCX archive
pub const MANIFEST_PATH: &str = "utf8dok/manifest.json";

/// Manifest for tracking utf8dok-managed content within a DOCX
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Manifest format version (e.g., "1.0")
    pub version: String,
    /// Generator identifier (e.g., "utf8dok v0.1.0")
    pub generator: String,
    /// ISO 8601 timestamp when the manifest was generated
    pub generated_at: String,
    /// Map of element IDs to their metadata
    #[serde(default)]
    pub elements: HashMap<String, ElementMeta>,
    /// Compiler intent for document compilation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<DocumentIntent>,
}

/// Metadata for a tracked element within the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMeta {
    /// Element type: "figure", "table", "section", "code", etc.
    #[serde(rename = "type")]
    pub type_: String,
    /// Source file path within utf8dok/ folder (e.g., "utf8dok/diagrams/fig1.mmd")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Content hash for drift detection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Optional description or caption
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Manifest {
    /// Create a new manifest with default values
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            generator: format!("utf8dok v{}", env!("CARGO_PKG_VERSION")),
            generated_at: Self::current_timestamp(),
            elements: HashMap::new(),
            compiler: None,
        }
    }

    /// Get the current ISO 8601 timestamp
    fn current_timestamp() -> String {
        // Simple implementation without external time crate
        // In production, consider using chrono or time crate
        "2025-01-01T00:00:00Z".to_string() // Placeholder
    }

    /// Add an element to the manifest
    pub fn add_element(&mut self, id: impl Into<String>, meta: ElementMeta) {
        self.elements.insert(id.into(), meta);
    }

    /// Get an element by ID
    pub fn get_element(&self, id: &str) -> Option<&ElementMeta> {
        self.elements.get(id)
    }

    /// Remove an element by ID
    pub fn remove_element(&mut self, id: &str) -> Option<ElementMeta> {
        self.elements.remove(id)
    }

    /// Check if the manifest has any elements
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get the number of tracked elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Serialize the manifest to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Serialize the manifest to JSON bytes
    pub fn to_json_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    /// Deserialize a manifest from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Deserialize a manifest from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl ElementMeta {
    /// Create a new element metadata entry
    pub fn new(type_: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            source: None,
            hash: None,
            description: None,
        }
    }

    /// Set the source path
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the content hash
    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_new() {
        let manifest = Manifest::new();
        assert_eq!(manifest.version, "1.0");
        assert!(manifest.generator.starts_with("utf8dok v"));
        assert!(manifest.elements.is_empty());
    }

    #[test]
    fn test_manifest_add_element() {
        let mut manifest = Manifest::new();

        manifest.add_element(
            "fig1",
            ElementMeta::new("figure")
                .with_source("utf8dok/diagrams/fig1.mmd")
                .with_hash("abc123"),
        );

        assert_eq!(manifest.len(), 1);

        let elem = manifest.get_element("fig1").unwrap();
        assert_eq!(elem.type_, "figure");
        assert_eq!(elem.source, Some("utf8dok/diagrams/fig1.mmd".to_string()));
        assert_eq!(elem.hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_manifest_serialize_deserialize() {
        let mut manifest = Manifest::new();
        manifest.add_element(
            "table1",
            ElementMeta::new("table").with_description("Sales data"),
        );
        manifest.add_element(
            "fig1",
            ElementMeta::new("figure").with_source("utf8dok/diagrams/architecture.mmd"),
        );

        // Serialize to JSON
        let json = manifest.to_json().unwrap();
        println!("Serialized manifest:\n{}", json);

        // Verify JSON structure
        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("\"type\": \"table\""));
        assert!(json.contains("\"type\": \"figure\""));
        assert!(json.contains("\"description\": \"Sales data\""));

        // Deserialize back
        let restored = Manifest::from_json(&json).unwrap();
        assert_eq!(restored.version, manifest.version);
        assert_eq!(restored.len(), 2);

        let table = restored.get_element("table1").unwrap();
        assert_eq!(table.type_, "table");
        assert_eq!(table.description, Some("Sales data".to_string()));
    }

    #[test]
    fn test_manifest_empty_elements() {
        let manifest = Manifest::new();
        let json = manifest.to_json().unwrap();

        // Empty elements should still serialize
        assert!(json.contains("\"elements\": {}"));

        // And deserialize
        let restored = Manifest::from_json(&json).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_element_meta_builder() {
        let meta = ElementMeta::new("code")
            .with_source("utf8dok/code/snippet.rs")
            .with_hash("sha256:abcdef")
            .with_description("Rust code example");

        assert_eq!(meta.type_, "code");
        assert_eq!(meta.source, Some("utf8dok/code/snippet.rs".to_string()));
        assert_eq!(meta.hash, Some("sha256:abcdef".to_string()));
        assert_eq!(meta.description, Some("Rust code example".to_string()));
    }

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_remove_element() {
        let mut manifest = Manifest::new();
        manifest.add_element("elem1", ElementMeta::new("test"));

        assert_eq!(manifest.len(), 1);
        assert!(!manifest.is_empty());

        // Remove existing element
        let removed = manifest.remove_element("elem1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().type_, "test");
        assert!(manifest.is_empty());

        // Remove non-existent element
        let removed_again = manifest.remove_element("elem1");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_to_json_bytes() {
        let mut manifest = Manifest::new();
        manifest.add_element("fig1", ElementMeta::new("figure"));

        let bytes = manifest.to_json_bytes().unwrap();
        assert!(!bytes.is_empty());

        // Verify it's valid UTF-8 JSON
        let json_str = std::str::from_utf8(&bytes).unwrap();
        assert!(json_str.contains("\"version\""));
        assert!(json_str.contains("\"fig1\""));
    }

    #[test]
    fn test_from_json_bytes() {
        let json = r#"{"version":"1.0","generator":"test","generated_at":"2025-01-01","elements":{"x":{"type":"section"}}}"#;

        let manifest = Manifest::from_json_bytes(json.as_bytes()).unwrap();
        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.generator, "test");
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest.get_element("x").unwrap().type_, "section");
    }

    #[test]
    fn test_manifest_with_compiler_intent() {
        let mut manifest = Manifest::new();
        manifest.compiler = Some(DocumentIntent::default());

        let json = manifest.to_json().unwrap();
        assert!(json.contains("\"compiler\":"));

        let restored = Manifest::from_json(&json).unwrap();
        assert!(restored.compiler.is_some());
    }

    #[test]
    fn test_manifest_without_compiler_intent() {
        let manifest = Manifest::new();
        assert!(manifest.compiler.is_none());

        let json = manifest.to_json().unwrap();
        // compiler field should be skipped when None
        assert!(!json.contains("\"compiler\""));
    }

    #[test]
    fn test_is_empty_and_len() {
        let mut manifest = Manifest::new();

        assert!(manifest.is_empty());
        assert_eq!(manifest.len(), 0);

        manifest.add_element("a", ElementMeta::new("test"));
        assert!(!manifest.is_empty());
        assert_eq!(manifest.len(), 1);

        manifest.add_element("b", ElementMeta::new("test"));
        assert_eq!(manifest.len(), 2);

        manifest.remove_element("a");
        assert_eq!(manifest.len(), 1);
    }

    #[test]
    fn test_get_element_missing() {
        let manifest = Manifest::new();
        assert!(manifest.get_element("nonexistent").is_none());
    }

    #[test]
    fn test_element_meta_minimal() {
        // Test element with only type (no optional fields)
        let meta = ElementMeta::new("table");
        assert_eq!(meta.type_, "table");
        assert!(meta.source.is_none());
        assert!(meta.hash.is_none());
        assert!(meta.description.is_none());

        // Verify optional fields are skipped in JSON
        let mut manifest = Manifest::new();
        manifest.add_element("minimal", meta);
        let json = manifest.to_json().unwrap();

        // Should not contain source, hash, description for minimal element
        assert!(!json.contains("\"source\""));
        assert!(!json.contains("\"hash\""));
        // Note: description might still appear if it's in another element
    }

    #[test]
    fn test_manifest_roundtrip_bytes() {
        let mut original = Manifest::new();
        original.add_element(
            "fig1",
            ElementMeta::new("figure")
                .with_source("diagrams/arch.mmd")
                .with_hash("abc123"),
        );
        original.add_element(
            "table1",
            ElementMeta::new("table").with_description("Data table"),
        );

        // Roundtrip through bytes
        let bytes = original.to_json_bytes().unwrap();
        let restored = Manifest::from_json_bytes(&bytes).unwrap();

        assert_eq!(restored.version, original.version);
        assert_eq!(restored.len(), original.len());

        let fig = restored.get_element("fig1").unwrap();
        assert_eq!(fig.source, Some("diagrams/arch.mmd".to_string()));
        assert_eq!(fig.hash, Some("abc123".to_string()));

        let table = restored.get_element("table1").unwrap();
        assert_eq!(table.description, Some("Data table".to_string()));
    }

    #[test]
    fn test_overwrite_element() {
        let mut manifest = Manifest::new();

        manifest.add_element("elem", ElementMeta::new("original"));
        assert_eq!(manifest.get_element("elem").unwrap().type_, "original");

        // Overwrite with new value
        manifest.add_element("elem", ElementMeta::new("updated"));
        assert_eq!(manifest.get_element("elem").unwrap().type_, "updated");
        assert_eq!(manifest.len(), 1); // Still only one element
    }

    #[test]
    fn test_manifest_path_constant() {
        assert_eq!(MANIFEST_PATH, "utf8dok/manifest.json");
    }

    // ==================== Sprint 11: Manifest Edge Cases ====================

    #[test]
    fn test_manifest_with_all_element_meta_fields() {
        let mut manifest = Manifest::new();

        let meta = ElementMeta::new("diagram")
            .with_source("diagrams/flow.mmd")
            .with_hash("sha256:abc123def456")
            .with_description("System architecture overview");

        manifest.add_element("arch_diagram", meta);

        let retrieved = manifest.get_element("arch_diagram").unwrap();
        assert_eq!(retrieved.type_, "diagram");
        assert_eq!(retrieved.source, Some("diagrams/flow.mmd".to_string()));
        assert_eq!(retrieved.hash, Some("sha256:abc123def456".to_string()));
        assert_eq!(
            retrieved.description,
            Some("System architecture overview".to_string())
        );
    }

    #[test]
    fn test_manifest_element_iteration() {
        let mut manifest = Manifest::new();

        manifest.add_element("source", ElementMeta::new("asciidoc"));
        manifest.add_element("config", ElementMeta::new("toml"));
        manifest.add_element("diagram1", ElementMeta::new("mermaid"));

        // Count elements by iteration
        let count = manifest.elements.len();
        assert_eq!(count, 3);

        // Verify keys
        assert!(manifest.elements.contains_key("source"));
        assert!(manifest.elements.contains_key("config"));
        assert!(manifest.elements.contains_key("diagram1"));
    }

    #[test]
    fn test_manifest_json_pretty_format() {
        let mut manifest = Manifest::new();
        manifest.add_element("test", ElementMeta::new("test_type"));

        let bytes = manifest.to_json_bytes().unwrap();
        let json_str = String::from_utf8(bytes).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("elements").is_some());
    }

    #[test]
    fn test_manifest_empty_elements_json() {
        let manifest = Manifest::new();
        let bytes = manifest.to_json_bytes().unwrap();
        let json_str = String::from_utf8(bytes).unwrap();

        // Empty manifest should have version and empty elements
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.get("version").unwrap(), "1.0");
        assert!(parsed
            .get("elements")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_element_meta_builder_chain() {
        let meta = ElementMeta::new("source")
            .with_source("doc.adoc")
            .with_hash("hash1")
            .with_description("desc1")
            .with_source("updated.adoc") // Can update
            .with_hash("hash2"); // Can update again

        assert_eq!(meta.type_, "source");
        assert_eq!(meta.source, Some("updated.adoc".to_string()));
        assert_eq!(meta.hash, Some("hash2".to_string()));
        assert_eq!(meta.description, Some("desc1".to_string()));
    }

    #[test]
    fn test_manifest_special_characters_in_keys() {
        let mut manifest = Manifest::new();

        // Keys with special characters
        manifest.add_element("file-with-dashes", ElementMeta::new("test"));
        manifest.add_element("file_with_underscores", ElementMeta::new("test"));
        manifest.add_element("file.with.dots", ElementMeta::new("test"));

        // Roundtrip
        let bytes = manifest.to_json_bytes().unwrap();
        let restored = Manifest::from_json_bytes(&bytes).unwrap();

        assert!(restored.get_element("file-with-dashes").is_some());
        assert!(restored.get_element("file_with_underscores").is_some());
        assert!(restored.get_element("file.with.dots").is_some());
    }

    #[test]
    fn test_manifest_unicode_values() {
        let mut manifest = Manifest::new();

        let meta = ElementMeta::new("document").with_description("文档说明 - Document with 日本語");

        manifest.add_element("doc", meta);

        // Roundtrip
        let bytes = manifest.to_json_bytes().unwrap();
        let restored = Manifest::from_json_bytes(&bytes).unwrap();

        let retrieved = restored.get_element("doc").unwrap();
        assert_eq!(
            retrieved.description,
            Some("文档说明 - Document with 日本語".to_string())
        );
    }
}
