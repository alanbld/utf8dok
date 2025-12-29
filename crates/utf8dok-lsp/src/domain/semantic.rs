//! Semantic Token Analyzer
//!
//! Produces semantic tokens for syntax highlighting using domain-aware classification.

use crate::domain::registry::DomainRegistry;
use crate::domain::traits::DocumentDomain;
use regex::Regex;
use std::sync::{Arc, OnceLock};
use tower_lsp::lsp_types::SemanticTokenType;

/// Information about a semantic token
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTokenInfo {
    /// The text of the token
    pub text: String,
    /// Token type for highlighting
    pub token_type: SemanticTokenType,
    /// Line number (0-indexed)
    pub line: u32,
    /// Start character position (0-indexed)
    pub start_char: u32,
    /// Length of the token
    pub length: u32,
}

/// LSP semantic token in delta format
#[derive(Debug, Clone)]
pub struct LspSemanticToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers: u32,
}

/// Semantic analyzer that uses domain plugins for classification
pub struct SemanticAnalyzer {
    registry: DomainRegistry,
}

impl SemanticAnalyzer {
    pub fn new(registry: DomainRegistry) -> Self {
        Self { registry }
    }

    /// Analyze a document and produce semantic tokens
    pub fn analyze(&self, text: &str) -> Vec<SemanticTokenInfo> {
        let mut tokens = Vec::new();

        // Detect the domain for this document
        let (domain, _score) = self
            .registry
            .detect_domain(text)
            .unwrap_or_else(|| (self.registry.fallback(), 0.1));

        // Parse and classify each line
        for (line_num, line) in text.lines().enumerate() {
            tokens.extend(self.analyze_line(line, line_num as u32, &domain));
        }

        tokens
    }

    /// Analyze a single line and extract tokens
    fn analyze_line(
        &self,
        line: &str,
        line_num: u32,
        domain: &Arc<dyn DocumentDomain>,
    ) -> Vec<SemanticTokenInfo> {
        let mut tokens = Vec::new();

        // Check for header
        if let Some(header_token) = self.extract_header(line, line_num, domain) {
            tokens.push(header_token);
            return tokens; // Headers are standalone
        }

        // Check for attribute
        if let Some(attr_tokens) = self.extract_attribute(line, line_num, domain) {
            tokens.extend(attr_tokens);
            return tokens;
        }

        // Check for anchor
        if let Some(anchor_token) = self.extract_anchor(line, line_num, domain) {
            tokens.push(anchor_token);
        }

        tokens
    }

    /// Extract header token
    fn extract_header(
        &self,
        line: &str,
        line_num: u32,
        domain: &Arc<dyn DocumentDomain>,
    ) -> Option<SemanticTokenInfo> {
        static HEADER_RE: OnceLock<Regex> = OnceLock::new();
        let header_re = HEADER_RE.get_or_init(|| Regex::new(r"^(=+)\s+(.+)$").unwrap());

        if let Some(cap) = header_re.captures(line) {
            let title = cap.get(2)?.as_str();
            let start = cap.get(2)?.start() as u32;

            let token_type = domain
                .classify_element("header", title)
                .unwrap_or(SemanticTokenType::CLASS);

            return Some(SemanticTokenInfo {
                text: title.to_string(),
                token_type,
                line: line_num,
                start_char: start,
                length: title.len() as u32,
            });
        }

        None
    }

