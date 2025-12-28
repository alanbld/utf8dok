//! Diagram renderer trait and error types
//!
//! This module defines the core abstraction for diagram renderers,
//! enabling a pluggable, fallback-based rendering architecture.

use crate::types::{DiagramType, OutputFormat};

/// Errors that can occur during diagram rendering
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The diagram type is not supported by this renderer
    #[error("Unsupported diagram type: {0}")]
    UnsupportedType(DiagramType),

    /// The output format is not supported
    #[error("Unsupported output format: {0}")]
    UnsupportedFormat(OutputFormat),

    /// The renderer is not available (e.g., network unavailable for Kroki)
    #[error("Renderer unavailable: {0}")]
    Unavailable(String),

    /// The diagram source is invalid or malformed
    #[error("Invalid source: {0}")]
    InvalidSource(String),

    /// Rendering failed during execution
    #[error("Rendering failed: {0}")]
    RenderFailed(String),

    /// The renderer panicked (caught via catch_unwind)
    #[error("Renderer panicked: {0}")]
    Panic(String),

    /// Network/HTTP error (Kroki)
    #[error("Network error: {0}")]
    Network(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for renderer operations
pub type RenderResult<T> = std::result::Result<T, RenderError>;

/// Options for diagram rendering
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Target width in pixels (renderer may scale proportionally)
    pub width: Option<u32>,
    /// Target height in pixels (renderer may scale proportionally)
    pub height: Option<u32>,
    /// Background color (CSS color string, e.g., "white", "#ffffff")
    pub background: Option<String>,
    /// Scale factor (1.0 = 100%)
    pub scale: Option<f32>,
}

impl RenderOptions {
    /// Create new options with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set target width
    pub fn with_width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set target height
    pub fn with_height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set background color
    pub fn with_background(mut self, bg: impl Into<String>) -> Self {
        self.background = Some(bg.into());
        self
    }

    /// Set scale factor
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Trait for diagram renderers
///
/// Implementors provide diagram rendering capabilities for specific
/// diagram types. The engine orchestrates multiple renderers with
/// fallback logic.
///
/// # Thread Safety
///
/// Renderers must be `Send + Sync` to support concurrent rendering
/// and use in async contexts.
pub trait DiagramRenderer: Send + Sync {
    /// Human-readable name of this renderer
    fn name(&self) -> &'static str;

    /// Check if this renderer supports the given diagram type
    fn supports(&self, diagram_type: DiagramType) -> bool;

    /// Check if this renderer supports the given output format
    fn supports_format(&self, format: OutputFormat) -> bool {
        // Default: support PNG and SVG
        matches!(format, OutputFormat::Png | OutputFormat::Svg)
    }

    /// Check if the renderer is currently available
    ///
    /// For native renderers, this always returns true.
    /// For network-based renderers (Kroki), this may perform a health check.
    fn is_available(&self) -> bool {
        true
    }

    /// Render a diagram to the specified format
    ///
    /// # Arguments
    /// * `source` - The diagram source code
    /// * `diagram_type` - The type of diagram
    /// * `format` - The desired output format
    /// * `options` - Optional rendering parameters
    ///
    /// # Returns
    /// The rendered diagram as bytes (PNG, SVG, etc.)
    fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        options: &RenderOptions,
    ) -> RenderResult<Vec<u8>>;

    /// Get the list of diagram types this renderer supports
    fn supported_types(&self) -> Vec<DiagramType> {
        DiagramType::all()
            .iter()
            .filter(|t| self.supports(**t))
            .copied()
            .collect()
    }

    /// Render to PNG with default options (convenience method)
    fn render_png(&self, source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
        self.render(
            source,
            diagram_type,
            OutputFormat::Png,
            &RenderOptions::default(),
        )
    }

    /// Render to SVG with default options (convenience method)
    fn render_svg(&self, source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
        self.render(
            source,
            diagram_type,
            OutputFormat::Svg,
            &RenderOptions::default(),
        )
    }
}

/// Rendered diagram with metadata
#[derive(Debug, Clone)]
pub struct RenderedDiagram {
    /// The rendered image bytes
    pub data: Vec<u8>,
    /// The diagram type that was rendered
    pub diagram_type: DiagramType,
    /// The output format
    pub format: OutputFormat,
    /// SHA-256 hash of the source (for cache invalidation)
    pub source_hash: String,
    /// Name of the renderer that produced this output
    pub renderer: String,
}

impl RenderedDiagram {
    /// Create a new rendered diagram
    pub fn new(
        data: Vec<u8>,
        diagram_type: DiagramType,
        format: OutputFormat,
        source_hash: String,
        renderer: impl Into<String>,
    ) -> Self {
        Self {
            data,
            diagram_type,
            format,
            source_hash,
            renderer: renderer.into(),
        }
    }

    /// Get the file extension for this diagram
    pub fn extension(&self) -> &'static str {
        self.format.extension()
    }

    /// Get the MIME type for this diagram
    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }

    /// Check if the data appears to be a valid PNG
    pub fn is_valid_png(&self) -> bool {
        self.data.len() >= 8 && &self.data[0..8] == b"\x89PNG\r\n\x1a\n"
    }

    /// Check if the data appears to be valid SVG
    pub fn is_valid_svg(&self) -> bool {
        if let Ok(s) = std::str::from_utf8(&self.data) {
            s.contains("<svg") || s.contains("<?xml")
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_options_builder() {
        let opts = RenderOptions::new()
            .with_width(800)
            .with_height(600)
            .with_background("white")
            .with_scale(2.0);

        assert_eq!(opts.width, Some(800));
        assert_eq!(opts.height, Some(600));
        assert_eq!(opts.background, Some("white".to_string()));
        assert_eq!(opts.scale, Some(2.0));
    }

    #[test]
    fn test_rendered_diagram_validation() {
        // Valid PNG header
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        let diagram = RenderedDiagram::new(
            png_data,
            DiagramType::Svgbob,
            OutputFormat::Png,
            "sha256:test".to_string(),
            "test-renderer",
        );
        assert!(diagram.is_valid_png());
        assert!(!diagram.is_valid_svg());

        // Valid SVG
        let svg_data = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec();
        let svg_diagram = RenderedDiagram::new(
            svg_data,
            DiagramType::Svgbob,
            OutputFormat::Svg,
            "sha256:test".to_string(),
            "test-renderer",
        );
        assert!(svg_diagram.is_valid_svg());
        assert!(!svg_diagram.is_valid_png());
    }
}
