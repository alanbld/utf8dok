//! POTX template loading and management.
//!
//! This module handles loading PowerPoint templates (.potx files) and
//! extracting layout information for content injection.

use crate::error::{PptxError, Result};
use crate::layout::{LayoutMapping, LayoutType, PlaceholderInfo, PlaceholderType, SlideLayout};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

/// Represents a loaded PPTX/POTX template
#[derive(Debug)]
pub struct PotxTemplate {
    /// Template file path (if loaded from file)
    pub path: Option<String>,

    /// Slide layouts extracted from template
    layouts: Vec<SlideLayout>,

    /// Theme information
    pub theme: Option<ThemeInfo>,

    /// Slide dimensions (width, height) in EMU
    pub slide_size: (i64, i64),

    /// Raw template archive (for content injection)
    archive_data: Vec<u8>,
}

/// Theme information from template
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Theme name
    pub name: String,

    /// Major font (headings)
    pub major_font: String,

    /// Minor font (body)
    pub minor_font: String,

    /// Color scheme
    pub colors: HashMap<String, String>,
}

impl Default for ThemeInfo {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            major_font: "Calibri Light".to_string(),
            minor_font: "Calibri".to_string(),
            colors: HashMap::new(),
        }
    }
}

impl PotxTemplate {
    /// Load a template from a file path
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        let mut template = Self::from_bytes(&data)?;
        template.path = Some(path.display().to_string());
        Ok(template)
    }

    /// Load a template from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let cursor = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)?;

        // Extract slide size from presentation.xml
        let slide_size = Self::extract_slide_size(&mut archive)?;

        // Extract layouts
        let layouts = Self::extract_layouts(&mut archive)?;

        // Extract theme (optional)
        let theme = Self::extract_theme(&mut archive).ok();

        Ok(Self {
            path: None,
            layouts,
            theme,
            slide_size,
            archive_data: data.to_vec(),
        })
    }

    /// Create a minimal template (for when no template is provided)
    pub fn minimal() -> Self {
        // Create a minimal set of standard layouts
        let layouts = vec![
            SlideLayout::new(1, "Title Slide", LayoutType::Title),
            SlideLayout::new(2, "Title and Content", LayoutType::TitleAndContent),
            SlideLayout::new(3, "Section Header", LayoutType::SectionHeader),
            SlideLayout::new(4, "Two Content", LayoutType::TwoContent),
            SlideLayout::new(5, "Comparison", LayoutType::Comparison),
            SlideLayout::new(6, "Title Only", LayoutType::TitleOnly),
            SlideLayout::new(7, "Blank", LayoutType::Blank),
            SlideLayout::new(8, "Content with Caption", LayoutType::ContentWithCaption),
            SlideLayout::new(9, "Picture with Caption", LayoutType::PictureWithCaption),
        ];

        Self {
            path: None,
            layouts,
            theme: Some(ThemeInfo::default()),
            slide_size: (9_144_000, 6_858_000), // Standard 4:3
            archive_data: Vec::new(),
        }
    }

    /// Get all layouts
    pub fn layouts(&self) -> &[SlideLayout] {
        &self.layouts
    }

    /// Get a layout by index (1-based)
    pub fn get_layout(&self, index: u32) -> Option<&SlideLayout> {
        self.layouts.iter().find(|l| l.index == index)
    }

    /// Get the number of layouts
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }

    /// Create a LayoutMapping from this template
    pub fn to_layout_mapping(&self) -> LayoutMapping {
        let mut mapping = LayoutMapping::new();
        for layout in &self.layouts {
            mapping.add_layout(layout.clone());
        }
        mapping
    }

    /// Check if this is a minimal (generated) template
    pub fn is_minimal(&self) -> bool {
        self.archive_data.is_empty()
    }

    /// Get the raw archive data
    pub fn archive_data(&self) -> &[u8] {
        &self.archive_data
    }

    /// Extract slide size from presentation.xml
    fn extract_slide_size<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<(i64, i64)> {
        let presentation_xml = match archive.by_name("ppt/presentation.xml") {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                contents
            }
            Err(_) => {
                // Default size if not found
                return Ok((9_144_000, 6_858_000));
            }
        };

        // Parse XML to find sldSz element
        let mut reader = Reader::from_str(&presentation_xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut width = 9_144_000i64;
        let mut height = 6_858_000i64;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"p:sldSz" => {
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"cx" => {
                                if let Ok(v) = std::str::from_utf8(&attr.value) {
                                    width = v.parse().unwrap_or(width);
                                }
                            }
                            b"cy" => {
                                if let Ok(v) = std::str::from_utf8(&attr.value) {
                                    height = v.parse().unwrap_or(height);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok((width, height))
    }

    /// Extract layouts from slideLayouts directory
    fn extract_layouts<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<SlideLayout>> {
        let mut layouts = Vec::new();

        // Find all slideLayout files
        let layout_files: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                archive
                    .by_index(i)
                    .ok()
                    .map(|f| f.name().to_string())
                    .filter(|name| {
                        name.starts_with("ppt/slideLayouts/slideLayout")
                            && name.ends_with(".xml")
                    })
            })
            .collect();

        for file_name in layout_files {
            // Extract index from filename (e.g., "slideLayout1.xml" -> 1)
            let index = file_name
                .trim_start_matches("ppt/slideLayouts/slideLayout")
                .trim_end_matches(".xml")
                .parse::<u32>()
                .unwrap_or(0);

            if index == 0 {
                continue;
            }

            // Read layout XML
            let mut file = archive.by_name(&file_name)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            // Parse layout
            if let Ok(layout) = Self::parse_layout_xml(index, &contents) {
                layouts.push(layout);
            }
        }

        // Sort by index
        layouts.sort_by_key(|l| l.index);

        Ok(layouts)
    }

    /// Parse a single layout XML file
    fn parse_layout_xml(index: u32, xml: &str) -> Result<SlideLayout> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut layout_type = LayoutType::Custom;
        let mut layout_name = format!("Layout {}", index);
        let mut placeholders = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                    if e.name().as_ref() == b"p:cSld" =>
                {
                    // Get layout name from cSld name attribute
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            if let Ok(name) = std::str::from_utf8(&attr.value) {
                                layout_name = name.to_string();
                            }
                        }
                    }
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"p:ph" => {
                    // Parse placeholder
                    let mut ph_type = PlaceholderType::Other;
                    let mut ph_idx = 0u32;

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"type" => {
                                if let Ok(t) = std::str::from_utf8(&attr.value) {
                                    ph_type = PlaceholderType::from_ooxml_type(t);

                                    // Infer layout type from placeholder types
                                    if ph_type == PlaceholderType::CenterTitle {
                                        layout_type = LayoutType::Title;
                                    }
                                }
                            }
                            b"idx" => {
                                if let Ok(i) = std::str::from_utf8(&attr.value) {
                                    ph_idx = i.parse().unwrap_or(0);
                                }
                            }
                            _ => {}
                        }
                    }

                    placeholders.push(PlaceholderInfo::new(
                        ph_idx,
                        ph_type,
                        (0, 0),     // Position will be extracted from spPr
                        (0, 0),     // Size will be extracted from spPr
                    ));
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        // Infer layout type from name if not already set
        if layout_type == LayoutType::Custom {
            layout_type = infer_layout_type(&layout_name);
        }

        let mut layout = SlideLayout::new(index, layout_name, layout_type);
        for ph in placeholders {
            layout.add_placeholder(ph);
        }

        Ok(layout)
    }

    /// Extract theme information
    fn extract_theme<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<ThemeInfo> {
        let mut file = archive.by_name("ppt/theme/theme1.xml")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut theme = ThemeInfo::default();
        let mut reader = Reader::from_str(&contents);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                    if e.name().as_ref() == b"a:theme" =>
                {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            if let Ok(name) = std::str::from_utf8(&attr.value) {
                                theme.name = name.to_string();
                            }
                        }
                    }
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"a:majorFont" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"typeface" {
                            if let Ok(font) = std::str::from_utf8(&attr.value) {
                                theme.major_font = font.to_string();
                            }
                        }
                    }
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"a:minorFont" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"typeface" {
                            if let Ok(font) = std::str::from_utf8(&attr.value) {
                                theme.minor_font = font.to_string();
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(theme)
    }
}

