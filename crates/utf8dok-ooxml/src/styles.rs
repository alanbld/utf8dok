//! Style definitions parsing (word/styles.xml)
//!
//! This module parses Word style definitions and provides
//! utilities for understanding the style hierarchy.
//!
//! # Style Mapping
//!
//! The [`StyleMap`] struct provides a mapping between semantic document elements
//! (headings, paragraphs, tables) and Word style IDs, enabling template-aware
//! document generation.

use std::collections::HashMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{OoxmlError, Result};

/// Collection of styles from a document
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    /// All styles, keyed by style ID
    styles: HashMap<String, Style>,
    /// Default paragraph style ID
    pub default_paragraph: Option<String>,
    /// Default character style ID
    pub default_character: Option<String>,
}

/// A Word style definition
#[derive(Debug, Clone)]
pub struct Style {
    /// Style ID (used in document references)
    pub id: String,
    /// Display name
    pub name: String,
    /// Style type
    pub style_type: StyleType,
    /// Base style ID (for inheritance)
    pub based_on: Option<String>,
    /// Next style ID (for following paragraphs)
    pub next: Option<String>,
    /// Whether this is a built-in style
    pub builtin: bool,
    /// UI priority for sorting
    pub ui_priority: Option<u32>,
    /// Outline level (for headings, 0-8, where 0 = Heading 1)
    pub outline_level: Option<u8>,
}

/// Type of style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleType {
    /// Paragraph style
    Paragraph,
    /// Character (run) style
    Character,
    /// Table style
    Table,
    /// Numbering style
    Numbering,
}

