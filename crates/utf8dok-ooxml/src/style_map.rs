//! Style Contract Layer for DOCX ↔ AsciiDoc Round-Trip
//!
//! This module implements the style contract architecture described in ADR-007.
//! The StyleContract separates semantic content from presentation, enabling:
//!
//! - Deterministic round-trips
//! - Multi-format portability
//! - Corporate style compliance
//! - Transparent failure modes
//!
//! # Architecture
//!
//! ```text
//! DOCX → (Semantic AST + StyleContract) → AsciiDoc → (StyleContract) → DOCX
//! ```
//!
//! The StyleContract captures:
//! - Paragraph style mappings (w:pStyle → semantic roles)
//! - Character style mappings (w:rStyle → semantic roles)
//! - Anchor/bookmark registry with semantic normalization
//! - Table style mappings
//! - Theme defaults
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ooxml::style_map::StyleContract;
//!
//! let mut contract = StyleContract::new();
//!
//! // Map Word styles to semantic roles
//! contract.add_paragraph_style("Heading1", ParagraphStyleMapping {
//!     role: "h1".into(),
//!     heading_level: Some(1),
//!     ..Default::default()
//! });
//!
//! // Normalize anchors
//! contract.add_anchor("_Toc192197374", AnchorMapping {
//!     semantic_id: "introduction".into(),
//!     anchor_type: AnchorType::Heading,
//!     target_heading: Some("Introduction".into()),
//! });
//!
//! // Serialize for round-trip
//! let toml = contract.to_toml()?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

/// Style contract between DOCX and AsciiDoc
///
/// This is the central artifact that enables round-trip fidelity.
/// It must be serialized alongside the AsciiDoc output and loaded
/// during DOCX rendering.
///
/// Note: Named `StyleContract` to distinguish from `styles::StyleMap`
/// which handles rendering-time element-to-style mappings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StyleContract {
    /// Metadata about the source document
    #[serde(default)]
    pub meta: StyleContractMeta,

    /// Paragraph style mappings (Word style ID → semantic role)
    #[serde(default)]
    pub paragraph_styles: HashMap<String, ParagraphStyleMapping>,

    /// Character style mappings (Word style ID → semantic role)
    #[serde(default)]
    pub character_styles: HashMap<String, CharacterStyleMapping>,

    /// Anchor registry (Word bookmark → semantic anchor)
    #[serde(default)]
    pub anchors: HashMap<String, AnchorMapping>,

    /// Hyperlink registry (tracks intent for round-trip)
    #[serde(default)]
    pub hyperlinks: HashMap<String, HyperlinkMapping>,

    /// Table style mappings
    #[serde(default)]
    pub table_styles: HashMap<String, TableStyleMapping>,

    /// Theme defaults extracted from document
    #[serde(default)]
    pub theme: ThemeDefaults,
}

/// Metadata about the style contract source
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StyleContractMeta {
    /// Original source file name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,

    /// Creation timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// utf8dok version that created this map
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator_version: Option<String>,

    /// Template file used (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// Mapping for a paragraph style
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParagraphStyleMapping {
    /// Semantic role name (used in AsciiDoc)
    pub role: String,

    /// Heading level if this is a heading style (1-9)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<u8>,

    /// Whether this style represents a list item
    #[serde(default)]
    pub is_list: bool,

    /// List type if is_list is true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_type: Option<ListType>,

    /// Base style this inherits from (for reference)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub based_on: Option<String>,
}

/// Mapping for a character style
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterStyleMapping {
    /// Semantic role name
    pub role: String,

    /// Whether this represents strong/bold emphasis
    #[serde(default)]
    pub is_strong: bool,

    /// Whether this represents emphasis/italic
    #[serde(default)]
    pub is_emphasis: bool,

    /// Whether this represents monospace/code
    #[serde(default)]
    pub is_code: bool,
}

/// Mapping for an anchor/bookmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMapping {
    /// Semantic anchor ID (used in AsciiDoc as [[id]])
    pub semantic_id: String,

    /// Type of anchor (affects round-trip behavior)
    #[serde(default)]
    pub anchor_type: AnchorType,

    /// Target heading text (for heading anchors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_heading: Option<String>,

    /// Original Word bookmark name (for restoration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_bookmark: Option<String>,
}

/// Type of anchor for different handling strategies
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AnchorType {
    /// Table of contents entry (_Toc...)
    Toc,
    /// Cross-reference (_Ref...)
    Reference,
    /// Heading anchor (generated from heading text)
    Heading,
    /// User-defined bookmark
    #[default]
    UserDefined,
    /// Internal highlight (_Hlk...)
    Highlight,
}

/// Mapping for hyperlink intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlinkMapping {
    /// Whether this is an external link
    pub is_external: bool,

    /// External URL (if external)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Internal anchor target (if internal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_target: Option<String>,

    /// Original relationship ID (for restoration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_rel_id: Option<String>,

    /// Original Word anchor (for internal links)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_anchor: Option<String>,
}

