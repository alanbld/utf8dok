//! Attribute value completion
//!
//! Provides completion for attribute values like :status: Draft

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};

/// Value option with metadata
struct ValueOption {
    value: &'static str,
    description: &'static str,
}

/// Status values for ADRs
const STATUS_VALUES: &[ValueOption] = &[
    ValueOption { value: "Draft", description: "Initial draft, not yet reviewed" },
    ValueOption { value: "Proposed", description: "Proposed for review and discussion" },
    ValueOption { value: "Accepted", description: "Accepted and in effect" },
    ValueOption { value: "Rejected", description: "Rejected after review" },
    ValueOption { value: "Deprecated", description: "No longer recommended" },
    ValueOption { value: "Superseded", description: "Replaced by another decision" },
];

/// TOC placement values
const TOC_VALUES: &[ValueOption] = &[
    ValueOption { value: "left", description: "Table of contents on the left side" },
    ValueOption { value: "right", description: "Table of contents on the right side" },
    ValueOption { value: "preamble", description: "Table of contents after preamble" },
    ValueOption { value: "auto", description: "Automatic TOC placement" },
    ValueOption { value: "macro", description: "TOC placed where toc::[] macro appears" },
];

/// Icon mode values
const ICONS_VALUES: &[ValueOption] = &[
    ValueOption { value: "font", description: "Use Font Awesome icons" },
    ValueOption { value: "image", description: "Use image files for icons" },
];

/// Source highlighter values
const HIGHLIGHTER_VALUES: &[ValueOption] = &[
    ValueOption { value: "highlight.js", description: "Highlight.js syntax highlighter" },
    ValueOption { value: "rouge", description: "Rouge syntax highlighter" },
    ValueOption { value: "pygments", description: "Pygments syntax highlighter" },
    ValueOption { value: "coderay", description: "CodeRay syntax highlighter" },
];

/// Boolean-like values
const BOOLEAN_VALUES: &[ValueOption] = &[
    ValueOption { value: "true", description: "Enable the feature" },
    ValueOption { value: "false", description: "Disable the feature" },
];

/// Attribute value completer
pub struct ValueCompleter;

impl ValueCompleter {
    pub fn new() -> Self {
        Self
    }

    /// Complete attribute values based on attribute name
    pub fn complete(&self, attribute_name: &str, prefix: &str) -> Vec<CompletionItem> {
        let values = self.get_values_for_attribute(attribute_name);

        values
            .iter()
            .filter(|v| prefix.is_empty() || v.value.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|v| self.value_to_completion(v, attribute_name))
            .collect()
    }

    /// Get valid values for a given attribute
    fn get_values_for_attribute(&self, name: &str) -> &'static [ValueOption] {
        match name {
            "status" => STATUS_VALUES,
            "toc" => TOC_VALUES,
            "icons" => ICONS_VALUES,
            "source-highlighter" => HIGHLIGHTER_VALUES,
            "sectnums" | "sectanchors" | "numbered" => BOOLEAN_VALUES,
            _ => &[],
        }
    }

    /// Convert value option to completion item
    fn value_to_completion(&self, opt: &ValueOption, attr_name: &str) -> CompletionItem {
        CompletionItem {
            label: opt.value.to_string(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some(format!(":{}: {}", attr_name, opt.value)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: opt.description.to_string(),
            })),
            ..Default::default()
        }
    }
}

impl Default for ValueCompleter {
    fn default() -> Self {
        Self::new()
    }
}