impl StyleSheet {
    /// Parse styles from XML bytes
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut stylesheet = StyleSheet::default();
        let mut buf = Vec::new();
        let mut current_style: Option<StyleBuilder> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.local_name();
                    match name.as_ref() {
                        b"style" => {
                            let mut builder = StyleBuilder::default();

                            // Get style type
                            if let Some(t) = get_attr(e, b"w:type") {
                                builder.style_type = Some(match t.as_str() {
                                    "paragraph" => StyleType::Paragraph,
                                    "character" => StyleType::Character,
                                    "table" => StyleType::Table,
                                    "numbering" => StyleType::Numbering,
                                    _ => StyleType::Paragraph,
                                });
                            }

                            // Get style ID
                            if let Some(id) = get_attr(e, b"w:styleId") {
                                builder.id = Some(id);
                            }

                            // Check if default
                            if get_attr(e, b"w:default").as_deref() == Some("1") {
                                builder.is_default = true;
                            }

                            current_style = Some(builder);
                        }
                        b"name" if current_style.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                current_style.as_mut().unwrap().name = Some(val);
                            }
                        }
                        b"basedOn" if current_style.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                current_style.as_mut().unwrap().based_on = Some(val);
                            }
                        }
                        b"next" if current_style.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                current_style.as_mut().unwrap().next = Some(val);
                            }
                        }
                        b"uiPriority" if current_style.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(priority) = val.parse() {
                                    current_style.as_mut().unwrap().ui_priority = Some(priority);
                                }
                            }
                        }
                        b"outlineLvl" if current_style.is_some() => {
                            if let Some(val) = get_attr(e, b"w:val") {
                                if let Ok(level) = val.parse() {
                                    current_style.as_mut().unwrap().outline_level = Some(level);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.local_name().as_ref() == b"style" {
                        if let Some(builder) = current_style.take() {
                            let is_default = builder.is_default;
                            if let Some(style) = builder.build() {
                                // Track default styles
                                if is_default {
                                    match style.style_type {
                                        StyleType::Paragraph => {
                                            stylesheet.default_paragraph = Some(style.id.clone());
                                        }
                                        StyleType::Character => {
                                            stylesheet.default_character = Some(style.id.clone());
                                        }
                                        _ => {}
                                    }
                                }
                                stylesheet.styles.insert(style.id.clone(), style);
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(OoxmlError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(stylesheet)
    }

    /// Get a style by ID
    pub fn get(&self, id: &str) -> Option<&Style> {
        self.styles.get(id)
    }

    /// Get all styles
    pub fn all(&self) -> impl Iterator<Item = &Style> {
        self.styles.values()
    }

    /// Get all paragraph styles
    pub fn paragraph_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles
            .values()
            .filter(|s| s.style_type == StyleType::Paragraph)
    }

    /// Get all heading styles (styles with outline level)
    pub fn heading_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles.values().filter(|s| s.outline_level.is_some())
    }

    /// Get all table styles
    pub fn table_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles
            .values()
            .filter(|s| s.style_type == StyleType::Table)
    }

    /// Check if a style ID represents a heading
    pub fn is_heading(&self, style_id: &str) -> bool {
        self.get(style_id)
            .map(|s| s.outline_level.is_some())
            .unwrap_or(false)
    }

    /// Get the heading level (1-9) for a style, if it's a heading
    pub fn heading_level(&self, style_id: &str) -> Option<u8> {
        self.get(style_id)
            .and_then(|s| s.outline_level)
            .map(|l| l + 1) // Convert 0-based to 1-based
    }

    /// Resolve the full inheritance chain for a style
    pub fn resolve_chain(&self, style_id: &str) -> Vec<&Style> {
        let mut chain = Vec::new();
        let mut current = style_id;
        let mut seen = std::collections::HashSet::new();

        while let Some(style) = self.get(current) {
            if !seen.insert(&style.id) {
                break; // Avoid infinite loops
            }
            chain.push(style);
            if let Some(ref base) = style.based_on {
                current = base;
            } else {
                break;
            }
        }

        chain
    }
}

#[derive(Default)]
struct StyleBuilder {
    id: Option<String>,
    name: Option<String>,
    style_type: Option<StyleType>,
    based_on: Option<String>,
    next: Option<String>,
    ui_priority: Option<u32>,
    outline_level: Option<u8>,
    is_default: bool,
}

impl StyleBuilder {
    fn build(self) -> Option<Style> {
        let id = self.id?;
        Some(Style {
            id: id.clone(),
            name: self.name.unwrap_or(id),
            style_type: self.style_type.unwrap_or(StyleType::Paragraph),
            based_on: self.based_on,
            next: self.next,
            builtin: true, // TODO: detect custom styles
            ui_priority: self.ui_priority,
            outline_level: self.outline_level,
        })
    }
}

fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

// ============================================================================
// Style Mapping for Template Injection
// ============================================================================

/// Semantic element types that can be mapped to Word styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementType {
    /// Heading level 1-9
    Heading(u8),
    /// Normal paragraph
    Paragraph,
    /// Code/literal block
    CodeBlock,
    /// Unordered list item
    ListBullet,
    /// Ordered list item
    ListNumber,
    /// Description list item
    ListDescription,
    /// Table
    Table,
    /// Table header row
    TableHeader,
    /// Admonition: Note
    AdmonitionNote,
    /// Admonition: Tip
    AdmonitionTip,
    /// Admonition: Important
    AdmonitionImportant,
    /// Admonition: Warning
    AdmonitionWarning,
    /// Admonition: Caution
    AdmonitionCaution,
}

/// Maps semantic document elements to Word style IDs
///
/// This struct provides the bridge between AST elements and template styles,
/// enabling template-aware document generation.
///
/// # Example
///
/// ```
/// use utf8dok_ooxml::styles::{StyleMap, ElementType};
///
/// let mut map = StyleMap::default();
/// map.set(ElementType::Heading(1), "CorporateHeading1");
/// map.set(ElementType::Paragraph, "BodyText");
///
/// assert_eq!(map.get(ElementType::Heading(1)), "CorporateHeading1");
/// assert_eq!(map.get(ElementType::Paragraph), "BodyText");
/// ```
#[derive(Debug, Clone)]
pub struct StyleMap {
    /// Mapping from element type to style ID
    mappings: HashMap<ElementType, String>,
}

impl Default for StyleMap {
    /// Create a StyleMap with sensible defaults for standard Word templates
    fn default() -> Self {
        let mut mappings = HashMap::new();

        // Heading styles (Word convention)
        mappings.insert(ElementType::Heading(1), "Heading1".to_string());
        mappings.insert(ElementType::Heading(2), "Heading2".to_string());
        mappings.insert(ElementType::Heading(3), "Heading3".to_string());
        mappings.insert(ElementType::Heading(4), "Heading4".to_string());
        mappings.insert(ElementType::Heading(5), "Heading5".to_string());
        mappings.insert(ElementType::Heading(6), "Heading6".to_string());
        mappings.insert(ElementType::Heading(7), "Heading7".to_string());
        mappings.insert(ElementType::Heading(8), "Heading8".to_string());
        mappings.insert(ElementType::Heading(9), "Heading9".to_string());

        // Paragraph styles
        mappings.insert(ElementType::Paragraph, "Normal".to_string());
        mappings.insert(ElementType::CodeBlock, "CodeBlock".to_string());

        // List styles
        mappings.insert(ElementType::ListBullet, "ListBullet".to_string());
        mappings.insert(ElementType::ListNumber, "ListNumber".to_string());
        mappings.insert(ElementType::ListDescription, "ListParagraph".to_string());

        // Table styles
        mappings.insert(ElementType::Table, "TableGrid".to_string());
        mappings.insert(ElementType::TableHeader, "TableGrid".to_string());

        // Admonition styles (may not exist in all templates)
        mappings.insert(ElementType::AdmonitionNote, "Note".to_string());
        mappings.insert(ElementType::AdmonitionTip, "Tip".to_string());
        mappings.insert(ElementType::AdmonitionImportant, "Important".to_string());
        mappings.insert(ElementType::AdmonitionWarning, "Warning".to_string());
        mappings.insert(ElementType::AdmonitionCaution, "Caution".to_string());

        Self { mappings }
    }
}

impl StyleMap {
    /// Create an empty style map
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Set a mapping from element type to style ID
    pub fn set(&mut self, element: ElementType, style_id: impl Into<String>) {
        self.mappings.insert(element, style_id.into());
    }

    /// Get the style ID for an element type
    ///
    /// Returns the mapped style ID, or a sensible fallback if not mapped.
    pub fn get(&self, element: ElementType) -> &str {
        self.mappings
            .get(&element)
            .map(|s| s.as_str())
            .unwrap_or_else(|| Self::fallback_style(element))
    }

    /// Get style ID for a heading level (1-9)
    pub fn heading(&self, level: u8) -> &str {
        let level = level.clamp(1, 9);
        self.get(ElementType::Heading(level))
    }

    /// Get style ID for paragraphs
    pub fn paragraph(&self) -> &str {
        self.get(ElementType::Paragraph)
    }

    /// Get style ID for code blocks
    pub fn code_block(&self) -> &str {
        self.get(ElementType::CodeBlock)
    }

    /// Get style ID for tables
    pub fn table(&self) -> &str {
        self.get(ElementType::Table)
    }

    /// Get style ID for list items
    pub fn list(&self, ordered: bool) -> &str {
        if ordered {
            self.get(ElementType::ListNumber)
        } else {
            self.get(ElementType::ListBullet)
        }
    }

    /// Provide fallback styles for unmapped elements
    fn fallback_style(element: ElementType) -> &'static str {
        match element {
            ElementType::Heading(_) => "Heading1",
            ElementType::Paragraph => "Normal",
            ElementType::CodeBlock => "Normal",
            ElementType::ListBullet => "ListBullet",
            ElementType::ListNumber => "ListNumber",
            ElementType::ListDescription => "Normal",
            ElementType::Table => "TableGrid",
            ElementType::TableHeader => "TableGrid",
            ElementType::AdmonitionNote
            | ElementType::AdmonitionTip
            | ElementType::AdmonitionImportant
            | ElementType::AdmonitionWarning
            | ElementType::AdmonitionCaution => "Normal",
        }
    }

    /// Create a StyleMap from a StyleSheet by auto-detecting available styles
    ///
    /// This inspects the template's styles and maps to the best available match.
    pub fn from_stylesheet(stylesheet: &StyleSheet) -> Self {
        let mut map = StyleMap::default();

        // Check for alternative heading styles
        for level in 1..=9 {
            let default_id = format!("Heading{}", level);
            if stylesheet.get(&default_id).is_none() {
                // Try alternative naming conventions
                let alternatives = [
                    format!("heading {}", level),
                    format!("Heading {}", level),
                    format!("H{}", level),
                ];
                for alt in alternatives {
                    if stylesheet.get(&alt).is_some() {
                        map.set(ElementType::Heading(level), alt);
                        break;
                    }
                }
            }
        }

        // Check for code block style alternatives
        let code_alternatives = ["CodeBlock", "Code", "NoSpacing", "SourceCode", "Verbatim"];
        for alt in code_alternatives {
            if stylesheet.get(alt).is_some() {
                map.set(ElementType::CodeBlock, alt);
                break;
            }
        }

        // Check for table style alternatives
        let table_alternatives = ["TableGrid", "Table Grid", "GridTable1Light", "PlainTable1"];
        for alt in table_alternatives {
            if stylesheet.get(alt).is_some() {
                map.set(ElementType::Table, alt);
                break;
            }
        }

        map
    }

    /// Validate that all mapped styles exist in the stylesheet
    ///
    /// Returns a list of missing style IDs.
    pub fn validate(&self, stylesheet: &StyleSheet) -> Vec<String> {
        self.mappings
            .values()
            .filter(|style_id| stylesheet.get(style_id).is_none())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_map_default() {
        let map = StyleMap::default();
        assert_eq!(map.heading(1), "Heading1");
        assert_eq!(map.heading(2), "Heading2");
        assert_eq!(map.paragraph(), "Normal");
        assert_eq!(map.code_block(), "CodeBlock");
        assert_eq!(map.table(), "TableGrid");
        assert_eq!(map.list(true), "ListNumber");
        assert_eq!(map.list(false), "ListBullet");
    }

    #[test]
    fn test_style_map_custom() {
        let mut map = StyleMap::new();
        map.set(ElementType::Heading(1), "CorporateH1");
        map.set(ElementType::Paragraph, "BodyText");

        assert_eq!(map.heading(1), "CorporateH1");
        assert_eq!(map.paragraph(), "BodyText");
        // Fallback for unmapped
        assert_eq!(map.heading(2), "Heading1"); // Falls back to default
    }

    #[test]
    fn test_style_map_heading_clamp() {
        let map = StyleMap::default();
        // Level 0 should clamp to 1
        assert_eq!(map.heading(0), "Heading1");
        // Level 10 should clamp to 9
        assert_eq!(map.heading(10), "Heading9");
    }

    #[test]
    fn test_parse_heading_style() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:style w:type="paragraph" w:styleId="Heading1">
                <w:name w:val="heading 1"/>
                <w:basedOn w:val="Normal"/>
                <w:next w:val="Normal"/>
                <w:pPr>
                    <w:outlineLvl w:val="0"/>
                </w:pPr>
            </w:style>
        </w:styles>"#;

        let styles = StyleSheet::parse(xml).unwrap();
        let h1 = styles.get("Heading1").unwrap();

        assert_eq!(h1.name, "heading 1");
        assert_eq!(h1.outline_level, Some(0));
        assert_eq!(styles.heading_level("Heading1"), Some(1));
    }

    #[test]
    fn test_style_inheritance() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:style w:type="paragraph" w:styleId="Normal" w:default="1">
                <w:name w:val="Normal"/>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading1">
                <w:name w:val="heading 1"/>
                <w:basedOn w:val="Normal"/>
            </w:style>
        </w:styles>"#;

        let styles = StyleSheet::parse(xml).unwrap();
        let chain = styles.resolve_chain("Heading1");

        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, "Heading1");
        assert_eq!(chain[1].id, "Normal");
    }
}
