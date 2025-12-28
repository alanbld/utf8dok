//! Block type completion
//!
//! Provides completion for [block-type] declarations.

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};

/// Block type with metadata
#[allow(dead_code)]
struct BlockType {
    name: &'static str,
    description: &'static str,
    snippet: &'static str,
}

/// Available block types
const BLOCK_TYPES: &[BlockType] = &[
    BlockType {
        name: "source",
        description: "Source code block with syntax highlighting",
        snippet: "source,${1:language}]\n----\n${2:code}\n----",
    },
    BlockType {
        name: "listing",
        description: "Literal listing block",
        snippet: "listing]\n----\n${1:content}\n----",
    },
    BlockType {
        name: "example",
        description: "Example block",
        snippet: "example]\n====\n${1:example content}\n====",
    },
    BlockType {
        name: "sidebar",
        description: "Sidebar block",
        snippet: "sidebar]\n****\n${1:sidebar content}\n****",
    },
    BlockType {
        name: "quote",
        description: "Quote block with attribution",
        snippet: "quote,${1:attribution}]\n____\n${2:quote text}\n____",
    },
    BlockType {
        name: "verse",
        description: "Verse/poetry block",
        snippet: "verse,${1:attribution}]\n____\n${2:verse text}\n____",
    },
    BlockType {
        name: "literal",
        description: "Literal block (preserves whitespace)",
        snippet: "literal]\n....\n${1:literal content}\n....",
    },
    BlockType {
        name: "NOTE",
        description: "Note admonition",
        snippet: "NOTE]\n====\n${1:note content}\n====",
    },
    BlockType {
        name: "TIP",
        description: "Tip admonition",
        snippet: "TIP]\n====\n${1:tip content}\n====",
    },
    BlockType {
        name: "WARNING",
        description: "Warning admonition",
        snippet: "WARNING]\n====\n${1:warning content}\n====",
    },
    BlockType {
        name: "IMPORTANT",
        description: "Important admonition",
        snippet: "IMPORTANT]\n====\n${1:important content}\n====",
    },
    BlockType {
        name: "CAUTION",
        description: "Caution admonition",
        snippet: "CAUTION]\n====\n${1:caution content}\n====",
    },
    BlockType {
        name: "cols",
        description: "Table with column specification",
        snippet: "cols=\"${1:1,1}\"]\n|===\n| ${2:Header 1} | ${3:Header 2}\n\n| ${4:Cell 1} | ${5:Cell 2}\n|===",
    },
    BlockType {
        name: "options",
        description: "Block options",
        snippet: "options=\"${1:header,footer}\"]",
    },
];

/// Simplified block types for quick completion
const SIMPLE_BLOCKS: &[(&str, &str)] = &[
    ("source", "Source code block"),
    ("listing", "Listing block"),
    ("example", "Example block"),
    ("sidebar", "Sidebar block"),
    ("quote", "Quote block"),
    ("admonition", "Admonition block"),
    ("table", "Table block"),
    ("passthrough", "Passthrough block"),
    ("open", "Open block"),
];

/// Block type completer
pub struct BlockCompleter;

impl BlockCompleter {
    pub fn new() -> Self {
        Self
    }

    /// Complete block types
    pub fn complete(&self, prefix: &str) -> Vec<CompletionItem> {
        SIMPLE_BLOCKS
            .iter()
            .filter(|(name, _)| prefix.is_empty() || name.starts_with(prefix))
            .map(|(name, desc)| self.block_to_completion(name, desc))
            .collect()
    }

    /// Convert block type to completion item
    fn block_to_completion(&self, name: &str, description: &str) -> CompletionItem {
        // Find snippet if available
        let snippet = BLOCK_TYPES
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.snippet.to_string());

        CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some(description.to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**[{}]**\n\n{}", name, description),
            })),
            insert_text: snippet.or_else(|| Some(format!("{}]", name))),
            ..Default::default()
        }
    }
}

impl Default for BlockCompleter {
    fn default() -> Self {
        Self::new()
    }
}