    /// Extract attribute tokens (name and value)
    fn extract_attribute(
        &self,
        line: &str,
        line_num: u32,
        domain: &Arc<dyn DocumentDomain>,
    ) -> Option<Vec<SemanticTokenInfo>> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let attr_re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w\-]+):\s*(.*)$").unwrap());

        if let Some(cap) = attr_re.captures(line) {
            let mut tokens = Vec::new();

            // Attribute name
            let name = cap.get(1)?.as_str();
            let name_start = cap.get(1)?.start() as u32;
            let name_type = domain
                .classify_element("attribute_name", name)
                .unwrap_or(SemanticTokenType::VARIABLE);

            tokens.push(SemanticTokenInfo {
                text: name.to_string(),
                token_type: name_type,
                line: line_num,
                start_char: name_start,
                length: name.len() as u32,
            });

            // Attribute value (if present)
            let value = cap.get(2)?.as_str().trim();
            if !value.is_empty() {
                let value_start = cap.get(2)?.start() as u32;
                let value_type = domain
                    .classify_element("attribute_value", value)
                    .unwrap_or(SemanticTokenType::STRING);

                tokens.push(SemanticTokenInfo {
                    text: value.to_string(),
                    token_type: value_type,
                    line: line_num,
                    start_char: value_start,
                    length: value.len() as u32,
                });
            }

            return Some(tokens);
        }

        None
    }

    /// Extract anchor token [[id]]
    fn extract_anchor(
        &self,
        line: &str,
        line_num: u32,
        domain: &Arc<dyn DocumentDomain>,
    ) -> Option<SemanticTokenInfo> {
        static ANCHOR_RE: OnceLock<Regex> = OnceLock::new();
        let anchor_re = ANCHOR_RE.get_or_init(|| Regex::new(r"^\[\[([\w\-]+)\]\]").unwrap());

        if let Some(cap) = anchor_re.captures(line) {
            let id = cap.get(1)?.as_str();
            let start = cap.get(1)?.start() as u32;

            let token_type = domain
                .classify_element("anchor", id)
                .unwrap_or(SemanticTokenType::VARIABLE);

            return Some(SemanticTokenInfo {
                text: id.to_string(),
                token_type,
                line: line_num,
                start_char: start,
                length: id.len() as u32,
            });
        }

        None
    }

    /// Convert tokens to LSP delta format
    pub fn to_lsp_tokens(&self, tokens: &[SemanticTokenInfo]) -> Vec<LspSemanticToken> {
        let mut lsp_tokens = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for token in tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_start
            } else {
                token.start_char
            };

            lsp_tokens.push(LspSemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: self.token_type_to_index(&token.token_type),
                token_modifiers: 0,
            });

            prev_line = token.line;
            prev_start = token.start_char;
        }

        lsp_tokens
    }

    /// Map semantic token type to legend index
    fn token_type_to_index(&self, token_type: &SemanticTokenType) -> u32 {
        // Standard LSP semantic token types
        match token_type.as_str() {
            "comment" => 0,
            "keyword" => 1,
            "string" => 2,
            "number" => 3,
            "regexp" => 4,
            "operator" => 5,
            "namespace" => 6,
            "type" => 7,
            "struct" => 8,
            "class" => 9,
            "interface" => 10,
            "enum" => 11,
            "enumMember" => 12,
            "typeParameter" => 13,
            "function" => 14,
            "method" => 15,
            "decorator" => 16,
            "macro" => 17,
            "variable" => 18,
            "parameter" => 19,
            "property" => 20,
            "label" => 21,
            _ => 18, // Default to variable
        }
    }

    /// Get the token type legend for LSP
    pub fn token_legend() -> Vec<SemanticTokenType> {
        vec![
            SemanticTokenType::COMMENT,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::CLASS,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::ENUM,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::DECORATOR,
            SemanticTokenType::MACRO,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::new("label"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_extraction() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = "= Document Title";
        let tokens = analyzer.analyze(text);

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "Document Title");
        assert_eq!(tokens[0].token_type, SemanticTokenType::CLASS);
    }

    #[test]
    fn test_attribute_extraction() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = ":author: John Doe";
        let tokens = analyzer.analyze(text);

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "author");
        assert_eq!(tokens[1].text, "John Doe");
    }

    #[test]
    fn test_delta_encoding() {
        let registry = DomainRegistry::default();
        let analyzer = SemanticAnalyzer::new(registry);

        let text = ":status: Draft\n:author: Team";
        let tokens = analyzer.analyze(text);
        let lsp_tokens = analyzer.to_lsp_tokens(&tokens);

        assert!(!lsp_tokens.is_empty());
        assert_eq!(lsp_tokens[0].delta_line, 0); // First token
    }
}
