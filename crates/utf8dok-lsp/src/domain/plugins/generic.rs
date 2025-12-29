//! Generic Domain Plugin
//!
//! Provides basic domain intelligence for any AsciiDoc document.
//! This is the fallback plugin when no specific domain matches.

use crate::domain::traits::DocumentDomain;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Diagnostic, Position, SemanticTokenType,
};

/// Generic/fallback domain plugin
pub struct GenericPlugin;

impl GenericPlugin {
    pub fn new() -> Self {
        Self
    }

    /// Get common attribute completions
    fn complete_attribute_names(&self, prefix: &str) -> Vec<CompletionItem> {
        let attributes = [
            ("author", "Document author"),
            ("date", "Document date"),
            ("version", "Document version"),
            ("title", "Document title"),
            ("toc", "Table of contents position"),
            ("toclevels", "TOC depth"),
            ("icons", "Icon mode (font, image)"),
            ("source-highlighter", "Syntax highlighter"),
            ("sectanchors", "Enable section anchors"),
            ("sectnums", "Enable section numbering"),
        ];

        attributes
            .iter()
            .filter(|(name, _)| prefix.is_empty() || name.starts_with(prefix))
            .map(|(name, desc)| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(desc.to_string()),
                insert_text: Some(format!("{}: ", name)),
                ..Default::default()
            })
            .collect()
    }
}

impl Default for GenericPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentDomain for GenericPlugin {
    fn name(&self) -> &str {
        "generic"
    }

    fn score_document(&self, text: &str) -> f32 {
        // Generic always matches with low confidence
        // This ensures it's used as a fallback
        if text.is_empty() {
            0.1
        } else {
            0.2
        }
    }

    fn validate(&self, _text: &str) -> Vec<Diagnostic> {
        // Generic domain doesn't enforce any specific validation
        Vec::new()
    }

    fn complete(&self, _position: Position, line_prefix: &str) -> Vec<CompletionItem> {
        let trimmed = line_prefix.trim();

        // Attribute name completion
        if trimmed.starts_with(':') && !trimmed.contains(": ") {
            let prefix = trimmed.trim_start_matches(':');
            return self.complete_attribute_names(prefix);
        }

        Vec::new()
    }

    fn classify_element(&self, element_type: &str, _value: &str) -> Option<SemanticTokenType> {
        // Basic classification for all documents
        match element_type {
            "header" => Some(SemanticTokenType::CLASS),
            "attribute_name" => Some(SemanticTokenType::VARIABLE),
            "attribute_value" => Some(SemanticTokenType::STRING),
            "xref" | "anchor" => Some(SemanticTokenType::VARIABLE),
            "block_delimiter" => Some(SemanticTokenType::COMMENT),
            "comment" => Some(SemanticTokenType::COMMENT),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_always_matches() {
        let plugin = GenericPlugin::new();

        let score = plugin.score_document("Any document");
        assert!(score > 0.0 && score < 0.5);
    }

    #[test]
    fn test_generic_no_validation() {
        let plugin = GenericPlugin::new();

        let diagnostics = plugin.validate("Any document with :invalid: attributes");
        assert!(diagnostics.is_empty());
    }
}
