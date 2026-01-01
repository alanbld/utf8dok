//! Layout mapping and slide layout management.
//!
//! This module handles the mapping between semantic slide types and
//! the actual slide layouts available in PPTX templates.

use crate::slide::SlideLayoutHint;
use crate::slide_contract::SlideContract;
use std::collections::HashMap;

/// Represents a slide layout from a PPTX template
#[derive(Debug, Clone)]
pub struct SlideLayout {
    /// Layout index (1-based, matching slideLayoutN.xml)
    pub index: u32,

    /// Layout name (from template)
    pub name: String,

    /// Layout type (e.g., "title", "obj", "twoObj")
    pub layout_type: LayoutType,

    /// Placeholders available in this layout
    pub placeholders: Vec<PlaceholderInfo>,
}

/// Standard layout types in PPTX
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    /// Title slide (ctrTitle)
    Title,

    /// Title and content (obj)
    TitleAndContent,

    /// Section header (secHead)
    SectionHeader,

    /// Two content (twoObj)
    TwoContent,

    /// Comparison (twoTxTwoObj)
    Comparison,

    /// Title only (titleOnly)
    TitleOnly,

    /// Blank
    Blank,

    /// Picture with caption (picTx)
    PictureWithCaption,

    /// Content with caption (objTx)
    ContentWithCaption,

    /// Custom layout
    Custom,
}

impl LayoutType {
    /// Get the OOXML type attribute value
    pub fn ooxml_type(&self) -> Option<&'static str> {
        match self {
            Self::Title => Some("ctrTitle"),
            Self::TitleAndContent => Some("obj"),
            Self::SectionHeader => Some("secHead"),
            Self::TwoContent => Some("twoObj"),
            Self::Comparison => Some("twoTxTwoObj"),
            Self::TitleOnly => Some("titleOnly"),
            Self::Blank => Some("blank"),
            Self::PictureWithCaption => Some("picTx"),
            Self::ContentWithCaption => Some("objTx"),
            Self::Custom => None,
        }
    }

    /// Parse from OOXML type attribute
    pub fn from_ooxml_type(s: &str) -> Self {
        match s {
            "ctrTitle" | "title" => Self::Title,
            "obj" => Self::TitleAndContent,
            "secHead" => Self::SectionHeader,
            "twoObj" => Self::TwoContent,
            "twoTxTwoObj" => Self::Comparison,
            "titleOnly" => Self::TitleOnly,
            "blank" => Self::Blank,
            "picTx" => Self::PictureWithCaption,
            "objTx" => Self::ContentWithCaption,
            _ => Self::Custom,
        }
    }
}

/// Information about a placeholder in a layout
#[derive(Debug, Clone)]
pub struct PlaceholderInfo {
    /// Placeholder index (idx attribute)
    pub index: u32,

    /// Placeholder type
    pub placeholder_type: PlaceholderType,

    /// Position (x, y) in EMU
    pub position: (i64, i64),

    /// Size (width, height) in EMU
    pub size: (i64, i64),
}

/// Types of placeholders
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderType {
    /// Title placeholder
    Title,

    /// Center title (for title slides)
    CenterTitle,

    /// Subtitle
    Subtitle,

    /// Body content
    Body,

    /// Object (content)
    Object,

    /// Date/time
    DateTime,

    /// Footer
    Footer,

    /// Slide number
    SlideNumber,

    /// Chart
    Chart,

    /// Table
    Table,

    /// Diagram/SmartArt
    Diagram,

    /// Media (video/audio)
    Media,

    /// Picture
    Picture,

    /// Other/custom
    Other,
}

impl PlaceholderType {
    /// Get the OOXML type attribute value
    pub fn ooxml_type(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::CenterTitle => "ctrTitle",
            Self::Subtitle => "subTitle",
            Self::Body => "body",
            Self::Object => "obj",
            Self::DateTime => "dt",
            Self::Footer => "ftr",
            Self::SlideNumber => "sldNum",
            Self::Chart => "chart",
            Self::Table => "tbl",
            Self::Diagram => "dgm",
            Self::Media => "media",
            Self::Picture => "pic",
            Self::Other => "",
        }
    }

    /// Parse from OOXML type attribute
    pub fn from_ooxml_type(s: &str) -> Self {
        match s {
            "title" => Self::Title,
            "ctrTitle" => Self::CenterTitle,
            "subTitle" => Self::Subtitle,
            "body" => Self::Body,
            "obj" => Self::Object,
            "dt" => Self::DateTime,
            "ftr" => Self::Footer,
            "sldNum" => Self::SlideNumber,
            "chart" => Self::Chart,
            "tbl" => Self::Table,
            "dgm" => Self::Diagram,
            "media" => Self::Media,
            "pic" => Self::Picture,
            _ => Self::Other,
        }
    }
}

