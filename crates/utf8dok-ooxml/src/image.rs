//! Image support for OOXML documents
//!
//! This module provides data structures and utilities for handling embedded
//! images in Word documents. Images in OOXML are embedded via `<w:drawing>`
//! elements containing either inline (`<wp:inline>`) or anchored (`<wp:anchor>`)
//! positioning.
//!
//! # OOXML Image Structure
//!
//! ```xml
//! <w:drawing>
//!   <wp:inline|wp:anchor>
//!     <wp:extent cx="..." cy="..."/>           <!-- Dimensions in EMUs -->
//!     <wp:docPr id="..." name="..." descr="..."/>  <!-- Alt text -->
//!     <a:graphic>
//!       <a:graphicData uri="...picture">
//!         <pic:pic>
//!           <pic:blipFill>
//!             <a:blip r:embed="rIdNN"/>        <!-- Relationship ID -->
//!           </pic:blipFill>
//!         </pic:pic>
//!       </a:graphicData>
//!     </a:graphic>
//!   </wp:inline|wp:anchor>
//! </w:drawing>
//! ```
//!
//! # Unit Conversions
//!
//! OOXML uses EMUs (English Metric Units) for dimensions:
//! - 914400 EMUs = 1 inch
//! - 9525 EMUs = 1 pixel (at 96 DPI)

/// EMUs per inch (914400)
pub const EMU_PER_INCH: i64 = 914400;

/// EMUs per pixel at 96 DPI (9525)
pub const EMU_PER_PIXEL: i64 = 9525;

/// An embedded image in a document
#[derive(Debug, Clone)]
pub struct Image {
    /// Unique identifier within the document
    pub id: u32,
    /// Relationship ID (e.g., "rId11")
    pub rel_id: String,
    /// Target path in archive (e.g., "media/image1.png")
    pub target: String,
    /// Alt text / description from docPr
    pub alt: Option<String>,
    /// Name from docPr
    pub name: Option<String>,
    /// Width in EMUs
    pub width_emu: Option<i64>,
    /// Height in EMUs
    pub height_emu: Option<i64>,
    /// Position type (inline or anchored)
    pub position: ImagePosition,
}

impl Image {
    /// Create a new inline image
    pub fn new_inline(id: u32, rel_id: String, target: String) -> Self {
        Self {
            id,
            rel_id,
            target,
            alt: None,
            name: None,
            width_emu: None,
            height_emu: None,
            position: ImagePosition::Inline,
        }
    }

    /// Create a new anchored image
    pub fn new_anchor(
        id: u32,
        rel_id: String,
        target: String,
        horizontal: i64,
        vertical: i64,
        wrap: WrapType,
    ) -> Self {
        Self {
            id,
            rel_id,
            target,
            alt: None,
            name: None,
            width_emu: None,
            height_emu: None,
            position: ImagePosition::Anchor {
                horizontal,
                vertical,
                wrap,
            },
        }
    }

    /// Set alt text
    pub fn with_alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Set name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set dimensions in EMUs
    pub fn with_dimensions_emu(mut self, width: i64, height: i64) -> Self {
        self.width_emu = Some(width);
        self.height_emu = Some(height);
        self
    }

    /// Set dimensions in pixels (converts to EMUs)
    pub fn with_dimensions_px(mut self, width: u32, height: u32) -> Self {
        self.width_emu = Some(width as i64 * EMU_PER_PIXEL);
        self.height_emu = Some(height as i64 * EMU_PER_PIXEL);
        self
    }

    /// Get width in pixels (at 96 DPI)
    pub fn width_px(&self) -> Option<u32> {
        self.width_emu.map(|emu| (emu / EMU_PER_PIXEL) as u32)
    }

    /// Get height in pixels (at 96 DPI)
    pub fn height_px(&self) -> Option<u32> {
        self.height_emu.map(|emu| (emu / EMU_PER_PIXEL) as u32)
    }

    /// Get the filename from target path
    pub fn filename(&self) -> &str {
        self.target.rsplit('/').next().unwrap_or(&self.target)
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.filename().rsplit('.').next()
    }

    /// Check if this is an inline image
    pub fn is_inline(&self) -> bool {
        matches!(self.position, ImagePosition::Inline)
    }

    /// Check if this is an anchored image
    pub fn is_anchor(&self) -> bool {
        matches!(self.position, ImagePosition::Anchor { .. })
    }
}

/// Image positioning type
#[derive(Debug, Clone, PartialEq)]
pub enum ImagePosition {
    /// Flows inline with text
    Inline,
    /// Floating, anchored to a position
    Anchor {
        /// Horizontal offset in EMUs
        horizontal: i64,
        /// Vertical offset in EMUs
        vertical: i64,
        /// Text wrapping style
        wrap: WrapType,
    },
}

