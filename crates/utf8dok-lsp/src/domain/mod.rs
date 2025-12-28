//! Domain Intelligence Engine for utf8dok LSP
//!
//! Provides context-aware assistance:
//! - Completion (xrefs, attributes, values, blocks)
//! - Validation (ADR rules, template detection)
//! - Code actions (quick fixes)

pub mod completion;
pub mod validation;

#[cfg(test)]
mod tests;

pub use completion::CompletionEngine;
pub use validation::DomainValidator;

use tower_lsp::lsp_types::{CompletionItem, Diagnostic, CodeAction, CodeActionParams, Position};

/// Main domain engine coordinating all domain intelligence
pub struct DomainEngine {
    completion_engine: CompletionEngine,
    validator: DomainValidator,
}

impl DomainEngine {
    /// Create a new domain engine with default configuration
    pub fn new() -> Self {
        Self {
            completion_engine: CompletionEngine::new(),
            validator: DomainValidator::new(),
        }
    }

    /// Get completions at the given position
    pub fn get_completions(&self, text: &str, position: Position) -> Vec<CompletionItem> {
        self.completion_engine.get_completions(text, position)
    }

    /// Validate the document for domain-specific rules
    #[allow(dead_code)]
    pub fn validate_document(&self, text: &str) -> Vec<Diagnostic> {
        self.validator.validate_document(text)
    }

    /// Get code actions for the given context
    pub fn get_code_actions(&self, text: &str, params: &CodeActionParams) -> Vec<CodeAction> {
        self.validator.get_code_actions(text, params)
    }
}

impl Default for DomainEngine {
    fn default() -> Self {
        Self::new()
    }
}
