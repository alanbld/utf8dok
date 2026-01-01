//! SlideContract configuration for PPTX generation.
//!
//! SlideContract defines the mapping between semantic slide types and
//! PowerPoint template layouts, similar to StyleContract for DOCX.

use crate::error::{PptxError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// SlideContract configuration for PPTX generation.
///
/// Maps semantic slide types to template layout indices and configures
/// various aspects of slide generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlideContract {
    /// Metadata about the contract and template
    pub meta: ContractMeta,

    /// Layout index mappings
    #[serde(default)]
    pub layouts: LayoutMappings,

    /// Placeholder index mappings
    #[serde(default)]
    pub placeholders: PlaceholderMappings,

    /// Speaker notes configuration
    #[serde(default)]
    pub notes: NotesConfig,

    /// Default layout assignments for content types
    #[serde(default)]
    pub defaults: DefaultLayouts,

    /// Slide transition settings
    #[serde(default)]
    pub transitions: TransitionConfig,

    /// Code block styling
    #[serde(default)]
    pub code: CodeConfig,

    /// Table styling
    #[serde(default)]
    pub table: TableConfig,
}

/// Contract metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMeta {
    /// Template file name
    #[serde(default)]
    pub template: String,

    /// Template display name
    #[serde(default)]
    pub template_name: String,

    /// Locale code (e.g., "en-US", "it-IT")
    #[serde(default = "default_locale")]
    pub locale: String,

    /// Contract version
    #[serde(default = "default_version")]
    pub version: String,

    /// Description
    #[serde(default)]
    pub description: String,
}

fn default_locale() -> String {
    "en-US".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Layout index mappings (1-based indices matching slideLayoutN.xml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutMappings {
    /// Title Slide layout (centered title)
    #[serde(default = "default_title_layout")]
    pub title: u32,

    /// Title and Content layout (most common)
    #[serde(default = "default_content_layout")]
    pub content: u32,

    /// Section Header layout
    #[serde(default = "default_section_layout")]
    pub section: u32,

    /// Two Content layout (side by side)
    #[serde(default = "default_two_column_layout")]
    pub two_column: u32,

    /// Comparison layout
    #[serde(default = "default_comparison_layout")]
    pub comparison: u32,

    /// Title Only layout
    #[serde(default = "default_title_only_layout")]
    pub title_only: u32,

    /// Blank layout
    #[serde(default = "default_blank_layout")]
    pub blank: u32,

    /// Picture with Caption layout
    #[serde(default = "default_image_layout")]
    pub image: u32,

    /// Quote layout
    #[serde(default = "default_quote_layout")]
    pub quote: u32,

    /// Additional custom layouts
    #[serde(flatten)]
    pub custom: HashMap<String, u32>,
}

fn default_title_layout() -> u32 {
    1
}
fn default_content_layout() -> u32 {
    2
}
fn default_section_layout() -> u32 {
    3
}
fn default_two_column_layout() -> u32 {
    4
}
fn default_comparison_layout() -> u32 {
    5
}
fn default_title_only_layout() -> u32 {
    6
}
fn default_blank_layout() -> u32 {
    7
}
fn default_image_layout() -> u32 {
    8
}
fn default_quote_layout() -> u32 {
    9
}

/// Placeholder index mappings within layouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceholderMappings {
    /// Title placeholder index
    #[serde(default)]
    pub title: u32,

    /// Subtitle placeholder index
    #[serde(default = "default_subtitle_ph")]
    pub subtitle: u32,

    /// Body/content placeholder index
    #[serde(default = "default_body_ph")]
    pub body: u32,

    /// Footer placeholder index
    #[serde(default = "default_footer_ph")]
    pub footer: u32,

    /// Slide number placeholder index
    #[serde(default = "default_slide_number_ph")]
    pub slide_number: u32,

    /// Date placeholder index
    #[serde(default = "default_date_ph")]
    pub date: u32,
}

fn default_subtitle_ph() -> u32 {
    1
}
fn default_body_ph() -> u32 {
    2
}
fn default_footer_ph() -> u32 {
    10
}
fn default_slide_number_ph() -> u32 {
    11
}
fn default_date_ph() -> u32 {
    12
}

/// Speaker notes configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesConfig {
    /// Enable speaker notes
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Font size in half-points
    #[serde(default = "default_notes_font_size")]
    pub font_size: u32,

    /// Font family
    #[serde(default = "default_notes_font")]
    pub font_family: String,

    /// Line spacing multiplier
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
}

