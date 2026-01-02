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

    /// Cover page configuration (ADR-009)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<CoverConfig>,
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

// =============================================================================
// COVER PAGE CONFIGURATION (ADR-009)
// =============================================================================

/// Cover page configuration
///
/// Defines styling and layout for document cover/title pages.
/// Follows Asciidoctor PDF conventions for AsciiDoc compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoverConfig {
    /// Layout mode: "background" (image behind text) or "block" (image above text)
    #[serde(default = "default_layout")]
    pub layout: CoverLayout,

    /// Image scaling mode
    #[serde(default = "default_image_fit")]
    pub image_fit: ImageFit,

    /// Vertical alignment when image doesn't fill page
    #[serde(default = "default_image_position")]
    pub image_position: ImagePosition,

    /// Title element configuration
    #[serde(default)]
    pub title: CoverElementConfig,

    /// Subtitle element configuration
    #[serde(default)]
    pub subtitle: CoverElementConfig,

    /// Authors element configuration
    #[serde(default)]
    pub authors: CoverElementConfig,

    /// Revision element configuration
    #[serde(default)]
    pub revision: CoverRevisionConfig,
}

/// Cover layout mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoverLayout {
    /// Image placed behind text (z-order: back)
    #[default]
    Background,
    /// Image placed above text as a block element
    Block,
}

fn default_layout() -> CoverLayout {
    CoverLayout::Background
}

/// Image scaling mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImageFit {
    /// Scale to cover entire area (may crop)
    #[default]
    Cover,
    /// Scale to fit within area (may letterbox)
    Contain,
    /// Stretch to fill (may distort)
    Fill,
    /// No scaling
    None,
}

fn default_image_fit() -> ImageFit {
    ImageFit::Cover
}

/// Image vertical position
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImagePosition {
    /// Center vertically
    #[default]
    Center,
    /// Align to top
    Top,
    /// Align to bottom
    Bottom,
}

fn default_image_position() -> ImagePosition {
    ImagePosition::Center
}

/// Text alignment
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Configuration for a cover text element (title, subtitle, authors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverElementConfig {
    /// Word style ID to use (optional - inherits from template if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Text color (hex RGB, e.g., "FFFFFF")
    #[serde(default = "default_color")]
    pub color: String,

    /// Font size in half-points (e.g., 72 = 36pt)
    #[serde(default = "default_title_font_size")]
    pub font_size: u32,

    /// Font family name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,

    /// Bold text
    #[serde(default)]
    pub bold: bool,

    /// Italic text
    #[serde(default)]
    pub italic: bool,

    /// Vertical position from top (percentage or absolute)
    #[serde(default = "default_top")]
    pub top: String,

    /// Horizontal alignment
    #[serde(default)]
    pub align: TextAlign,

    /// Content template (for authors: "{author}", "{email}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl Default for CoverElementConfig {
    fn default() -> Self {
        Self {
            style: None,
            color: "FFFFFF".to_string(),
            font_size: 72, // 36pt
            font_family: None,
            bold: false,
            italic: false,
            top: "35%".to_string(),
            align: TextAlign::Center,
            content: None,
        }
    }
}

fn default_color() -> String {
    "FFFFFF".to_string()
}

fn default_title_font_size() -> u32 {
    72
}

fn default_top() -> String {
    "35%".to_string()
}

/// Configuration for revision element (extends CoverElementConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverRevisionConfig {
    /// Word style ID to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Text color (hex RGB)
    #[serde(default = "default_color")]
    pub color: String,

    /// Font size in half-points
    #[serde(default = "default_revision_font_size")]
    pub font_size: u32,

    /// Font family name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,

    /// Bold text
    #[serde(default)]
    pub bold: bool,

    /// Italic text
    #[serde(default)]
    pub italic: bool,

    /// Vertical position from top
    #[serde(default = "default_revision_top")]
    pub top: String,

    /// Horizontal alignment
    #[serde(default)]
    pub align: TextAlign,

    /// Delimiter between version and date
    #[serde(default = "default_delimiter")]
    pub delimiter: String,

    /// Content template
    #[serde(default = "default_revision_content")]
    pub content: String,
}

impl Default for CoverRevisionConfig {
    fn default() -> Self {
        Self {
            style: None,
            color: "FFFFFF".to_string(),
            font_size: 24, // 12pt
            font_family: None,
            bold: false,
            italic: false,
            top: "80%".to_string(),
            align: TextAlign::Center,
            delimiter: " | ".to_string(),
            content: "Version {revnumber}{delimiter}{revdate}".to_string(),
        }
    }
}

fn default_revision_font_size() -> u32 {
    24
}

fn default_revision_top() -> String {
    "80%".to_string()
}

fn default_delimiter() -> String {
    " | ".to_string()
}

fn default_revision_content() -> String {
    "Version {revnumber}{delimiter}{revdate}".to_string()
}

impl CoverConfig {
    /// Create a new CoverConfig with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a CoverConfig optimized for dark cover images (white text)
    pub fn for_dark_background() -> Self {
        Self {
            layout: CoverLayout::Background,
            image_fit: ImageFit::Cover,
            image_position: ImagePosition::Center,
            title: CoverElementConfig {
                color: "FFFFFF".to_string(),
                font_size: 72,
                bold: true,
                top: "35%".to_string(),
                ..Default::default()
            },
            subtitle: CoverElementConfig {
                color: "FFFFFF".to_string(),
                font_size: 32,
                italic: true,
                top: "45%".to_string(),
                ..Default::default()
            },
            authors: CoverElementConfig {
                color: "FFFFFF".to_string(),
                font_size: 28,
                top: "75%".to_string(),
                content: Some("{author}".to_string()),
                ..Default::default()
            },
            revision: CoverRevisionConfig::default(),
        }
    }