/// Maps between semantic slide types and template layouts
#[derive(Debug, Clone)]
pub struct LayoutMapping {
    /// Available layouts from template
    layouts: Vec<SlideLayout>,

    /// Mapping from semantic type to layout index
    type_to_index: HashMap<String, u32>,

    /// Default layout index for unknown types
    default_layout: u32,
}

impl Default for LayoutMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutMapping {
    /// Create a new empty layout mapping
    pub fn new() -> Self {
        Self {
            layouts: Vec::new(),
            type_to_index: HashMap::new(),
            default_layout: 2, // Typically "Title and Content"
        }
    }

    /// Create layout mapping from a SlideContract
    pub fn from_contract(contract: &SlideContract) -> Self {
        let mut mapping = Self::new();

        // Set up standard type mappings from contract
        mapping
            .type_to_index
            .insert("title".to_string(), contract.layouts.title);
        mapping
            .type_to_index
            .insert("content".to_string(), contract.layouts.content);
        mapping
            .type_to_index
            .insert("section".to_string(), contract.layouts.section);
        mapping
            .type_to_index
            .insert("two_column".to_string(), contract.layouts.two_column);
        mapping
            .type_to_index
            .insert("comparison".to_string(), contract.layouts.comparison);
        mapping
            .type_to_index
            .insert("title_only".to_string(), contract.layouts.title_only);
        mapping
            .type_to_index
            .insert("blank".to_string(), contract.layouts.blank);
        mapping
            .type_to_index
            .insert("image".to_string(), contract.layouts.image);
        mapping
            .type_to_index
            .insert("quote".to_string(), contract.layouts.quote);

        // Add custom layouts
        for (name, index) in &contract.layouts.custom {
            mapping.type_to_index.insert(name.clone(), *index);
        }

        mapping.default_layout = contract.layouts.content;

        mapping
    }

    /// Add a layout to the mapping
    pub fn add_layout(&mut self, layout: SlideLayout) {
        self.layouts.push(layout);
    }

    /// Get the layout index for a semantic type
    pub fn get_layout_index(&self, semantic_type: &str) -> u32 {
        self.type_to_index
            .get(semantic_type)
            .copied()
            .unwrap_or(self.default_layout)
    }

    /// Get the layout index for a SlideLayoutHint
    pub fn get_layout_for_hint(&self, hint: SlideLayoutHint) -> u32 {
        let type_name = match hint {
            SlideLayoutHint::Title => "title",
            SlideLayoutHint::Section => "section",
            SlideLayoutHint::Content => "content",
            SlideLayoutHint::TwoColumn => "two_column",
            SlideLayoutHint::Comparison => "comparison",
            SlideLayoutHint::TitleOnly => "title_only",
            SlideLayoutHint::Blank => "blank",
            SlideLayoutHint::Image => "image",
            SlideLayoutHint::Quote => "quote",
        };

        self.get_layout_index(type_name)
    }

    /// Get a layout by index
    pub fn get_layout(&self, index: u32) -> Option<&SlideLayout> {
        self.layouts.iter().find(|l| l.index == index)
    }

    /// Get all layouts
    pub fn layouts(&self) -> &[SlideLayout] {
        &self.layouts
    }

    /// Get the number of layouts
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }

    /// Set the default layout index
    pub fn set_default(&mut self, index: u32) {
        self.default_layout = index;
    }
}

impl SlideLayout {
    /// Create a new slide layout
    pub fn new(index: u32, name: impl Into<String>, layout_type: LayoutType) -> Self {
        Self {
            index,
            name: name.into(),
            layout_type,
            placeholders: Vec::new(),
        }
    }

    /// Add a placeholder to this layout
    pub fn add_placeholder(&mut self, placeholder: PlaceholderInfo) {
        self.placeholders.push(placeholder);
    }