/// Mapping for a table style
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableStyleMapping {
    /// Semantic role name
    pub role: String,

    /// Whether first row is header
    #[serde(default)]
    pub first_row_header: bool,

    /// Whether first column is header
    #[serde(default)]
    pub first_col_header: bool,
}

/// List type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ListType {
    /// Unordered/bullet list
    Unordered,
    /// Ordered/numbered list
    Ordered,
    /// Definition list
    Definition,
}

/// Theme defaults extracted from the document
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeDefaults {
    /// Major font family (headings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_font: Option<String>,

    /// Minor font family (body)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_font: Option<String>,

    /// Base font size in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_font_size: Option<u32>,

    /// Primary accent color (hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
}

impl StyleContract {
    /// Create a new empty StyleContract
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StyleContract with source metadata
    pub fn with_source(source_file: &str) -> Self {
        Self {
            meta: StyleContractMeta {
                source_file: Some(source_file.to_string()),
                created: Some(chrono_now()),
                generator_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                template: None,
            },
            ..Default::default()
        }
    }

    /// Add a paragraph style mapping
    pub fn add_paragraph_style(&mut self, word_style: &str, mapping: ParagraphStyleMapping) {
        self.paragraph_styles
            .insert(word_style.to_string(), mapping);
    }

    /// Add a character style mapping
    pub fn add_character_style(&mut self, word_style: &str, mapping: CharacterStyleMapping) {
        self.character_styles
            .insert(word_style.to_string(), mapping);
    }

    /// Add an anchor mapping
    pub fn add_anchor(&mut self, word_bookmark: &str, mapping: AnchorMapping) {
        self.anchors.insert(word_bookmark.to_string(), mapping);
    }

    /// Add a hyperlink mapping
    pub fn add_hyperlink(&mut self, id: &str, mapping: HyperlinkMapping) {
        self.hyperlinks.insert(id.to_string(), mapping);
    }

    /// Add a table style mapping
    pub fn add_table_style(&mut self, word_style: &str, mapping: TableStyleMapping) {
        self.table_styles.insert(word_style.to_string(), mapping);
    }

    /// Get the semantic role for a paragraph style
    pub fn get_paragraph_role(&self, word_style: &str) -> Option<&str> {
        self.paragraph_styles.get(word_style).map(|m| m.role.as_str())
    }

    /// Get the heading level for a paragraph style
    pub fn get_heading_level(&self, word_style: &str) -> Option<u8> {
        self.paragraph_styles
            .get(word_style)
            .and_then(|m| m.heading_level)
    }

    /// Get the semantic anchor ID for a Word bookmark
    pub fn get_semantic_anchor(&self, word_bookmark: &str) -> Option<&str> {
        self.anchors.get(word_bookmark).map(|m| m.semantic_id.as_str())
    }

    /// Get the Word bookmark for a semantic anchor ID
    pub fn get_word_bookmark(&self, semantic_id: &str) -> Option<&str> {
        self.anchors
            .iter()
            .find(|(_, m)| m.semantic_id == semantic_id)
            .map(|(k, _)| k.as_str())
    }

