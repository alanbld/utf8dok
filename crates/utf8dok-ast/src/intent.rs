//! Document intent and invariant definitions
//!
//! This module defines the structures for expressing document compilation
//! intent and validation invariants. These are used by the compiler to
//! understand what guarantees the document should maintain.

use serde::{Deserialize, Serialize};

/// Document compilation intent
///
/// Captures the high-level goals and constraints for document compilation.
/// This is used by the compiler to make informed decisions about rendering
/// and validation.
///
/// # Example
///
/// ```
/// use utf8dok_ast::intent::{DocumentIntent, Invariant, ValidationLevel};
///
/// let intent = DocumentIntent::new()
///     .with_target_format("docx")
///     .with_validation_level(ValidationLevel::Strict)
///     .with_invariant(Invariant::new("heading_hierarchy", "Headings must be properly nested"))
///     .with_preserve_source(true);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentIntent {
    /// Target output format (e.g., "docx", "pdf", "html")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_format: Option<String>,

    /// Validation strictness level
    #[serde(default)]
    pub validation_level: ValidationLevel,

    /// List of invariants that must be maintained
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invariants: Vec<Invariant>,

    /// Whether to preserve source code in output
    #[serde(default)]
    pub preserve_source: bool,

    /// Custom options for specific renderers
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub options: std::collections::HashMap<String, String>,
}

/// A document invariant that must be maintained
///
/// Invariants represent rules or constraints that the document must
/// satisfy. The compiler uses these to validate the document structure
/// and content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    /// Unique identifier for this invariant type
    pub id: String,

    /// Human-readable description of the invariant
    pub description: String,

    /// Whether this invariant is critical (blocks compilation if violated)
    #[serde(default)]
    pub critical: bool,

    /// Optional context or parameters for the invariant
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl Invariant {
    /// Create a new invariant
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            critical: false,
            context: None,
        }
    }

    /// Mark this invariant as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Add context to this invariant
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Validation strictness level
///
/// Controls how strictly the compiler validates document structure
/// and content against invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    /// No validation - allow any document structure
    None,

    /// Lenient validation - warn on issues but continue
    #[default]
    Lenient,

    /// Strict validation - fail on any invariant violation
    Strict,

    /// Pedantic validation - fail on any issue including style
    Pedantic,
}

impl DocumentIntent {
    /// Create a new empty document intent
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target format
    pub fn with_target_format(mut self, format: impl Into<String>) -> Self {
        self.target_format = Some(format.into());
        self
    }

    /// Set the validation level
    pub fn with_validation_level(mut self, level: ValidationLevel) -> Self {
        self.validation_level = level;
        self
    }

    /// Add an invariant
    pub fn with_invariant(mut self, invariant: Invariant) -> Self {
        self.invariants.push(invariant);
        self
    }

    /// Set whether to preserve source
    pub fn with_preserve_source(mut self, preserve: bool) -> Self {
        self.preserve_source = preserve;
        self
    }

    /// Add a custom option
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_intent_default() {
        let intent = DocumentIntent::new();
        assert_eq!(intent.target_format, None);
        assert_eq!(intent.validation_level, ValidationLevel::Lenient);
        assert!(intent.invariants.is_empty());
        assert!(!intent.preserve_source);
    }

    #[test]
    fn test_document_intent_builder() {
        let intent = DocumentIntent::new()
            .with_target_format("docx")
            .with_validation_level(ValidationLevel::Strict)
            .with_invariant(Invariant::new("heading_hierarchy", "Nested headings").critical())
            .with_preserve_source(true);

        assert_eq!(intent.target_format, Some("docx".to_string()));
        assert_eq!(intent.validation_level, ValidationLevel::Strict);
        assert_eq!(intent.invariants.len(), 1);
        assert!(intent.invariants[0].critical);
        assert!(intent.preserve_source);
    }

    #[test]
    fn test_invariant_builder() {
        let invariant = Invariant::new("xref_valid", "All cross-references must resolve")
            .critical()
            .with_context("document-wide");

        assert_eq!(invariant.id, "xref_valid");
        assert!(invariant.critical);
        assert_eq!(invariant.context, Some("document-wide".to_string()));
    }

    #[test]
    fn test_validation_level_serialize() {
        assert_eq!(
            serde_json::to_string(&ValidationLevel::Strict).unwrap(),
            "\"strict\""
        );
        assert_eq!(
            serde_json::from_str::<ValidationLevel>("\"pedantic\"").unwrap(),
            ValidationLevel::Pedantic
        );
    }

    #[test]
    fn test_document_intent_serialize() {
        let intent = DocumentIntent::new()
            .with_target_format("pdf")
            .with_validation_level(ValidationLevel::Strict);

        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"target_format\":\"pdf\""));
        assert!(json.contains("\"validation_level\":\"strict\""));

        let restored: DocumentIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.target_format, Some("pdf".to_string()));
        assert_eq!(restored.validation_level, ValidationLevel::Strict);
    }
}