fn default_true() -> bool {
    true
}
fn default_notes_font_size() -> u32 {
    24
}
fn default_notes_font() -> String {
    "Arial".to_string()
}
fn default_line_spacing() -> f32 {
    1.15
}

/// Default layout assignments for content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLayouts {
    /// Default for `== Heading` slides
    #[serde(default = "default_heading_layout")]
    pub heading_slide: String,

    /// Default for bullet list slides
    #[serde(default = "default_bullet_layout")]
    pub bullet_slide: String,

    /// Default for full-image slides
    #[serde(default = "default_image_slide")]
    pub image_slide: String,

    /// Default for table slides
    #[serde(default = "default_table_slide")]
    pub table_slide: String,

    /// Default for code block slides
    #[serde(default = "default_code_slide")]
    pub code_slide: String,

    /// Default for block quote slides
    #[serde(default = "default_quote_slide")]
    pub quote_slide: String,

    /// Default for `=== Subheading` slides
    #[serde(default = "default_section_break")]
    pub section_break: String,
}

fn default_heading_layout() -> String {
    "content".to_string()
}
fn default_bullet_layout() -> String {
    "content".to_string()
}
fn default_image_slide() -> String {
    "image".to_string()
}
fn default_table_slide() -> String {
    "content".to_string()
}
fn default_code_slide() -> String {
    "content".to_string()
}
fn default_quote_slide() -> String {
    "quote".to_string()
}
fn default_section_break() -> String {
    "section".to_string()
}

/// Slide transition settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionConfig {
    /// Default transition type
    #[serde(default = "default_transition")]
    pub default: TransitionType,

    /// Transition duration in milliseconds
    #[serde(default = "default_duration")]
    pub duration: u32,
}

fn default_transition() -> TransitionType {
    TransitionType::None
}
fn default_duration() -> u32 {
    500
}

/// Transition types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransitionType {
    None,
    Fade,
    Push,
    Wipe,
    Split,
    Cover,
    Uncover,
}

/// Code block styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeConfig {
    /// Font family for code
    #[serde(default = "default_code_font")]
    pub font_family: String,

    /// Font size in half-points
    #[serde(default = "default_code_font_size")]
    pub font_size: u32,

    /// Background color (hex RGB)
    #[serde(default = "default_code_bg")]
    pub background_color: String,

    /// Text color (hex RGB)
    #[serde(default = "default_code_text")]
    pub text_color: String,

    /// Show border
    #[serde(default = "default_true")]
    pub border: bool,

    /// Border color (hex RGB)
    #[serde(default = "default_code_border")]
    pub border_color: String,

    /// Padding
    #[serde(default = "default_code_padding")]
    pub padding: String,
}

fn default_code_font() -> String {
    "JetBrains Mono".to_string()
}
fn default_code_font_size() -> u32 {
    20
}
fn default_code_bg() -> String {
    "1E1E1E".to_string()
}
fn default_code_text() -> String {
    "D4D4D4".to_string()
}
fn default_code_border() -> String {
    "3C3C3C".to_string()
}
fn default_code_padding() -> String {
    "10pt".to_string()
}

/// Table styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    /// Header background color (hex RGB)
    #[serde(default = "default_table_header_bg")]
    pub header_background: String,

    /// Header text color (hex RGB)
    #[serde(default = "default_table_header_text")]
    pub header_text_color: String,

    /// Row background color (hex RGB)
    #[serde(default = "default_table_row_bg")]
    pub row_background: String,

    /// Alternating row background color (hex RGB)
    #[serde(default = "default_table_alt_row_bg")]
    pub alt_row_background: String,

    /// Border color (hex RGB)
    #[serde(default = "default_table_border")]
    pub border_color: String,

    /// Font size in half-points
    #[serde(default = "default_table_font_size")]
    pub font_size: u32,
}