    /// Check if an anchor is a TOC entry
    pub fn is_toc_anchor(&self, word_bookmark: &str) -> bool {
        self.anchors
            .get(word_bookmark)
            .map(|m| m.anchor_type == AnchorType::Toc)
            .unwrap_or(false)
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| {
            crate::error::OoxmlError::Other(format!("Failed to serialize StyleContract: {}", e))
        })
    }

    /// Deserialize from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str).map_err(|e| {
            crate::error::OoxmlError::Other(format!("Failed to parse StyleContract: {}", e))
        })
    }

    /// Merge another StyleContract into this one (other takes precedence)
    pub fn merge(&mut self, other: &StyleContract) {
        for (k, v) in &other.paragraph_styles {
            self.paragraph_styles.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.character_styles {
            self.character_styles.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.anchors {
            self.anchors.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.hyperlinks {
            self.hyperlinks.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.table_styles {
            self.table_styles.insert(k.clone(), v.clone());
        }
    }

    /// Create default mappings for common Word styles
    pub fn with_defaults() -> Self {
        let mut map = Self::new();

        // Standard heading styles
        for level in 1..=9 {
            map.add_paragraph_style(
                &format!("Heading{}", level),
                ParagraphStyleMapping {
                    role: format!("h{}", level),
                    heading_level: Some(level),
                    ..Default::default()
                },
            );
            // Also lowercase variant
            map.add_paragraph_style(
                &format!("heading{}", level),
                ParagraphStyleMapping {
                    role: format!("h{}", level),
                    heading_level: Some(level),
                    ..Default::default()
                },
            );
        }

        // Common paragraph styles
        map.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "body".into(),
                ..Default::default()
            },
        );
        map.add_paragraph_style(
            "BodyText",
            ParagraphStyleMapping {
                role: "body".into(),
                ..Default::default()
            },
        );
        map.add_paragraph_style(
            "Quote",
            ParagraphStyleMapping {
                role: "quote".into(),
                ..Default::default()
            },
        );
        map.add_paragraph_style(
            "BlockText",
            ParagraphStyleMapping {
                role: "quote".into(),
                ..Default::default()
            },
        );

        // Character styles
        map.add_character_style(
            "Strong",
            CharacterStyleMapping {
                role: "strong".into(),
                is_strong: true,
                ..Default::default()
            },
        );
        map.add_character_style(
            "Emphasis",
            CharacterStyleMapping {
                role: "emphasis".into(),
                is_emphasis: true,
                ..Default::default()
            },
        );

        map
    }
}

/// Generate a simple timestamp (without chrono dependency)
fn chrono_now() -> String {
    // Simple ISO 8601 format without external dependency
    // In a real implementation, you might use chrono
    "2025-01-01T00:00:00Z".to_string()
}

/// Normalize a heading text to a semantic anchor ID
///
/// Converts "1.2 Purpose and Scope" → "purpose-and-scope"
pub fn normalize_heading_to_anchor(heading: &str) -> String {
    // Remove leading numbers and dots (e.g., "1.2.3 ")
    let without_numbers = heading
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
        .trim();

    // Convert to lowercase, replace spaces and non-alphanumeric chars with hyphens
    let normalized: String = without_numbers
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Remove consecutive hyphens and trim
    let mut result = String::new();
    let mut last_was_hyphen = false;
    for c in normalized.chars() {
        if c == '-' {
            if !last_was_hyphen && !result.is_empty() {
                result.push(c);
                last_was_hyphen = true;
            }
        } else {
            result.push(c);
            last_was_hyphen = false;
        }
    }

    result.trim_matches('-').to_string()
}

/// Classify a Word bookmark by its prefix
pub fn classify_bookmark(bookmark: &str) -> AnchorType {
    if bookmark.starts_with("_Toc") {
        AnchorType::Toc
    } else if bookmark.starts_with("_Ref") {
        AnchorType::Reference
    } else if bookmark.starts_with("_Hlk") {
        AnchorType::Highlight
    } else if bookmark.starts_with('_') {
        // Other internal bookmarks
        AnchorType::Highlight
    } else {
        AnchorType::UserDefined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_heading() {
        assert_eq!(normalize_heading_to_anchor("Introduction"), "introduction");
        assert_eq!(
            normalize_heading_to_anchor("1.2 Purpose and Scope"),
            "purpose-and-scope"
        );
        assert_eq!(
            normalize_heading_to_anchor("3.1.4 API Gateway Configuration"),
            "api-gateway-configuration"
        );
        assert_eq!(
            normalize_heading_to_anchor("  Multiple   Spaces  "),
            "multiple-spaces"
        );
    }

    #[test]
    fn test_classify_bookmark() {
        assert_eq!(classify_bookmark("_Toc123456"), AnchorType::Toc);
        assert_eq!(classify_bookmark("_Ref789012"), AnchorType::Reference);
        assert_eq!(classify_bookmark("_Hlk345678"), AnchorType::Highlight);
        assert_eq!(classify_bookmark("my_bookmark"), AnchorType::UserDefined);
        assert_eq!(classify_bookmark("custom"), AnchorType::UserDefined);
    }

    #[test]
    fn test_style_contract_serialization() {
        let mut contract = StyleContract::with_source("test.docx");
        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".into(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "introduction".into(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".into()),
                original_bookmark: Some("_Toc123".into()),
            },
        );

        let toml_str = contract.to_toml().unwrap();
        assert!(toml_str.contains("[paragraph_styles.Heading1]"));
        assert!(toml_str.contains("role = \"h1\""));
        assert!(toml_str.contains("[anchors._Toc123]"));
        assert!(toml_str.contains("semantic_id = \"introduction\""));

        // Round-trip
        let parsed = StyleContract::from_toml(&toml_str).unwrap();
        assert_eq!(parsed.get_heading_level("Heading1"), Some(1));
        assert_eq!(parsed.get_semantic_anchor("_Toc123"), Some("introduction"));
    }

    #[test]
    fn test_default_styles() {
        let contract = StyleContract::with_defaults();

        assert_eq!(contract.get_heading_level("Heading1"), Some(1));
        assert_eq!(contract.get_heading_level("Heading2"), Some(2));
        assert_eq!(contract.get_paragraph_role("Normal"), Some("body"));
        assert_eq!(contract.get_paragraph_role("Quote"), Some("quote"));
    }

    #[test]
    fn test_anchor_bidirectional_lookup() {
        let mut contract = StyleContract::new();
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "overview".into(),
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: None,
            },
        );

        assert_eq!(contract.get_semantic_anchor("_Toc123"), Some("overview"));
        assert_eq!(contract.get_word_bookmark("overview"), Some("_Toc123"));
    }
}
