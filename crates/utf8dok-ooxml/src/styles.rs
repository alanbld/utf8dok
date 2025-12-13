//! Style definitions parsing (word/styles.xml)
//!
//! This module parses Word style definitions and provides
//! utilities for understanding the style hierarchy.

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
        self.styles.values().filter(|s| s.style_type == StyleType::Paragraph)
    }

    /// Get all heading styles (styles with outline level)
    pub fn heading_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles.values().filter(|s| s.outline_level.is_some())
    }

    /// Get all table styles
    pub fn table_styles(&self) -> impl Iterator<Item = &Style> {
        self.styles.values().filter(|s| s.style_type == StyleType::Table)
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

#[cfg(test)]
mod tests {
    use super::*;

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