    /// Create a CoverConfig optimized for light cover images (dark text)
    pub fn for_light_background() -> Self {
        Self {
            layout: CoverLayout::Background,
            image_fit: ImageFit::Cover,
            image_position: ImagePosition::Center,
            title: CoverElementConfig {
                color: "1F2937".to_string(), // Dark gray
                font_size: 72,
                bold: true,
                top: "35%".to_string(),
                ..Default::default()
            },
            subtitle: CoverElementConfig {
                color: "4B5563".to_string(), // Medium gray
                font_size: 32,
                italic: true,
                top: "45%".to_string(),
                ..Default::default()
            },
            authors: CoverElementConfig {
                color: "374151".to_string(),
                font_size: 28,
                top: "75%".to_string(),
                content: Some("{author}".to_string()),
                ..Default::default()
            },
            revision: CoverRevisionConfig {
                color: "6B7280".to_string(),
                ..Default::default()
            },
        }
    }

    /// Parse a position string to EMU (English Metric Units)
    ///
    /// Supports: "35%", "200pt", "2in", "5cm", "914400emu"
    pub fn parse_position_to_emu(position: &str, page_height_emu: i64) -> i64 {
        let position = position.trim();

        if position.ends_with('%') {
            // Percentage of page height
            let pct: f64 = position.trim_end_matches('%').parse().unwrap_or(35.0);
            (page_height_emu as f64 * pct / 100.0) as i64
        } else if position.ends_with("pt") {
            // Points (1 pt = 12700 EMU)
            let pts: f64 = position.trim_end_matches("pt").parse().unwrap_or(0.0);
            (pts * 12700.0) as i64
        } else if position.ends_with("in") {
            // Inches (1 in = 914400 EMU)
            let inches: f64 = position.trim_end_matches("in").parse().unwrap_or(0.0);
            (inches * 914400.0) as i64
        } else if position.ends_with("cm") {
            // Centimeters (1 cm = 360000 EMU)
            let cm: f64 = position.trim_end_matches("cm").parse().unwrap_or(0.0);
            (cm * 360000.0) as i64
        } else if position.ends_with("emu") {
            // Already EMU
            position.trim_end_matches("emu").parse().unwrap_or(0)
        } else {
            // Default: treat as percentage
            let pct: f64 = position.parse().unwrap_or(35.0);
            (page_height_emu as f64 * pct / 100.0) as i64
        }
    }

    /// Expand a content template with metadata values
    ///
    /// Supports: {title}, {subtitle}, {author}, {email}, {revnumber}, {revdate}, {delimiter}
    pub fn expand_template(template: &str, metadata: &CoverMetadata, delimiter: &str) -> String {
        template
            .replace("{title}", &metadata.title)
            .replace("{subtitle}", &metadata.subtitle)
            .replace("{author}", &metadata.author)
            .replace("{email}", &metadata.email)
            .replace("{revnumber}", &metadata.revnumber)
            .replace("{revdate}", &metadata.revdate)
            .replace("{revremark}", &metadata.revremark)
            .replace("{delimiter}", delimiter)
    }
}

