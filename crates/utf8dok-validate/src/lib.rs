//! utf8dok-validate - Document validation engine
//!
//! This crate provides a pluggable validation engine for checking document
//! structure, content, and invariants.
//!
//! # Architecture
//!
//! The validation engine uses a trait-based design where individual validators
//! implement the `Validator` trait. The `ValidationEngine` orchestrates running
//! all registered validators and collecting diagnostics.
//!
//! # Example
//!
//! ```
//! use utf8dok_validate::{ValidationEngine, SectionHierarchyValidator};
//! use utf8dok_ast::Document;
//!
//! let mut engine = ValidationEngine::new();
//! engine.add_validator(Box::new(SectionHierarchyValidator));
//!
//! let doc = Document::new();
//! let diagnostics = engine.validate(&doc);
//! ```

pub mod hierarchy;

use utf8dok_ast::Document;
use utf8dok_core::diagnostics::Diagnostic;

// Re-export validators
pub use hierarchy::SectionHierarchyValidator;

/// Trait for document validators
///
/// Validators inspect a document and return a list of diagnostics
/// for any issues found. Each validator has a unique code prefix
/// for its diagnostics.
pub trait Validator: Send + Sync {
    /// Get the validator's unique code (e.g., "DOC1" for document structure)
    fn code(&self) -> &'static str;

    /// Get a human-readable name for this validator
    fn name(&self) -> &'static str {
        "unnamed"
    }

    /// Validate the document and return any diagnostics
    fn validate(&self, doc: &Document) -> Vec<Diagnostic>;
}

/// Validation engine that orchestrates multiple validators
///
/// The engine manages a collection of validators and runs them
/// against documents, collecting all diagnostics.
pub struct ValidationEngine {
    /// Registered validators
    validators: Vec<Box<dyn Validator>>,
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationEngine {
    /// Create a new empty validation engine
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Create an engine with default validators
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();
        engine.add_validator(Box::new(SectionHierarchyValidator));
        engine
    }

    /// Add a validator to the engine
    pub fn add_validator(&mut self, validator: Box<dyn Validator>) {
        self.validators.push(validator);
    }

    /// Get the number of registered validators
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Get the names of all registered validators
    pub fn validator_names(&self) -> Vec<&'static str> {
        self.validators.iter().map(|v| v.name()).collect()
    }

    /// Validate a document using all registered validators
    ///
    /// Returns a vector of all diagnostics from all validators.
    pub fn validate(&self, doc: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for validator in &self.validators {
            let validator_diagnostics = validator.validate(doc);
            diagnostics.extend(validator_diagnostics);
        }

        diagnostics
    }

    /// Check if a document has any errors
    pub fn has_errors(&self, doc: &Document) -> bool {
        self.validate(doc).iter().any(|d| d.is_error())
    }

    /// Check if a document has any warnings or errors
    pub fn has_issues(&self, doc: &Document) -> bool {
        !self.validate(doc).is_empty()
    }
}

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use utf8dok_ast::{Block, Heading, Inline};

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_engine_new() {
        let engine = ValidationEngine::new();
        assert_eq!(engine.validator_count(), 0);
    }

    #[test]
    fn test_engine_with_defaults() {
        let engine = ValidationEngine::with_defaults();
        assert!(engine.validator_count() > 0);
        assert!(engine.validator_names().contains(&"section-hierarchy"));
    }

    #[test]
    fn test_engine_add_validator() {
        let mut engine = ValidationEngine::new();
        engine.add_validator(Box::new(SectionHierarchyValidator));
        assert_eq!(engine.validator_count(), 1);
    }

    #[test]
    fn test_validate_empty_document() {
        let engine = ValidationEngine::with_defaults();
        let doc = Document::new();
        let diagnostics = engine.validate(&doc);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_valid_hierarchy() {
        let engine = ValidationEngine::with_defaults();

        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Level 1".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Heading(Heading {
                    level: 2,
                    text: vec![Inline::Text("Level 2".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Heading(Heading {
                    level: 3,
                    text: vec![Inline::Text("Level 3".to_string())],
                    style_id: None,
                    anchor: None,
                }),
            ],
            intent: None,
        };

        let diagnostics = engine.validate(&doc);
        assert!(
            diagnostics.is_empty(),
            "Valid hierarchy should have no diagnostics"
        );
    }

    #[test]
    fn test_validate_hierarchy_jump() {
        let engine = ValidationEngine::with_defaults();

        // Create a document with a hierarchy jump (Level 2 -> Level 4)
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Level 1".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Heading(Heading {
                    level: 2,
                    text: vec![Inline::Text("Level 2".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                // Jump from level 2 to level 4 - missing level 3!
                Block::Heading(Heading {
                    level: 4,
                    text: vec![Inline::Text("Level 4".to_string())],
                    style_id: None,
                    anchor: None,
                }),
            ],
            intent: None,
        };

        let diagnostics = engine.validate(&doc);

        // Should have exactly one warning about the hierarchy jump
        assert_eq!(diagnostics.len(), 1, "Should detect one hierarchy jump");

        let diag = &diagnostics[0];
        assert_eq!(diag.code, Some("DOC101".to_string()));
        assert!(diag.is_warning());
        assert!(diag.message.contains("Level 2"));
        assert!(diag.message.contains("Level 4"));
    }

    #[test]
    fn test_validate_first_heading_not_level_1() {
        let engine = ValidationEngine::with_defaults();

        // Document starting with level 3 heading
        let doc = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![Block::Heading(Heading {
                level: 3,
                text: vec![Inline::Text("Starting at Level 3".to_string())],
                style_id: None,
                anchor: None,
            })],
            intent: None,
        };

        let diagnostics = engine.validate(&doc);

        // Should warn about starting at level 3 (jump from implicit 0 to 3)
        assert!(!diagnostics.is_empty(), "Should detect hierarchy issue");

        let has_doc101 = diagnostics
            .iter()
            .any(|d| d.code == Some("DOC101".to_string()));
        assert!(has_doc101, "Should have DOC101 diagnostic");
    }

    #[test]
    fn test_has_errors() {
        let engine = ValidationEngine::with_defaults();
        let doc = Document::new();
        assert!(!engine.has_errors(&doc));
    }

    #[test]
    fn test_has_issues() {
        let engine = ValidationEngine::with_defaults();

        let doc_with_jump = Document {
            metadata: utf8dok_ast::DocumentMeta::default(),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Level 1".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Heading(Heading {
                    level: 4,
                    text: vec![Inline::Text("Level 4".to_string())],
                    style_id: None,
                    anchor: None,
                }),
            ],
            intent: None,
        };

        assert!(engine.has_issues(&doc_with_jump));
    }
}