fn default_table_header_bg() -> String {
    "2563EB".to_string()
}
fn default_table_header_text() -> String {
    "FFFFFF".to_string()
}
fn default_table_row_bg() -> String {
    "FFFFFF".to_string()
}
fn default_table_alt_row_bg() -> String {
    "F3F4F6".to_string()
}
fn default_table_border() -> String {
    "D1D5DB".to_string()
}
fn default_table_font_size() -> u32 {
    20
}

impl Default for ContractMeta {
    fn default() -> Self {
        Self {
            template: String::new(),
            template_name: "Default".to_string(),
            locale: default_locale(),
            version: default_version(),
            description: String::new(),
        }
    }
}

impl Default for LayoutMappings {
    fn default() -> Self {
        Self {
            title: default_title_layout(),
            content: default_content_layout(),
            section: default_section_layout(),
            two_column: default_two_column_layout(),
            comparison: default_comparison_layout(),
            title_only: default_title_only_layout(),
            blank: default_blank_layout(),
            image: default_image_layout(),
            quote: default_quote_layout(),
            custom: HashMap::new(),
        }
    }
}

impl Default for PlaceholderMappings {
    fn default() -> Self {
        Self {
            title: 0,
            subtitle: default_subtitle_ph(),
            body: default_body_ph(),
            footer: default_footer_ph(),
            slide_number: default_slide_number_ph(),
            date: default_date_ph(),
        }
    }
}

impl Default for NotesConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            font_size: default_notes_font_size(),
            font_family: default_notes_font(),
            line_spacing: default_line_spacing(),
        }
    }
}

impl Default for DefaultLayouts {
    fn default() -> Self {
        Self {
            heading_slide: default_heading_layout(),
            bullet_slide: default_bullet_layout(),
            image_slide: default_image_slide(),
            table_slide: default_table_slide(),
            code_slide: default_code_slide(),
            quote_slide: default_quote_slide(),
            section_break: default_section_break(),
        }
    }
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            default: default_transition(),
            duration: default_duration(),
        }
    }
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            font_family: default_code_font(),
            font_size: default_code_font_size(),
            background_color: default_code_bg(),
            text_color: default_code_text(),
            border: default_true(),
            border_color: default_code_border(),
            padding: default_code_padding(),
        }
    }
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            header_background: default_table_header_bg(),
            header_text_color: default_table_header_text(),
            row_background: default_table_row_bg(),
            alt_row_background: default_table_alt_row_bg(),
            border_color: default_table_border(),
            font_size: default_table_font_size(),
        }
    }
}

impl SlideContract {
    /// Load SlideContract from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// Parse SlideContract from a TOML string
    pub fn parse(toml_content: &str) -> Result<Self> {
        let contract: SlideContract = toml::from_str(toml_content)?;
        Ok(contract)
    }

    /// Get the layout index for a semantic layout type
    pub fn get_layout_index(&self, layout_type: &str) -> Option<u32> {
        match layout_type {
            "title" => Some(self.layouts.title),
            "content" => Some(self.layouts.content),
            "section" => Some(self.layouts.section),
            "two_column" => Some(self.layouts.two_column),
            "comparison" => Some(self.layouts.comparison),
            "title_only" => Some(self.layouts.title_only),
            "blank" => Some(self.layouts.blank),
            "image" => Some(self.layouts.image),
            "quote" => Some(self.layouts.quote),
            other => self.layouts.custom.get(other).copied(),
        }
    }

    /// Get the layout type for a given content type
    pub fn get_default_layout(&self, content_type: &str) -> &str {
        match content_type {
            "heading" => &self.defaults.heading_slide,
            "bullet" => &self.defaults.bullet_slide,
            "image" => &self.defaults.image_slide,
            "table" => &self.defaults.table_slide,
            "code" => &self.defaults.code_slide,
            "quote" => &self.defaults.quote_slide,
            "section" => &self.defaults.section_break,
            _ => "content",
        }
    }

