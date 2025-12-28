//! Attribute name completion
//!
//! Provides completion for :attribute-name: definitions.

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};

/// Known attribute with metadata
struct AttributeInfo {
    name: &'static str,
    description: &'static str,
    category: &'static str,
}

/// Standard AsciiDoc attributes
const STANDARD_ATTRIBUTES: &[AttributeInfo] = &[
    AttributeInfo { name: "author", description: "Document author name", category: "Document" },
    AttributeInfo { name: "email", description: "Author email address", category: "Document" },
    AttributeInfo { name: "version", description: "Document version", category: "Document" },
    AttributeInfo { name: "date", description: "Document date", category: "Document" },
    AttributeInfo { name: "revision", description: "Document revision info", category: "Document" },
    AttributeInfo { name: "title", description: "Document title override", category: "Document" },
    AttributeInfo { name: "description", description: "Document description", category: "Document" },
    AttributeInfo { name: "keywords", description: "Document keywords", category: "Document" },
    AttributeInfo { name: "toc", description: "Table of contents placement", category: "Layout" },
    AttributeInfo { name: "toclevels", description: "TOC depth (1-5)", category: "Layout" },
    AttributeInfo { name: "sectnums", description: "Enable section numbering", category: "Layout" },
    AttributeInfo { name: "sectanchors", description: "Enable section anchors", category: "Layout" },
    AttributeInfo { name: "icons", description: "Icon mode (font, image)", category: "Rendering" },
    AttributeInfo { name: "imagesdir", description: "Default images directory", category: "Paths" },
    AttributeInfo { name: "source-highlighter", description: "Code highlighter", category: "Rendering" },
];

/// Bridge Framework / ADR attributes
const ADR_ATTRIBUTES: &[AttributeInfo] = &[
    AttributeInfo { name: "status", description: "ADR status (Draft, Accepted, etc.)", category: "ADR" },
    AttributeInfo { name: "context", description: "Decision context summary", category: "ADR" },
    AttributeInfo { name: "decision", description: "Decision summary", category: "ADR" },
    AttributeInfo { name: "consequences", description: "Decision consequences summary", category: "ADR" },
    AttributeInfo { name: "deciders", description: "Decision makers", category: "ADR" },
    AttributeInfo { name: "consulted", description: "Consulted stakeholders", category: "ADR" },
    AttributeInfo { name: "informed", description: "Informed stakeholders", category: "ADR" },
    AttributeInfo { name: "adr-id", description: "ADR identifier (e.g., ADR-001)", category: "ADR" },
    AttributeInfo { name: "supersedes", description: "ADR this supersedes", category: "ADR" },
    AttributeInfo { name: "superseded-by", description: "ADR that supersedes this", category: "ADR" },
];

/// Custom/extension attributes
const EXTENSION_ATTRIBUTES: &[AttributeInfo] = &[
    AttributeInfo { name: "stage", description: "Document lifecycle stage", category: "Workflow" },
    AttributeInfo { name: "owner", description: "Document owner", category: "Workflow" },
    AttributeInfo { name: "reviewers", description: "Document reviewers", category: "Workflow" },
    AttributeInfo { name: "tags", description: "Document tags/labels", category: "Metadata" },
    AttributeInfo { name: "category", description: "Document category", category: "Metadata" },
];

/// Attribute name completer
pub struct AttributeCompleter;

impl AttributeCompleter {
    pub fn new() -> Self {
        Self
    }

    /// Complete attribute names
    pub fn complete(&self, prefix: &str) -> Vec<CompletionItem> {
        let all_attributes = STANDARD_ATTRIBUTES
            .iter()
            .chain(ADR_ATTRIBUTES.iter())
            .chain(EXTENSION_ATTRIBUTES.iter());

        all_attributes
            .filter(|attr| prefix.is_empty() || attr.name.starts_with(prefix))
            .map(|attr| self.attribute_to_completion(attr))
            .collect()
    }

    /// Convert attribute info to completion item
    fn attribute_to_completion(&self, attr: &AttributeInfo) -> CompletionItem {
        CompletionItem {
            label: attr.name.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(format!("[{}]", attr.category)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}**\n\n{}\n\n*Category: {}*",
                    attr.name, attr.description, attr.category
                ),
            })),
            insert_text: Some(format!("{}: ", attr.name)),
            ..Default::default()
        }
    }
}

impl Default for AttributeCompleter {
    fn default() -> Self {
        Self::new()
    }
}
