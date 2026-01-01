//! # utf8dok-pptx
//!
//! PowerPoint (PPTX) generation from AsciiDoc sources.
//!
//! This crate provides functionality to generate PPTX presentations from AsciiDoc
//! documents, supporting both dedicated presentation documents and "dual-nature"
//! documents that produce both DOCX and PPTX from a single source.
//!
//! ## Features
//!
//! - **Template Injection**: Inject content into corporate POTX templates
//! - **Dual-Nature Documents**: Extract `[slides]` blocks for PPTX, rest for DOCX
//! - **Reveal.js Compatible**: Uses `== Heading` for slide boundaries
//! - **Speaker Notes**: Support for `[.notes]` blocks
//! - **SlideContract**: TOML-based mapping of semantic types to layouts
//!
//! ## Example
//!
//! ```rust,ignore
//! use utf8dok_pptx::{PptxWriter, SlideContract};
//! use utf8dok_ast::Document;
//!
//! let doc = Document::default();
//! let contract = SlideContract::default();
//! let mut writer = PptxWriter::new(contract);
//!
//! let pptx_bytes = writer.generate(&doc)?;
//! std::fs::write("output.pptx", pptx_bytes)?;
//! ```

pub mod error;
pub mod extractor;
pub mod layout;
pub mod slide;
pub mod slide_contract;
pub mod template;
pub mod writer;

// Re-exports
pub use error::{PptxError, Result};
pub use extractor::{Deck, ExtractorConfig, SlideExtractor};
pub use layout::{LayoutMapping, SlideLayout};
pub use slide::{Slide, SlideContent, SpeakerNotes};
pub use slide_contract::SlideContract;
pub use template::PotxTemplate;
pub use writer::PptxWriter;

/// PPTX-related constants
pub mod constants {
    /// Default slide width in EMU (914400 EMU = 1 inch, standard 10" width)
    pub const DEFAULT_SLIDE_WIDTH_EMU: i64 = 9_144_000;

    /// Default slide height in EMU (standard 7.5" height for 4:3)
    pub const DEFAULT_SLIDE_HEIGHT_EMU: i64 = 6_858_000;

    /// Widescreen 16:9 slide width in EMU (13.333" width)
    pub const WIDESCREEN_SLIDE_WIDTH_EMU: i64 = 12_192_000;

    /// Widescreen 16:9 slide height in EMU (7.5" height)
    pub const WIDESCREEN_SLIDE_HEIGHT_EMU: i64 = 6_858_000;

    /// EMU per inch
    pub const EMU_PER_INCH: i64 = 914_400;

    /// EMU per point
    pub const EMU_PER_POINT: i64 = 12_700;

    /// EMU per centimeter
    pub const EMU_PER_CM: i64 = 360_000;

    /// Half-points per point (for font sizes)
    pub const HALF_POINTS_PER_POINT: u32 = 2;

    /// PresentationML namespace
    pub const NS_PRESENTATION: &str =
        "http://schemas.openxmlformats.org/presentationml/2006/main";

    /// DrawingML namespace
    pub const NS_DRAWING: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    /// Relationships namespace
    pub const NS_RELATIONSHIPS: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    /// Content Types namespace
    pub const NS_CONTENT_TYPES: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

    /// Slide relationship type
    pub const REL_TYPE_SLIDE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

    /// Slide layout relationship type
    pub const REL_TYPE_SLIDE_LAYOUT: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";

    /// Slide master relationship type
    pub const REL_TYPE_SLIDE_MASTER: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";

    /// Notes slide relationship type
    pub const REL_TYPE_NOTES_SLIDE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";

    /// Theme relationship type
    pub const REL_TYPE_THEME: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

    /// Image relationship type
    pub const REL_TYPE_IMAGE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emu_constants() {
        // Verify EMU calculations
        assert_eq!(constants::EMU_PER_INCH, 914_400);
        assert_eq!(constants::EMU_PER_POINT, 12_700);

        // 1 inch = 72 points, so EMU_PER_INCH should be 72 * EMU_PER_POINT
        assert_eq!(
            constants::EMU_PER_INCH,
            72 * constants::EMU_PER_POINT
        );
    }

    #[test]
    fn test_default_slide_dimensions() {
        // Standard 4:3 slide is 10" x 7.5"
        let expected_width = 10 * constants::EMU_PER_INCH;
        let expected_height = (7.5 * constants::EMU_PER_INCH as f64) as i64;

        assert_eq!(constants::DEFAULT_SLIDE_WIDTH_EMU, expected_width);
        assert_eq!(constants::DEFAULT_SLIDE_HEIGHT_EMU, expected_height);
    }

    #[test]
    fn test_widescreen_dimensions() {
        // 16:9 aspect ratio check
        let aspect_ratio = constants::WIDESCREEN_SLIDE_WIDTH_EMU as f64
            / constants::WIDESCREEN_SLIDE_HEIGHT_EMU as f64;

        // Should be approximately 16:9 = 1.777...
        assert!((aspect_ratio - 16.0 / 9.0).abs() < 0.01);
    }
}