    /// Get the title placeholder if present
    pub fn title_placeholder(&self) -> Option<&PlaceholderInfo> {
        self.placeholders.iter().find(|p| {
            matches!(
                p.placeholder_type,
                PlaceholderType::Title | PlaceholderType::CenterTitle
            )
        })
    }

    /// Get the body/content placeholder if present
    pub fn body_placeholder(&self) -> Option<&PlaceholderInfo> {
        self.placeholders.iter().find(|p| {
            matches!(
                p.placeholder_type,
                PlaceholderType::Body | PlaceholderType::Object
            )
        })
    }

    /// Check if this layout has a specific placeholder type
    pub fn has_placeholder(&self, placeholder_type: PlaceholderType) -> bool {
        self.placeholders
            .iter()
            .any(|p| p.placeholder_type == placeholder_type)
    }
}

impl PlaceholderInfo {
    /// Create a new placeholder info
    pub fn new(
        index: u32,
        placeholder_type: PlaceholderType,
        position: (i64, i64),
        size: (i64, i64),
    ) -> Self {
        Self {
            index,
            placeholder_type,
            position,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_type_ooxml() {
        assert_eq!(LayoutType::Title.ooxml_type(), Some("ctrTitle"));
        assert_eq!(LayoutType::TitleAndContent.ooxml_type(), Some("obj"));
        assert_eq!(LayoutType::Custom.ooxml_type(), None);

        assert_eq!(LayoutType::from_ooxml_type("ctrTitle"), LayoutType::Title);
        assert_eq!(
            LayoutType::from_ooxml_type("obj"),
            LayoutType::TitleAndContent
        );
        assert_eq!(LayoutType::from_ooxml_type("unknown"), LayoutType::Custom);
    }

    #[test]
    fn test_placeholder_type_ooxml() {
        assert_eq!(PlaceholderType::Title.ooxml_type(), "title");
        assert_eq!(PlaceholderType::Body.ooxml_type(), "body");

        assert_eq!(
            PlaceholderType::from_ooxml_type("title"),
            PlaceholderType::Title
        );
        assert_eq!(
            PlaceholderType::from_ooxml_type("unknown"),
            PlaceholderType::Other
        );
    }

    #[test]
    fn test_layout_mapping_from_contract() {
        let contract = SlideContract::default();
        let mapping = LayoutMapping::from_contract(&contract);

        assert_eq!(mapping.get_layout_index("title"), 1);
        assert_eq!(mapping.get_layout_index("content"), 2);
        assert_eq!(mapping.get_layout_index("section"), 3);
        assert_eq!(mapping.get_layout_index("unknown"), 2); // default
    }

    #[test]
    fn test_layout_for_hint() {
        let mapping = LayoutMapping::from_contract(&SlideContract::default());

        assert_eq!(mapping.get_layout_for_hint(SlideLayoutHint::Title), 1);
        assert_eq!(mapping.get_layout_for_hint(SlideLayoutHint::Content), 2);
        assert_eq!(mapping.get_layout_for_hint(SlideLayoutHint::Quote), 9);
    }

    #[test]
    fn test_slide_layout() {
        let mut layout = SlideLayout::new(1, "Title Slide", LayoutType::Title);

        layout.add_placeholder(PlaceholderInfo::new(
            0,
            PlaceholderType::CenterTitle,
            (0, 0),
            (9144000, 1325563),
        ));
        layout.add_placeholder(PlaceholderInfo::new(
            1,
            PlaceholderType::Subtitle,
            (0, 1500000),
            (9144000, 500000),
        ));

        assert!(layout.title_placeholder().is_some());
        assert!(layout.has_placeholder(PlaceholderType::Subtitle));
        assert!(!layout.has_placeholder(PlaceholderType::Body));
    }

    #[test]
    fn test_layout_mapping_add_layout() {
        let mut mapping = LayoutMapping::new();

        mapping.add_layout(SlideLayout::new(1, "Title Slide", LayoutType::Title));
        mapping.add_layout(SlideLayout::new(
            2,
            "Title and Content",
            LayoutType::TitleAndContent,
        ));

        assert_eq!(mapping.layout_count(), 2);
        assert!(mapping.get_layout(1).is_some());
        assert!(mapping.get_layout(99).is_none());
    }
}