/// Infer layout type from layout name
fn infer_layout_type(name: &str) -> LayoutType {
    let name_lower = name.to_lowercase();

    if name_lower.contains("title slide") || name_lower.contains("titolo") {
        LayoutType::Title
    } else if name_lower.contains("section") || name_lower.contains("sezione") {
        LayoutType::SectionHeader
    } else if name_lower.contains("two") || name_lower.contains("due") {
        LayoutType::TwoContent
    } else if name_lower.contains("comparison") || name_lower.contains("confronto") {
        LayoutType::Comparison
    } else if name_lower.contains("blank") || name_lower.contains("vuoto") {
        LayoutType::Blank
    } else if name_lower.contains("title only") {
        LayoutType::TitleOnly
    } else if name_lower.contains("picture") || name_lower.contains("immagine") {
        LayoutType::PictureWithCaption
    } else if name_lower.contains("content") || name_lower.contains("contenuto") {
        LayoutType::TitleAndContent
    } else {
        LayoutType::Custom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_template() {
        let template = PotxTemplate::minimal();

        assert!(template.is_minimal());
        assert_eq!(template.layout_count(), 9);
        assert!(template.get_layout(1).is_some());
        assert!(template.get_layout(2).is_some());
        assert!(template.get_layout(99).is_none());
    }

    #[test]
    fn test_template_slide_size() {
        let template = PotxTemplate::minimal();

        // Standard 4:3 dimensions
        assert_eq!(template.slide_size.0, 9_144_000);
        assert_eq!(template.slide_size.1, 6_858_000);
    }

    #[test]
    fn test_to_layout_mapping() {
        let template = PotxTemplate::minimal();
        let mapping = template.to_layout_mapping();

        assert_eq!(mapping.layout_count(), 9);
    }

    #[test]
    fn test_infer_layout_type() {
        assert_eq!(
            infer_layout_type("Title Slide"),
            LayoutType::Title
        );
        assert_eq!(
            infer_layout_type("Section Header"),
            LayoutType::SectionHeader
        );
        assert_eq!(
            infer_layout_type("Two Content"),
            LayoutType::TwoContent
        );
        assert_eq!(
            infer_layout_type("Blank"),
            LayoutType::Blank
        );
        assert_eq!(
            infer_layout_type("Title and Content"),
            LayoutType::TitleAndContent
        );
        assert_eq!(
            infer_layout_type("Custom Layout"),
            LayoutType::Custom
        );

        // Italian names
        assert_eq!(
            infer_layout_type("Diapositiva titolo"),
            LayoutType::Title
        );
        assert_eq!(
            infer_layout_type("Contenuto"),
            LayoutType::TitleAndContent
        );
    }

    #[test]
    fn test_theme_default() {
        let theme = ThemeInfo::default();

        assert_eq!(theme.name, "Default");
        assert_eq!(theme.major_font, "Calibri Light");
        assert_eq!(theme.minor_font, "Calibri");
    }
}
