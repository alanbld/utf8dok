//! Completion engine for AsciiDoc documents
//!
//! Provides intelligent completions for:
//! - Cross-references: <<section-id>>
//! - Attributes: :name: and {name}
//! - Attribute values: :status: Draft
//! - Block types: [source]

mod xref;
mod attribute;
mod value;
mod block;

pub use xref::XrefCompleter;
pub use attribute::AttributeCompleter;
pub use value::ValueCompleter;
pub use block::BlockCompleter;

use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{CompletionItem, Position};

/// Context detected for completion
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// After << for cross-reference
    Xref { prefix: String },
    /// At line start for attribute name
    AttributeName { prefix: String },
    /// After :name: for attribute value
    AttributeValue { name: String, prefix: String },
    /// After [ for block type
    BlockType { prefix: String },
    /// No completion context detected
    None,
}

/// Main completion engine
pub struct CompletionEngine {
    xref_completer: XrefCompleter,
    attribute_completer: AttributeCompleter,
    value_completer: ValueCompleter,
    block_completer: BlockCompleter,
}

impl CompletionEngine {
    /// Create a new completion engine
    pub fn new() -> Self {
        Self {
            xref_completer: XrefCompleter::new(),
            attribute_completer: AttributeCompleter::new(),
            value_completer: ValueCompleter::new(),
            block_completer: BlockCompleter::new(),
        }
    }

    /// Get completions at the given position (static method for tests)
    #[allow(dead_code)]
    pub fn complete(text: &str, position: Position) -> Vec<CompletionItem> {
        let engine = Self::new();
        engine.get_completions(text, position)
    }

    /// Get completions at the given position
    pub fn get_completions(&self, text: &str, position: Position) -> Vec<CompletionItem> {
        let context = self.detect_context(text, position);

        match context {
            CompletionContext::Xref { prefix } => {
                self.xref_completer.complete(text, &prefix)
            }
            CompletionContext::AttributeName { prefix } => {
                self.attribute_completer.complete(&prefix)
            }
            CompletionContext::AttributeValue { name, prefix } => {
                self.value_completer.complete(&name, &prefix)
            }
            CompletionContext::BlockType { prefix } => {
                self.block_completer.complete(&prefix)
            }
            CompletionContext::None => Vec::new(),
        }
    }

    /// Detect the completion context at the given position
    fn detect_context(&self, text: &str, position: Position) -> CompletionContext {
        let lines: Vec<&str> = text.lines().collect();
        let line_idx = position.line as usize;

        if line_idx >= lines.len() {
            return CompletionContext::None;
        }

        let line = lines[line_idx];
        let char_idx = (position.character as usize).min(line.len());
        let line_before = &line[..char_idx];

        // Check for xref: <<prefix
        if let Some(prefix) = self.detect_xref_context(line_before) {
            return CompletionContext::Xref { prefix };
        }

        // Check for attribute value: :name: prefix
        if let Some((name, prefix)) = self.detect_value_context(line_before) {
            return CompletionContext::AttributeValue { name, prefix };
        }

        // Check for attribute name at line start: :prefix
        if let Some(prefix) = self.detect_attribute_context(line_before) {
            return CompletionContext::AttributeName { prefix };
        }

        // Check for block type: [prefix
        if let Some(prefix) = self.detect_block_context(line_before) {
            return CompletionContext::BlockType { prefix };
        }

        CompletionContext::None
    }

    /// Detect xref context (after <<)
    fn detect_xref_context(&self, line_before: &str) -> Option<String> {
        static XREF_RE: OnceLock<Regex> = OnceLock::new();
        let re = XREF_RE.get_or_init(|| Regex::new(r"<<([\w\-]*)$").unwrap());

        re.captures(line_before).map(|cap| {
            cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
        })
    }

    /// Detect attribute name context (: at line start)
    fn detect_attribute_context(&self, line_before: &str) -> Option<String> {
        let trimmed = line_before.trim_start();

        // Must be at line start (or only whitespace before)
        if line_before.len() - trimmed.len() > 0 && !line_before.chars().all(|c| c.is_whitespace() || c == ':') {
            return None;
        }

        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w\-]*)$").unwrap());

        re.captures(trimmed).map(|cap| {
            cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
        })
    }

    /// Detect attribute value context (:name: value)
    fn detect_value_context(&self, line_before: &str) -> Option<(String, String)> {
        static VALUE_RE: OnceLock<Regex> = OnceLock::new();
        let re = VALUE_RE.get_or_init(|| Regex::new(r"^:([\w\-]+):\s*(\S*)$").unwrap());

        re.captures(line_before.trim_start()).map(|cap| {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let prefix = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            (name, prefix)
        })
    }

    /// Detect block type context (after [)
    fn detect_block_context(&self, line_before: &str) -> Option<String> {
        let trimmed = line_before.trim_start();

        static BLOCK_RE: OnceLock<Regex> = OnceLock::new();
        let re = BLOCK_RE.get_or_init(|| Regex::new(r"^\[([\w\-]*)$").unwrap());

        re.captures(trimmed).map(|cap| {
            cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
        })
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}