    /// Validate the contract against a template
    pub fn validate(&self, max_layout_index: u32) -> Result<()> {
        // Check all layout indices are valid
        let indices = [
            ("title", self.layouts.title),
            ("content", self.layouts.content),
            ("section", self.layouts.section),
            ("two_column", self.layouts.two_column),
            ("comparison", self.layouts.comparison),
            ("title_only", self.layouts.title_only),
            ("blank", self.layouts.blank),
            ("image", self.layouts.image),
            ("quote", self.layouts.quote),
        ];

        for (name, index) in indices {
            if index < 1 || index > max_layout_index {
                return Err(PptxError::invalid_layout(
                    index,
                    format!(
                        "Layout '{}' index must be between 1 and {}",
                        name, max_layout_index
                    ),
                ));
            }
        }

        // Check custom layouts
        for (name, index) in &self.layouts.custom {
            if *index < 1 || *index > max_layout_index {
                return Err(PptxError::invalid_layout(
                    *index,
                    format!(
                        "Custom layout '{}' index must be between 1 and {}",
                        name, max_layout_index
                    ),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_slide_contract() {
        let contract = SlideContract::default();

        assert_eq!(contract.layouts.title, 1);
        assert_eq!(contract.layouts.content, 2);
        assert_eq!(contract.layouts.section, 3);
        assert_eq!(contract.meta.locale, "en-US");
        assert!(contract.notes.enabled);
    }

    #[test]
    fn test_parse_slide_contract() {
        let toml = r#"
[meta]
template = "corporate.potx"
template_name = "Corporate"
locale = "it-IT"

[layouts]
title = 1
content = 2
section = 5

[notes]
enabled = true
font_size = 28
font_family = "Calibri"

[defaults]
heading_slide = "content"
quote_slide = "title_only"
"#;

        let contract = SlideContract::parse(toml).unwrap();

        assert_eq!(contract.meta.template, "corporate.potx");
        assert_eq!(contract.meta.locale, "it-IT");
        assert_eq!(contract.layouts.section, 5);
        assert_eq!(contract.notes.font_size, 28);
        assert_eq!(contract.notes.font_family, "Calibri");
        assert_eq!(contract.defaults.quote_slide, "title_only");
    }

    #[test]
    fn test_get_layout_index() {
        let contract = SlideContract::default();

        assert_eq!(contract.get_layout_index("title"), Some(1));
        assert_eq!(contract.get_layout_index("content"), Some(2));
        assert_eq!(contract.get_layout_index("nonexistent"), None);
    }

    #[test]
    fn test_custom_layouts() {
        let toml = r#"
[meta]
template = "test.potx"

[layouts]
title = 1
content = 2
custom_agenda = 10
custom_summary = 11
"#;

        let contract = SlideContract::parse(toml).unwrap();

        assert_eq!(contract.get_layout_index("custom_agenda"), Some(10));
        assert_eq!(contract.get_layout_index("custom_summary"), Some(11));
    }

    #[test]
    fn test_get_default_layout() {
        let contract = SlideContract::default();

        assert_eq!(contract.get_default_layout("heading"), "content");
        assert_eq!(contract.get_default_layout("quote"), "quote");
        assert_eq!(contract.get_default_layout("image"), "image");
        assert_eq!(contract.get_default_layout("unknown"), "content");
    }

    #[test]
    fn test_validate_layout_indices() {
        let contract = SlideContract::default();

        // Valid with 10 layouts
        assert!(contract.validate(10).is_ok());

        // Invalid - layout index 9 (quote) exceeds max of 5
        assert!(contract.validate(5).is_err());
    }

    #[test]
    fn test_transition_types() {
        let toml = r#"
[meta]
template = "test.potx"

[transitions]
default = "fade"
duration = 750
"#;

        let contract = SlideContract::parse(toml).unwrap();

        assert_eq!(contract.transitions.default, TransitionType::Fade);
        assert_eq!(contract.transitions.duration, 750);
    }

    #[test]
    fn test_code_config() {
        let toml = r#"
[meta]
template = "test.potx"

[code]
font_family = "Fira Code"
font_size = 18
background_color = "282C34"
"#;

        let contract = SlideContract::parse(toml).unwrap();

        assert_eq!(contract.code.font_family, "Fira Code");
        assert_eq!(contract.code.font_size, 18);
        assert_eq!(contract.code.background_color, "282C34");
        // Defaults preserved
        assert!(contract.code.border);
    }

    #[test]
    fn test_table_config() {
        let contract = SlideContract::default();

        assert_eq!(contract.table.header_background, "2563EB");
        assert_eq!(contract.table.header_text_color, "FFFFFF");
        assert_eq!(contract.table.font_size, 20);
    }
}