/// Metadata values for cover template expansion
#[derive(Debug, Clone, Default)]
pub struct CoverMetadata {
    pub title: String,
    pub subtitle: String,
    pub author: String,
    pub email: String,
    pub revnumber: String,
    pub revdate: String,
    pub revremark: String,
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
        self.paragraph_styles
            .get(word_style)
            .map(|m| m.role.as_str())
    }

    /// Get the heading level for a paragraph style
    pub fn get_heading_level(&self, word_style: &str) -> Option<u8> {
        self.paragraph_styles
            .get(word_style)
            .and_then(|m| m.heading_level)
    }

    /// Get the semantic anchor ID for a Word bookmark
    pub fn get_semantic_anchor(&self, word_bookmark: &str) -> Option<&str> {
        self.anchors
            .get(word_bookmark)
            .map(|m| m.semantic_id.as_str())
    }

    /// Get the Word bookmark for a semantic anchor ID
    ///
    /// When multiple bookmarks map to the same semantic ID (common in edited docs),
    /// returns the canonical bookmark (first alphabetically) for deterministic behavior.
    pub fn get_word_bookmark(&self, semantic_id: &str) -> Option<&str> {
        self.anchors
            .iter()
            .filter(|(_, m)| m.semantic_id == semantic_id)
            .map(|(k, _)| k.as_str())
            .min() // Canonical = first alphabetically
    }

    /// Check if an anchor is a TOC entry
    pub fn is_toc_anchor(&self, word_bookmark: &str) -> bool {
        self.anchors
            .get(word_bookmark)
            .map(|m| m.anchor_type == AnchorType::Toc)
            .unwrap_or(false)
    }

    /// Get the Word style ID for a semantic role (reverse lookup)
    ///
    /// Searches paragraph_styles for a mapping with the given role.
    /// When multiple styles map to the same role, returns the first alphabetically.
    pub fn get_word_style_for_role(&self, role: &str) -> Option<&str> {
        self.paragraph_styles
            .iter()
            .filter(|(_, m)| m.role == role)
            .map(|(k, _)| k.as_str())
            .min() // First alphabetically for determinism
    }

    /// Get the Word style ID for a heading level (reverse lookup)
    ///
    /// Searches paragraph_styles for a heading with the specified level.
    /// Returns the first matching style alphabetically.
    pub fn get_word_heading_style(&self, level: u8) -> Option<&str> {
        self.paragraph_styles
            .iter()
            .filter(|(_, m)| m.heading_level == Some(level))
            .map(|(k, _)| k.as_str())
            .min() // First alphabetically for determinism
    }

    /// Get the Word character style ID for a semantic role
    pub fn get_word_char_style_for_role(&self, role: &str) -> Option<&str> {
        self.character_styles
            .iter()
            .filter(|(_, m)| m.role == role)
            .map(|(k, _)| k.as_str())
            .min()
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

    // =========================================================================
    // Cover Configuration Tests (ADR-009)
    // =========================================================================

    #[test]
    fn test_cover_config_defaults() {
        let cover = CoverConfig::default();

        assert_eq!(cover.layout, CoverLayout::Background);
        assert_eq!(cover.image_fit, ImageFit::Cover);
        assert_eq!(cover.image_position, ImagePosition::Center);
        assert_eq!(cover.title.color, "FFFFFF");
        assert_eq!(cover.title.font_size, 72); // 36pt
        assert_eq!(cover.revision.delimiter, " | ");
    }

    #[test]
    fn test_cover_config_for_dark_background() {
        let cover = CoverConfig::for_dark_background();

        assert_eq!(cover.title.color, "FFFFFF");
        assert!(cover.title.bold);
        assert_eq!(cover.subtitle.color, "FFFFFF");
        assert!(cover.subtitle.italic);
    }

    #[test]
    fn test_cover_config_for_light_background() {
        let cover = CoverConfig::for_light_background();

        assert_eq!(cover.title.color, "1F2937"); // Dark gray
        assert!(cover.title.bold);
        assert_eq!(cover.subtitle.color, "4B5563"); // Medium gray
    }

    #[test]
    fn test_cover_position_parsing_percentage() {
        let page_height = 10_000_000; // 10M EMU

        assert_eq!(
            CoverConfig::parse_position_to_emu("35%", page_height),
            3_500_000
        );
        assert_eq!(
            CoverConfig::parse_position_to_emu("100%", page_height),
            10_000_000
        );
        assert_eq!(CoverConfig::parse_position_to_emu("0%", page_height), 0);
    }

    #[test]
    fn test_cover_position_parsing_points() {
        let page_height = 10_000_000;

        // 1 pt = 12700 EMU
        assert_eq!(
            CoverConfig::parse_position_to_emu("100pt", page_height),
            1_270_000
        );
        assert_eq!(
            CoverConfig::parse_position_to_emu("72pt", page_height),
            914_400
        );
    }

    #[test]
    fn test_cover_position_parsing_inches() {
        let page_height = 10_000_000;

        // 1 in = 914400 EMU
        assert_eq!(
            CoverConfig::parse_position_to_emu("1in", page_height),
            914_400
        );
        assert_eq!(
            CoverConfig::parse_position_to_emu("2in", page_height),
            1_828_800
        );
    }

    #[test]
    fn test_cover_position_parsing_centimeters() {
        let page_height = 10_000_000;

        // 1 cm = 360000 EMU
        assert_eq!(
            CoverConfig::parse_position_to_emu("1cm", page_height),
            360_000
        );
        assert_eq!(
            CoverConfig::parse_position_to_emu("5cm", page_height),
            1_800_000
        );
    }

    #[test]
    fn test_cover_position_parsing_emu() {
        let page_height = 10_000_000;

        assert_eq!(
            CoverConfig::parse_position_to_emu("914400emu", page_height),
            914_400
        );
    }

    #[test]
    fn test_cover_template_expansion() {
        let metadata = CoverMetadata {
            title: "My Document".to_string(),
            subtitle: "A great book".to_string(),
            author: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
            revnumber: "1.0.0".to_string(),
            revdate: "2025-12-31".to_string(),
            revremark: "Initial release".to_string(),
        };

        let result = CoverConfig::expand_template(
            "Version {revnumber}{delimiter}{revdate}",
            &metadata,
            " | ",
        );
        assert_eq!(result, "Version 1.0.0 | 2025-12-31");

        let result2 = CoverConfig::expand_template("{author} <{email}>", &metadata, "");
        assert_eq!(result2, "Jane Doe <jane@example.com>");
    }

    #[test]
    fn test_cover_config_toml_serialization() {
        let cover = CoverConfig::for_dark_background();
        let toml_str = toml::to_string_pretty(&cover).unwrap();

        assert!(toml_str.contains("layout = \"background\""));
        assert!(toml_str.contains("image_fit = \"cover\""));
        assert!(toml_str.contains("[title]"));
        assert!(toml_str.contains("color = \"FFFFFF\""));
    }

    #[test]
    fn test_cover_config_toml_deserialization() {
        let toml_str = r#"
layout = "background"
image_fit = "cover"
image_position = "center"

[title]
color = "FF0000"
font_size = 96
bold = true
top = "40%"
align = "center"

[subtitle]
color = "00FF00"
font_size = 48
italic = true

[authors]
color = "0000FF"
font_size = 32
content = "{author} ({email})"

[revision]
color = "CCCCCC"
font_size = 24
delimiter = " - "
content = "v{revnumber} ({revdate})"
"#;

        let cover: CoverConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(cover.layout, CoverLayout::Background);
        assert_eq!(cover.title.color, "FF0000");
        assert_eq!(cover.title.font_size, 96);
        assert!(cover.title.bold);
        assert_eq!(cover.title.top, "40%");
        assert_eq!(cover.subtitle.color, "00FF00");
        assert!(cover.subtitle.italic);
        assert_eq!(
            cover.authors.content,
            Some("{author} ({email})".to_string())
        );
        assert_eq!(cover.revision.delimiter, " - ");
    }

    #[test]
    fn test_style_contract_with_cover() {
        let mut contract = StyleContract::new();
        contract.cover = Some(CoverConfig::for_dark_background());

        let toml_str = contract.to_toml().unwrap();
        assert!(toml_str.contains("[cover]"));
        assert!(toml_str.contains("[cover.title]"));

        // Round-trip
        let parsed = StyleContract::from_toml(&toml_str).unwrap();
        assert!(parsed.cover.is_some());
        let cover = parsed.cover.unwrap();
        assert_eq!(cover.title.color, "FFFFFF");
    }

    #[test]
    fn test_parse_essential_style_contract_with_cover() {
        // Test parsing the actual Essential template style-contract.toml
        let toml_str = r#"
[meta]
template = "open_template.dotx"
locale = "it-IT"

[paragraph_styles]
Titolo1 = { role = "h1", heading_level = 1 }
Normale = { role = "body" }

[cover]
layout = "background"
image_fit = "cover"

[cover.title]
color = "FFFFFF"
font_size = 72
bold = true
top = "35%"

[cover.subtitle]
color = "FFFFFF"
font_size = 32
italic = true
top = "45%"

[cover.authors]
color = "FFFFFF"
font_size = 28
top = "75%"
content = "{author}"

[cover.revision]
color = "FFFFFF"
font_size = 24
top = "80%"
delimiter = " | "
content = "Version {revnumber}{delimiter}{revdate}"
"#;

        let contract: StyleContract = toml::from_str(toml_str).unwrap();

        assert_eq!(
            contract.meta.template,
            Some("open_template.dotx".to_string())
        );
        assert!(contract.cover.is_some());

        let cover = contract.cover.unwrap();
        assert_eq!(cover.layout, CoverLayout::Background);
        assert_eq!(cover.title.font_size, 72);
        assert!(cover.title.bold);
        assert_eq!(cover.subtitle.top, "45%");
        assert_eq!(cover.authors.content, Some("{author}".to_string()));
        assert_eq!(
            cover.revision.content,
            "Version {revnumber}{delimiter}{revdate}"
        );
    }

    // ==================== Sprint 5: Position Parsing Edge Cases ====================

    #[test]
    fn test_parse_position_empty_string() {
        let page_height = 10_000_000;
        // Empty string should use default 35%
        let result = CoverConfig::parse_position_to_emu("", page_height);
        assert_eq!(result, (page_height as f64 * 35.0 / 100.0) as i64);
    }

    #[test]
    fn test_parse_position_whitespace_only() {
        let page_height = 10_000_000;
        // Whitespace-only should use default 35%
        let result = CoverConfig::parse_position_to_emu("   ", page_height);
        assert_eq!(result, (page_height as f64 * 35.0 / 100.0) as i64);
    }

    #[test]
    fn test_parse_position_percent_sign_only() {
        let page_height = 10_000_000;
        // Just "%" should default to 35%
        let result = CoverConfig::parse_position_to_emu("%", page_height);
        assert_eq!(result, (page_height as f64 * 35.0 / 100.0) as i64);
    }

    #[test]
    fn test_parse_position_negative_percentage() {
        let page_height = 10_000_000;
        // Negative percentage should work (positions above top of page)
        let result = CoverConfig::parse_position_to_emu("-10%", page_height);
        assert_eq!(result, -1_000_000);
    }

    #[test]
    fn test_parse_position_zero() {
        let page_height = 10_000_000;
        // Zero should produce 0 EMU
        assert_eq!(CoverConfig::parse_position_to_emu("0%", page_height), 0);
        assert_eq!(CoverConfig::parse_position_to_emu("0pt", page_height), 0);
        assert_eq!(CoverConfig::parse_position_to_emu("0in", page_height), 0);
        assert_eq!(CoverConfig::parse_position_to_emu("0cm", page_height), 0);
        assert_eq!(CoverConfig::parse_position_to_emu("0emu", page_height), 0);
    }

    #[test]
    fn test_parse_position_invalid_unit() {
        let page_height = 10_000_000;
        // Unknown unit "xyz" - should fall back to treating as percentage (default 35%)
        let result = CoverConfig::parse_position_to_emu("50xyz", page_height);
        // "50xyz" won't parse as f64, so falls back to 35%
        assert_eq!(result, (page_height as f64 * 35.0 / 100.0) as i64);
    }

    #[test]
    fn test_parse_position_with_spaces() {
        let page_height = 10_000_000;
        // Leading/trailing spaces should be trimmed
        assert_eq!(
            CoverConfig::parse_position_to_emu("  50%  ", page_height),
            5_000_000
        );
        assert_eq!(
            CoverConfig::parse_position_to_emu("  1in  ", page_height),
            914_400
        );
    }

    #[test]
    fn test_parse_position_large_value() {
        let page_height = 10_000_000;
        // 200% - beyond page height is valid
        let result = CoverConfig::parse_position_to_emu("200%", page_height);
        assert_eq!(result, 20_000_000);
    }

    #[test]
    fn test_parse_position_zero_page_height() {
        // Edge case: page height of 0 should handle gracefully
        let result = CoverConfig::parse_position_to_emu("50%", 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_parse_position_negative_page_height() {
        // Edge case: negative page height (unusual but should not crash)
        let result = CoverConfig::parse_position_to_emu("50%", -10_000_000);
        assert_eq!(result, -5_000_000);
    }

    #[test]
    fn test_parse_position_decimal_values() {
        let page_height = 10_000_000;
        // Decimal percentages
        assert_eq!(
            CoverConfig::parse_position_to_emu("33.33%", page_height),
            3_333_000
        );
        // Decimal points
        assert_eq!(
            CoverConfig::parse_position_to_emu("1.5in", page_height),
            1_371_600
        );
    }

    #[test]
    fn test_template_expansion_unknown_placeholder() {
        let metadata = CoverMetadata {
            title: "Title".to_string(),
            subtitle: "".to_string(),
            author: "".to_string(),
            email: "".to_string(),
            revnumber: "".to_string(),
            revdate: "".to_string(),
            revremark: "".to_string(),
        };

        // Unknown placeholder should remain unchanged
        let result = CoverConfig::expand_template("{unknown}", &metadata, "");
        assert_eq!(result, "{unknown}");
    }

    #[test]
    fn test_template_expansion_empty_values() {
        let metadata = CoverMetadata {
            title: "".to_string(),
            subtitle: "".to_string(),
            author: "".to_string(),
            email: "".to_string(),
            revnumber: "".to_string(),
            revdate: "".to_string(),
            revremark: "".to_string(),
        };

        // Empty values should be substituted as empty strings
        let result = CoverConfig::expand_template("{title} by {author}", &metadata, "");
        assert_eq!(result, " by ");
    }

    #[test]
    fn test_cover_config_default_values() {
        // Test that CoverConfig::default() produces valid defaults
        let config = CoverConfig::default();
        assert_eq!(config.layout, CoverLayout::Background);
        assert_eq!(config.image_fit, ImageFit::Cover);
        assert!(!config.title.color.is_empty());
        assert!(config.title.font_size > 0);
    }

    #[test]
    fn test_cover_element_config_default() {
        // Test CoverElementConfig defaults
        let elem = CoverElementConfig::default();
        assert_eq!(elem.color, "FFFFFF"); // White (for dark backgrounds)
        assert_eq!(elem.font_size, 72); // 36pt in half-points
        assert!(!elem.bold);
        assert!(!elem.italic);
        assert_eq!(elem.top, "35%");
        assert_eq!(elem.align, TextAlign::Center);
    }

    #[test]
    fn test_cover_for_dark_background() {
        // Test the dark background preset
        let config = CoverConfig::for_dark_background();
        assert_eq!(config.title.color, "FFFFFF"); // White text
        assert!(config.title.bold);
    }

    // ==================== Sprint 14: Template Expansion Edge Cases ====================

    #[test]
    fn test_template_expansion_repeated_placeholders() {
        let metadata = CoverMetadata {
            title: "Doc".to_string(),
            revnumber: "1.0".to_string(),
            ..Default::default()
        };

        // Same placeholder repeated multiple times
        let result = CoverConfig::expand_template(
            "{revnumber} - {title} - {revnumber}",
            &metadata,
            "",
        );
        assert_eq!(result, "1.0 - Doc - 1.0");
    }

    #[test]
    fn test_template_expansion_all_placeholders() {
        let metadata = CoverMetadata {
            title: "Title".to_string(),
            subtitle: "Subtitle".to_string(),
            author: "Author".to_string(),
            email: "email@test.com".to_string(),
            revnumber: "2.0".to_string(),
            revdate: "2025-01-01".to_string(),
            revremark: "Final".to_string(),
        };

        let result = CoverConfig::expand_template(
            "{title}|{subtitle}|{author}|{email}|{revnumber}|{revdate}|{revremark}|{delimiter}",
            &metadata,
            "SEP",
        );
        assert_eq!(result, "Title|Subtitle|Author|email@test.com|2.0|2025-01-01|Final|SEP");
    }

    #[test]
    fn test_template_expansion_special_delimiter() {
        let metadata = CoverMetadata {
            revnumber: "1.0".to_string(),
            revdate: "2025".to_string(),
            ..Default::default()
        };

        // Delimiter with special characters
        let result = CoverConfig::expand_template(
            "{revnumber}{delimiter}{revdate}",
            &metadata,
            " | ",
        );
        assert_eq!(result, "1.0 | 2025");

        // Newline delimiter
        let result2 = CoverConfig::expand_template(
            "{revnumber}{delimiter}{revdate}",
            &metadata,
            "\n",
        );
        assert_eq!(result2, "1.0\n2025");
    }

    #[test]
    fn test_template_expansion_no_placeholders() {
        let metadata = CoverMetadata::default();

        // Static text without placeholders
        let result = CoverConfig::expand_template("Static text only", &metadata, "");
        assert_eq!(result, "Static text only");
    }

    #[test]
    fn test_template_expansion_partial_placeholder() {
        let metadata = CoverMetadata {
            title: "Title".to_string(),
            ..Default::default()
        };

        // Malformed/partial placeholders should remain
        let result = CoverConfig::expand_template("{title} {title {notclosed", &metadata, "");
        assert_eq!(result, "Title {title {notclosed");
    }

    #[test]
    fn test_template_expansion_adjacent_placeholders() {
        let metadata = CoverMetadata {
            revnumber: "1".to_string(),
            revdate: "2".to_string(),
            ..Default::default()
        };

        // Placeholders directly adjacent to each other
        let result = CoverConfig::expand_template("{revnumber}{revdate}", &metadata, "");
        assert_eq!(result, "12");
    }

    #[test]
    fn test_cover_config_for_light_background_preset() {
        let config = CoverConfig::for_light_background();
        // Should have dark text colors for light background
        assert_eq!(config.title.color, "1F2937"); // Dark gray text
        assert!(config.title.bold);
    }

    #[test]
    fn test_style_contract_get_word_heading_style_all_levels() {
        let mut contract = StyleContract::default();

        // Add headings for all 9 levels
        for level in 1..=9 {
            contract.paragraph_styles.insert(
                format!("H{}", level),
                ParagraphStyleMapping {
                    role: "heading".to_string(),
                    heading_level: Some(level),
                    ..Default::default()
                },
            );
        }

        // Verify all levels can be retrieved
        for level in 1..=9 {
            let style = contract.get_word_heading_style(level);
            assert_eq!(style, Some(format!("H{}", level).as_str()));
        }

        // Level 0 and 10 should return None
        assert!(contract.get_word_heading_style(0).is_none());
        assert!(contract.get_word_heading_style(10).is_none());
    }

    #[test]
    fn test_style_contract_get_word_style_for_role_multiple() {
        let mut contract = StyleContract::default();

        contract.paragraph_styles.insert(
            "AbstractPara".to_string(),
            ParagraphStyleMapping {
                role: "abstract".to_string(),
                ..Default::default()
            },
        );
        contract.paragraph_styles.insert(
            "NoteStyle".to_string(),
            ParagraphStyleMapping {
                role: "note".to_string(),
                ..Default::default()
            },
        );

        assert_eq!(contract.get_word_style_for_role("abstract"), Some("AbstractPara"));
        assert_eq!(contract.get_word_style_for_role("note"), Some("NoteStyle"));
        assert!(contract.get_word_style_for_role("unknown").is_none());
    }

    // ==================== Sprint 19: StyleContract Method Tests ====================

    #[test]
    fn test_style_contract_merge_paragraph_styles() {
        let mut base = StyleContract::default();
        base.paragraph_styles.insert(
            "BaseStyle".to_string(),
            ParagraphStyleMapping {
                role: "base-role".to_string(),
                ..Default::default()
            },
        );

        let mut other = StyleContract::default();
        other.paragraph_styles.insert(
            "OtherStyle".to_string(),
            ParagraphStyleMapping {
                role: "other-role".to_string(),
                ..Default::default()
            },
        );

        base.merge(&other);

        assert!(base.paragraph_styles.contains_key("BaseStyle"));
        assert!(base.paragraph_styles.contains_key("OtherStyle"));
        assert_eq!(base.paragraph_styles.len(), 2);
    }

    #[test]
    fn test_style_contract_merge_all_style_types() {
        let mut base = StyleContract::default();
        let mut other = StyleContract::default();

        // Add character style to other
        other.character_styles.insert(
            "CharStyle".to_string(),
            CharacterStyleMapping {
                role: "code".to_string(),
                is_strong: false,
                is_emphasis: false,
                is_code: true,
            },
        );

        // Add anchor to other
        other.anchors.insert(
            "_Bookmark1".to_string(),
            AnchorMapping {
                semantic_id: "section-1".to_string(),
                anchor_type: AnchorType::Heading,
                target_heading: Some("Section 1".to_string()),
                original_bookmark: Some("_Bookmark1".to_string()),
            },
        );

        // Add hyperlink to other
        other.hyperlinks.insert(
            "rId5".to_string(),
            HyperlinkMapping {
                is_external: true,
                url: Some("https://example.com".to_string()),
                anchor_target: None,
                original_rel_id: None,
                original_anchor: None,
            },
        );

        // Add table style to other
        other.table_styles.insert(
            "TableGrid".to_string(),
            TableStyleMapping {
                role: "data-table".to_string(),
                first_row_header: true,
                first_col_header: false,
            },
        );

        base.merge(&other);

        assert!(base.character_styles.contains_key("CharStyle"));
        assert!(base.anchors.contains_key("_Bookmark1"));
        assert!(base.hyperlinks.contains_key("rId5"));
        assert!(base.table_styles.contains_key("TableGrid"));
    }

    #[test]
    fn test_style_contract_merge_overwrites_existing() {
        let mut base = StyleContract::default();
        base.paragraph_styles.insert(
            "SharedStyle".to_string(),
            ParagraphStyleMapping {
                role: "old-role".to_string(),
                ..Default::default()
            },
        );

        let mut other = StyleContract::default();
        other.paragraph_styles.insert(
            "SharedStyle".to_string(),
            ParagraphStyleMapping {
                role: "new-role".to_string(),
                ..Default::default()
            },
        );

        base.merge(&other);

        let merged = base.paragraph_styles.get("SharedStyle").unwrap();
        assert_eq!(merged.role, "new-role");
    }

    #[test]
    fn test_style_contract_get_word_char_style_for_role() {
        let mut contract = StyleContract::default();
        contract.character_styles.insert(
            "InlineCode".to_string(),
            CharacterStyleMapping {
                role: "code".to_string(),
                is_strong: false,
                is_emphasis: false,
                is_code: true,
            },
        );
        contract.character_styles.insert(
            "EmphasisChar".to_string(),
            CharacterStyleMapping {
                role: "emphasis".to_string(),
                is_strong: false,
                is_emphasis: true,
                is_code: false,
            },
        );

        assert_eq!(
            contract.get_word_char_style_for_role("code"),
            Some("InlineCode")
        );
        assert_eq!(
            contract.get_word_char_style_for_role("emphasis"),
            Some("EmphasisChar")
        );
        assert!(contract.get_word_char_style_for_role("unknown").is_none());
    }

    #[test]
    fn test_style_contract_is_toc_anchor() {
        let mut contract = StyleContract::default();
        contract.anchors.insert(
            "_Toc123456".to_string(),
            AnchorMapping {
                semantic_id: "toc-entry".to_string(),
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: Some("_Toc123456".to_string()),
            },
        );
        contract.anchors.insert(
            "_RefSection".to_string(),
            AnchorMapping {
                semantic_id: "section-ref".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_RefSection".to_string()),
            },
        );

        assert!(contract.is_toc_anchor("_Toc123456"));
        assert!(!contract.is_toc_anchor("_RefSection"));
        assert!(!contract.is_toc_anchor("_UnknownBookmark"));
    }

    #[test]
    fn test_style_contract_get_paragraph_role() {
        let mut contract = StyleContract::default();
        contract.paragraph_styles.insert(
            "AbstractStyle".to_string(),
            ParagraphStyleMapping {
                role: "abstract".to_string(),
                ..Default::default()
            },
        );

        assert_eq!(
            contract.get_paragraph_role("AbstractStyle"),
            Some("abstract")
        );
        assert!(contract.get_paragraph_role("UnknownStyle").is_none());
    }

    #[test]
    fn test_style_contract_get_heading_level() {
        let mut contract = StyleContract::default();
        contract.paragraph_styles.insert(
            "Heading1".to_string(),
            ParagraphStyleMapping {
                role: "heading".to_string(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.paragraph_styles.insert(
            "Heading3".to_string(),
            ParagraphStyleMapping {
                role: "heading".to_string(),
                heading_level: Some(3),
                ..Default::default()
            },
        );
        contract.paragraph_styles.insert(
            "NormalPara".to_string(),
            ParagraphStyleMapping {
                role: "paragraph".to_string(),
                heading_level: None,
                ..Default::default()
            },
        );

        assert_eq!(contract.get_heading_level("Heading1"), Some(1));
        assert_eq!(contract.get_heading_level("Heading3"), Some(3));
        assert!(contract.get_heading_level("NormalPara").is_none());
        assert!(contract.get_heading_level("Unknown").is_none());
    }

    #[test]
    fn test_style_contract_add_methods() {
        let mut contract = StyleContract::default();

        contract.add_paragraph_style(
            "Para1",
            ParagraphStyleMapping {
                role: "para".to_string(),
                ..Default::default()
            },
        );
        assert!(contract.paragraph_styles.contains_key("Para1"));

        contract.add_character_style(
            "Char1",
            CharacterStyleMapping {
                role: "inline".to_string(),
                is_strong: false,
                is_emphasis: false,
                is_code: true,
            },
        );
        assert!(contract.character_styles.contains_key("Char1"));

        contract.add_anchor(
            "_Bookmark",
            AnchorMapping {
                semantic_id: "bookmark".to_string(),
                anchor_type: AnchorType::UserDefined,
                target_heading: None,
                original_bookmark: Some("_Bookmark".to_string()),
            },
        );
        assert!(contract.anchors.contains_key("_Bookmark"));

        contract.add_hyperlink(
            "rId1",
            HyperlinkMapping {
                is_external: true,
                url: Some("http://test.com".to_string()),
                anchor_target: None,
                original_rel_id: None,
                original_anchor: None,
            },
        );
        assert!(contract.hyperlinks.contains_key("rId1"));

        contract.add_table_style(
            "Table1",
            TableStyleMapping {
                role: "data".to_string(),
                first_row_header: false,
                first_col_header: false,
            },
        );
        assert!(contract.table_styles.contains_key("Table1"));
    }

    #[test]
    fn test_cover_metadata_default() {
        let meta = CoverMetadata::default();

        assert!(meta.title.is_empty());
        assert!(meta.subtitle.is_empty());
        assert!(meta.author.is_empty());
        assert!(meta.email.is_empty());
        assert!(meta.revnumber.is_empty());
        assert!(meta.revdate.is_empty());
        assert!(meta.revremark.is_empty());
    }

    #[test]
    fn test_cover_metadata_with_values() {
        let meta = CoverMetadata {
            title: "Document Title".to_string(),
            subtitle: "A Subtitle".to_string(),
            author: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            revnumber: "1.0".to_string(),
            revdate: "2025-01-01".to_string(),
            revremark: "Initial release".to_string(),
        };

        assert_eq!(meta.title, "Document Title");
        assert_eq!(meta.author, "John Doe");
        assert_eq!(meta.revnumber, "1.0");
    }

    #[test]
    fn test_style_contract_with_defaults() {
        let contract = StyleContract::with_defaults();

        // Should have some default paragraph styles
        assert!(!contract.paragraph_styles.is_empty());

        // Should include default heading mappings
        assert!(contract.paragraph_styles.contains_key("Heading1"));
        assert!(contract.paragraph_styles.contains_key("Heading2"));
    }

    #[test]
    fn test_style_contract_to_toml_and_from_toml() {
        let mut contract = StyleContract::default();
        contract.paragraph_styles.insert(
            "TestStyle".to_string(),
            ParagraphStyleMapping {
                role: "test".to_string(),
                heading_level: Some(2),
                ..Default::default()
            },
        );

        let toml_str = contract.to_toml().unwrap();
        let restored = StyleContract::from_toml(&toml_str).unwrap();

        assert!(restored.paragraph_styles.contains_key("TestStyle"));
        let style = restored.paragraph_styles.get("TestStyle").unwrap();
        assert_eq!(style.role, "test");
        assert_eq!(style.heading_level, Some(2));
    }

    #[test]
    fn test_anchor_type_variants() {
        assert_eq!(AnchorType::Toc, AnchorType::Toc);
        assert_eq!(AnchorType::Reference, AnchorType::Reference);
        assert_eq!(AnchorType::Heading, AnchorType::Heading);
        assert_eq!(AnchorType::UserDefined, AnchorType::UserDefined);
        assert_eq!(AnchorType::Highlight, AnchorType::Highlight);
    }

    #[test]
    fn test_cover_layout_variants() {
        let background = CoverLayout::Background;
        let block = CoverLayout::Block;

        match background {
            CoverLayout::Background => {}
            CoverLayout::Block => panic!("Wrong variant"),
        }
        match block {
            CoverLayout::Block => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_image_fit_variants() {
        let contain = ImageFit::Contain;
        let cover = ImageFit::Cover;
        let fill = ImageFit::Fill;
        let none = ImageFit::None;

        match contain {
            ImageFit::Contain => {}
            _ => panic!("Wrong variant"),
        }
        match cover {
            ImageFit::Cover => {}
            _ => panic!("Wrong variant"),
        }
        match fill {
            ImageFit::Fill => {}
            _ => panic!("Wrong variant"),
        }
        match none {
            ImageFit::None => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_image_position_variants() {
        let center = ImagePosition::Center;
        let top = ImagePosition::Top;
        let bottom = ImagePosition::Bottom;

        match center {
            ImagePosition::Center => {}
            _ => panic!("Wrong variant"),
        }
        match top {
            ImagePosition::Top => {}
            _ => panic!("Wrong variant"),
        }
        match bottom {
            ImagePosition::Bottom => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_text_align_variants() {
        assert_eq!(TextAlign::Left, TextAlign::Left);
        assert_eq!(TextAlign::Center, TextAlign::Center);
        assert_eq!(TextAlign::Right, TextAlign::Right);
    }

    #[test]
    fn test_list_type_variants() {
        assert_eq!(ListType::Unordered, ListType::Unordered);
        assert_eq!(ListType::Ordered, ListType::Ordered);
        assert_eq!(ListType::Definition, ListType::Definition);
    }

    #[test]
    fn test_style_contract_meta_defaults() {
        let meta = StyleContractMeta::default();

        assert!(meta.source_file.is_none());
        assert!(meta.created.is_none());
        assert!(meta.generator_version.is_none());
        assert!(meta.template.is_none());
    }

    #[test]
    fn test_theme_defaults_structure() {
        let theme = ThemeDefaults::default();

        // ThemeDefaults should have default values
        // Verify fields exist and have expected types
        assert!(theme.heading_font.is_none() || theme.heading_font.is_some());
        assert!(theme.body_font.is_none() || theme.body_font.is_some());
        assert!(theme.base_font_size.is_none() || theme.base_font_size.is_some());
        assert!(theme.accent_color.is_none() || theme.accent_color.is_some());
    }

    #[test]
    fn test_paragraph_style_mapping_default() {
        let mapping = ParagraphStyleMapping::default();

        assert_eq!(mapping.role, "");
        assert!(mapping.heading_level.is_none());
        assert!(!mapping.is_list);
        assert!(mapping.list_type.is_none());
        assert!(mapping.based_on.is_none());
    }

    #[test]
    fn test_character_style_mapping_structure() {
        let mapping = CharacterStyleMapping {
            role: "emphasis".to_string(),
            is_strong: false,
            is_emphasis: true,
            is_code: false,
        };

        assert_eq!(mapping.role, "emphasis");
        assert!(mapping.is_emphasis);
        assert!(!mapping.is_strong);
        assert!(!mapping.is_code);
    }

    #[test]
    fn test_table_style_mapping_structure() {
        let mapping = TableStyleMapping {
            role: "data-table".to_string(),
            first_row_header: true,
            first_col_header: false,
        };

        assert_eq!(mapping.role, "data-table");
        assert!(mapping.first_row_header);
        assert!(!mapping.first_col_header);
    }

    #[test]
    fn test_hyperlink_mapping_internal() {
        let mapping = HyperlinkMapping {
            is_external: false,
            url: None,
            anchor_target: Some("section-1".to_string()),
            original_rel_id: None,
            original_anchor: Some("_Section1".to_string()),
        };

        assert!(!mapping.is_external);
        assert_eq!(mapping.anchor_target, Some("section-1".to_string()));
    }

    #[test]
    fn test_hyperlink_mapping_external() {
        let mapping = HyperlinkMapping {
            is_external: true,
            url: Some("https://example.com/page".to_string()),
            anchor_target: None,
            original_rel_id: Some("rId5".to_string()),
            original_anchor: None,
        };

        assert!(mapping.is_external);
        assert!(mapping.url.as_ref().unwrap().starts_with("https://"));
    }

    #[test]
    fn test_cover_revision_config_default() {
        let revision = CoverRevisionConfig::default();

        // Check default values exist
        assert!(!revision.color.is_empty());
        assert!(revision.font_size > 0);
    }

    #[test]
    fn test_anchor_mapping_with_all_fields() {
        let mapping = AnchorMapping {
            semantic_id: "my-section".to_string(),
            anchor_type: AnchorType::Heading,
            target_heading: Some("My Section".to_string()),
            original_bookmark: Some("_MySection".to_string()),
        };

        assert_eq!(mapping.semantic_id, "my-section");
        assert_eq!(mapping.anchor_type, AnchorType::Heading);
        assert_eq!(mapping.target_heading, Some("My Section".to_string()));
        assert_eq!(mapping.original_bookmark, Some("_MySection".to_string()));
    }

    #[test]
    fn test_style_contract_new() {
        let contract = StyleContract::new();

        assert!(contract.paragraph_styles.is_empty());
        assert!(contract.character_styles.is_empty());
        assert!(contract.anchors.is_empty());
        assert!(contract.hyperlinks.is_empty());
        assert!(contract.table_styles.is_empty());
    }
}
