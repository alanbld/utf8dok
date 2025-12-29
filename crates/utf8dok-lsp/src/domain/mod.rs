//! Domain Intelligence Engine for utf8dok LSP
//!
//! Provides context-aware assistance:
//! - Completion (xrefs, attributes, values, blocks)
//! - Validation (ADR rules, template detection)
//! - Code actions (quick fixes)
//! - Semantic highlighting (domain-aware token classification)
//!
//! Phase 10 introduces the plugin-based platform architecture:
//! - `traits`: The universal plugin contract
//! - `registry`: Plugin management and domain detection
//! - `plugins`: Domain-specific implementations (Bridge, RFC, Generic)
//! - `semantic`: Semantic token analysis

// Legacy modules (Phase 9)
pub mod completion;
pub mod validation;

// Platform modules (Phase 10)
pub mod plugins;
pub mod registry;
pub mod semantic;
pub mod traits;

#[cfg(test)]
mod platform_tests;
#[cfg(test)]
mod tests;

pub use completion::CompletionEngine;
#[allow(unused_imports)]
pub use plugins::{BridgePlugin, GenericPlugin, RfcPlugin};
pub use registry::DomainRegistry;
pub use semantic::{SemanticAnalyzer, SemanticTokenInfo};
pub use traits::DocumentDomain;
pub use validation::DomainValidator;

use tower_lsp::lsp_types::{CodeAction, CodeActionParams, CompletionItem, Diagnostic, Position};

/// Main domain engine coordinating all domain intelligence.
///
/// This engine wraps both the new registry-based platform (Phase 10)
/// and the legacy completion/validation systems (Phase 9) for
/// backward compatibility during migration.
pub struct DomainEngine {
    /// The domain registry for plugin-based detection
    registry: DomainRegistry,
    /// Semantic analyzer for syntax highlighting
    semantic_analyzer: SemanticAnalyzer,
    /// Legacy completion engine (Phase 9)
    completion_engine: CompletionEngine,
    /// Legacy validator (Phase 9)
    validator: DomainValidator,
}

impl DomainEngine {
    /// Create a new domain engine with default configuration
    pub fn new() -> Self {
        let registry = DomainRegistry::new();
        let semantic_analyzer = SemanticAnalyzer::new(registry.clone());

        Self {
            registry,
            semantic_analyzer,
            completion_engine: CompletionEngine::new(),
            validator: DomainValidator::new(),
        }
    }

    /// Detect the domain for a document
    #[allow(dead_code)]
    pub fn detect_domain(&self, text: &str) -> &str {
        if let Some((domain, _score)) = self.registry.detect_domain(text) {
            // Return static string based on domain name
            match domain.name() {
                "bridge" => "bridge",
                "rfc" => "rfc",
                _ => "generic",
            }
        } else {
            "generic"
        }
    }

    /// Get completions at the given position.
    ///
    /// Uses the registry to detect the domain and delegates to
    /// the appropriate plugin, falling back to legacy system.
    pub fn get_completions(&self, text: &str, position: Position) -> Vec<CompletionItem> {
        // Try new platform first for high-confidence domains
        if let Some((domain, score)) = self.registry.detect_domain(text) {
            if score > 0.7 {
                // Get line prefix for completion context
                let line_prefix = self.get_line_prefix(text, position);
                let completions = domain.complete(position, &line_prefix);
                if !completions.is_empty() {
                    return completions;
                }
            }
        }

        // Fallback to legacy Phase 9 completion
        self.completion_engine.get_completions(text, position)
    }

    /// Get semantic tokens for the document.
    pub fn get_semantic_tokens(&self, text: &str) -> Vec<SemanticTokenInfo> {
        self.semantic_analyzer.analyze(text)
    }

    /// Validate the document for domain-specific rules.
    #[allow(dead_code)]
    pub fn validate_document(&self, text: &str) -> Vec<Diagnostic> {
        // Try new platform first
        if let Some((domain, score)) = self.registry.detect_domain(text) {
            if score > 0.5 {
                let diagnostics = domain.validate(text);
                if !diagnostics.is_empty() {
                    return diagnostics;
                }
            }
        }

        // Fallback to legacy validation
        self.validator.validate_document(text)
    }

    /// Get code actions for the given context
    pub fn get_code_actions(&self, text: &str, params: &CodeActionParams) -> Vec<CodeAction> {
        self.validator.get_code_actions(text, params)
    }

    /// Get the registry for direct access
    #[allow(dead_code)]
    pub fn registry(&self) -> &DomainRegistry {
        &self.registry
    }

    /// Get the semantic analyzer for direct access
    pub fn semantic_analyzer(&self) -> &SemanticAnalyzer {
        &self.semantic_analyzer
    }

    /// Extract the line prefix up to the cursor position
    fn get_line_prefix(&self, text: &str, position: Position) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let line_idx = position.line as usize;

        if line_idx >= lines.len() {
            return String::new();
        }

        let line = lines[line_idx];
        let char_idx = (position.character as usize).min(line.len());
        line[..char_idx].to_string()
    }
}

impl Default for DomainEngine {
    fn default() -> Self {
        Self::new()
    }
}