impl Default for ImagePosition {
    fn default() -> Self {
        Self::Inline
    }
}

/// Text wrapping style for anchored images
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapType {
    /// No text wrapping
    #[default]
    None,
    /// Square wrapping (text flows around bounding box)
    Square,
    /// Tight wrapping (text follows image contour)
    Tight,
    /// Through wrapping (text flows through transparent areas)
    Through,
    /// Top and bottom (text above and below only)
    TopAndBottom,
}

impl WrapType {
    /// Parse wrap type from OOXML element name
    pub fn from_element_name(name: &str) -> Self {
        match name {
            "wrapSquare" => Self::Square,
            "wrapTight" => Self::Tight,
            "wrapThrough" => Self::Through,
            "wrapTopAndBottom" => Self::TopAndBottom,
            "wrapNone" => Self::None,
            _ => Self::None,
        }
    }

    /// Get the OOXML element name for this wrap type
    pub fn element_name(&self) -> &'static str {
        match self {
            Self::None => "wrapNone",
            Self::Square => "wrapSquare",
            Self::Tight => "wrapTight",
            Self::Through => "wrapThrough",
            Self::TopAndBottom => "wrapTopAndBottom",
        }
    }
}

/// Convert EMUs to pixels at 96 DPI
pub fn emu_to_pixels(emu: i64) -> i64 {
    (emu as f64 / EMU_PER_PIXEL as f64).round() as i64
}

/// Convert pixels to EMUs at 96 DPI
pub fn pixels_to_emu(pixels: i64) -> i64 {
    pixels * EMU_PER_PIXEL
}

/// Convert EMUs to inches
pub fn emu_to_inches(emu: i64) -> f64 {
    emu as f64 / EMU_PER_INCH as f64
}

/// Convert inches to EMUs
pub fn inches_to_emu(inches: f64) -> i64 {
    (inches * EMU_PER_INCH as f64).round() as i64
}

/// Get the MIME content type for an image extension
pub fn content_type_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
        "tiff" | "tif" => "image/tiff",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emu_to_pixels() {
        // 914400 EMUs = 1 inch = 96 pixels at 96 DPI
        assert_eq!(emu_to_pixels(914400), 96);
    }

    #[test]
    fn test_pixels_to_emu() {
        // 96 pixels = 914400 EMUs
        assert_eq!(pixels_to_emu(96), 914400);
    }

    #[test]
    fn test_image_dimensions() {
        let img = Image::new_inline(1, "rId1".to_string(), "media/image1.png".to_string())
            .with_dimensions_px(200, 150);

        assert_eq!(img.width_px(), Some(200));
        assert_eq!(img.height_px(), Some(150));
        assert_eq!(img.width_emu, Some(200 * EMU_PER_PIXEL));
        assert_eq!(img.height_emu, Some(150 * EMU_PER_PIXEL));
    }

    #[test]
    fn test_image_filename() {
        let img = Image::new_inline(1, "rId1".to_string(), "media/image1.png".to_string());
        assert_eq!(img.filename(), "image1.png");
        assert_eq!(img.extension(), Some("png"));
    }

    #[test]
    fn test_content_type_for_extension() {
        assert_eq!(content_type_for_extension("png"), "image/png");
        assert_eq!(content_type_for_extension("PNG"), "image/png");
        assert_eq!(content_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(content_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(content_type_for_extension("svg"), "image/svg+xml");
        assert_eq!(content_type_for_extension("emf"), "image/x-emf");
    }

    #[test]
    fn test_wrap_type_parsing() {
        assert_eq!(WrapType::from_element_name("wrapSquare"), WrapType::Square);
        assert_eq!(WrapType::from_element_name("wrapTight"), WrapType::Tight);
        assert_eq!(
            WrapType::from_element_name("wrapTopAndBottom"),
            WrapType::TopAndBottom
        );
        assert_eq!(WrapType::from_element_name("unknown"), WrapType::None);
    }

    #[test]
    fn test_image_position_types() {
        let inline = ImagePosition::Inline;
        let anchor = ImagePosition::Anchor {
            horizontal: 100,
            vertical: 200,
            wrap: WrapType::Square,
        };

        assert_eq!(inline, ImagePosition::Inline);
        if let ImagePosition::Anchor {
            horizontal,
            vertical,
            wrap,
        } = anchor
        {
            assert_eq!(horizontal, 100);
            assert_eq!(vertical, 200);
            assert_eq!(wrap, WrapType::Square);
        } else {
            panic!("Expected Anchor");
        }
    }
}
