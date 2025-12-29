//! Domain Plugin Trait
//!
//! Defines the universal contract for domain plugins.
//! Each plugin implements domain-specific logic for validation,
//! completion, and semantic classification.

use tower_lsp::lsp_types::{CompletionItem, Diagnostic, Position, SemanticTokenType};

/// The universal trait that all domain plugins must implement.
///
/// This trait defines the contract between the platform and domain-specific
/// intelligence. Each domain (Bridge/ADR, RFC, etc.) implements this trait
/// to provide specialized behavior.
pub trait DocumentDomain: Send + Sync {
    /// Unique identifier for this domain (e.g., "bridge", "rfc", "generic")
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Return a confidence score (0.0 to 1.0) that this domain understands the document.
    ///
    /// The registry uses this score to select the most appropriate domain.
    /// - 0.0: Definitely not this domain
    /// - 0.5: Might be this domain
    /// - 1.0: Definitely this domain
    fn score_document(&self, text: &str) -> f32;

    /// Domain-specific validation rules.
    ///
    /// Returns diagnostics for issues specific to this domain
    /// (e.g., missing ADR sections, invalid RFC category).
    fn validate(&self, text: &str) -> Vec<Diagnostic>;

    /// Domain-specific completions at a given position.
    ///
    /// The `line_prefix` contains the text from the start of the line
    /// up to the cursor position.
    fn complete(&self, position: Position, line_prefix: &str) -> Vec<CompletionItem>;

    /// Classify a document element for semantic highlighting.
    ///
    /// # Arguments
    /// * `element_type` - The type of element: "header", "attribute_name", "attribute_value", etc.
    /// * `value` - The actual text value of the element
    ///
    /// # Returns
    /// The semantic token type for syntax highlighting, or None if not classifiable.
    fn classify_element(&self, element_type: &str, value: &str) -> Option<SemanticTokenType>;

    /// Optional: Domain-specific token types for the legend.
    ///
    /// Most domains can use the standard types, but this allows
    /// domains to register custom token types if needed.
    #[allow(dead_code)]
    fn token_types(&self) -> Vec<SemanticTokenType> {
        vec![
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
        ]
    }

    /// Optional: Provide domain-specific code actions.
    ///
    /// Default implementation returns empty, domains can override.
    #[allow(dead_code)]
    fn code_actions(
        &self,
        _text: &str,
        _range: tower_lsp::lsp_types::Range,
    ) -> Vec<tower_lsp::lsp_types::CodeAction> {
        Vec::new()
    }
}
