//! Native Rust diagram renderers
//!
//! This module provides pure-Rust diagram rendering without external dependencies.
//! Currently supports:
//! - **Svgbob**: ASCII art to SVG conversion
//!
//! # Feature Flag
//!
//! This module requires the `native` feature:
//! ```toml
//! utf8dok-diagrams = { version = "0.1", features = ["native"] }
//! ```

use std::panic::catch_unwind;

use crate::renderer::{DiagramRenderer, RenderError, RenderOptions, RenderResult};
use crate::types::{DiagramType, OutputFormat};

/// Native Rust diagram renderer
///
/// Uses pure-Rust libraries for offline rendering:
/// - `svgbob` for ASCII art diagrams
/// - `resvg` for SVG to PNG conversion
///
/// # Example
///
/// ```ignore
/// use utf8dok_diagrams::{NativeRenderer, DiagramRenderer, DiagramType};
///
/// let renderer = NativeRenderer::new();
/// let png = renderer.render_png("+---+\n| A |\n+---+", DiagramType::Svgbob)?;
/// ```
pub struct NativeRenderer {
    /// Font database for text rendering (kept for future use)
    #[allow(dead_code)]
    fontdb: usvg::fontdb::Database,
}

impl Default for NativeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeRenderer {
    /// Create a new native renderer with system fonts loaded
    pub fn new() -> Self {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();

        // Fallback to default font if no system fonts available
        if fontdb.is_empty() {
            log::warn!("No system fonts found, text rendering may be limited");
        }

        Self { fontdb }
    }

    /// Render svgbob ASCII art to SVG string
    fn render_svgbob_svg(&self, source: &str) -> RenderResult<String> {
        // Wrap in catch_unwind for safety (svgbob may panic on malformed input)
        let result = catch_unwind(|| svgbob::to_svg(source));

        match result {
            Ok(svg) => {
                if svg.is_empty() {
                    Err(RenderError::InvalidSource(
                        "Svgbob produced empty output".to_string(),
                    ))
                } else {
                    Ok(svg)
                }
            }
            Err(_) => Err(RenderError::Panic(
                "Svgbob panicked while parsing input".to_string(),
            )),
        }
    }

    /// Convert SVG string to PNG bytes using resvg
    fn svg_to_png(&self, svg: &str, options: &RenderOptions) -> RenderResult<Vec<u8>> {
        // Parse SVG with usvg
        let tree = {
            let opts = usvg::Options::default();
            usvg::Tree::from_str(svg, &opts)
                .map_err(|e| RenderError::RenderFailed(format!("SVG parsing failed: {}", e)))?
        };

        // Get original size
        let original_size = tree.size();
        let original_width = original_size.width();
        let original_height = original_size.height();

        // Calculate target dimensions
        let (target_width, target_height, scale) = self.calculate_dimensions(
            original_width,
            original_height,
            options,
        );

        // Create pixmap
        let mut pixmap = tiny_skia::Pixmap::new(target_width, target_height).ok_or_else(|| {
            RenderError::RenderFailed(format!(
                "Failed to create pixmap ({}x{})",
                target_width, target_height
            ))
        })?;

        // Fill background if specified
        if let Some(ref bg) = options.background {
            if let Some(color) = parse_color(bg) {
                pixmap.fill(color);
            }
        }

        // Create transform for scaling
        let transform = tiny_skia::Transform::from_scale(scale, scale);

        // Render SVG to pixmap
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // Encode to PNG
        pixmap
            .encode_png()
            .map_err(|e| RenderError::RenderFailed(format!("PNG encoding failed: {}", e)))
    }

    /// Calculate target dimensions and scale factor
    fn calculate_dimensions(
        &self,
        original_width: f32,
        original_height: f32,
        options: &RenderOptions,
    ) -> (u32, u32, f32) {
        // Apply explicit scale if specified
        let base_scale = options.scale.unwrap_or(1.0);

        // Calculate dimensions based on options
        match (options.width, options.height) {
            (Some(w), Some(h)) => {
                // Both specified: fit within bounds while maintaining aspect ratio
                let scale_x = w as f32 / original_width;
                let scale_y = h as f32 / original_height;
                let scale = scale_x.min(scale_y) * base_scale;
                let final_width = (original_width * scale).ceil() as u32;
                let final_height = (original_height * scale).ceil() as u32;
                (final_width, final_height, scale)
            }
            (Some(w), None) => {
                // Only width specified: scale proportionally
                let scale = (w as f32 / original_width) * base_scale;
                let final_height = (original_height * scale).ceil() as u32;
                (w, final_height, scale)
            }
            (None, Some(h)) => {
                // Only height specified: scale proportionally
                let scale = (h as f32 / original_height) * base_scale;
                let final_width = (original_width * scale).ceil() as u32;
                (final_width, h, scale)
            }
            (None, None) => {
                // No size specified: use original with scale
                let final_width = (original_width * base_scale).ceil() as u32;
                let final_height = (original_height * base_scale).ceil() as u32;
                (final_width.max(1), final_height.max(1), base_scale)
            }
        }
    }
}

impl DiagramRenderer for NativeRenderer {
    fn name(&self) -> &'static str {
        "native"
    }

    fn supports(&self, diagram_type: DiagramType) -> bool {
        matches!(diagram_type, DiagramType::Svgbob)
    }

    fn supports_format(&self, format: OutputFormat) -> bool {
        matches!(format, OutputFormat::Png | OutputFormat::Svg)
    }

    fn is_available(&self) -> bool {
        true // Native renderer is always available
    }

    fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        options: &RenderOptions,
    ) -> RenderResult<Vec<u8>> {
        // Validate diagram type
        if !self.supports(diagram_type) {
            return Err(RenderError::UnsupportedType(diagram_type));
        }

        // Validate format
        if !self.supports_format(format) {
            return Err(RenderError::UnsupportedFormat(format));
        }

        // Validate source is not empty
        let source = source.trim();
        if source.is_empty() {
            return Err(RenderError::InvalidSource("Empty diagram source".to_string()));
        }

        // Render based on diagram type
        match diagram_type {
            DiagramType::Svgbob => {
                let svg = self.render_svgbob_svg(source)?;

                match format {
                    OutputFormat::Svg => Ok(svg.into_bytes()),
                    OutputFormat::Png => self.svg_to_png(&svg, options),
                    _ => Err(RenderError::UnsupportedFormat(format)),
                }
            }
            _ => Err(RenderError::UnsupportedType(diagram_type)),
        }
    }
}

/// Parse a CSS color string to tiny_skia::Color
fn parse_color(color: &str) -> Option<tiny_skia::Color> {
    let color = color.trim().to_lowercase();

    // Named colors
    match color.as_str() {
        "white" => return Some(tiny_skia::Color::WHITE),
        "black" => return Some(tiny_skia::Color::BLACK),
        "transparent" => return Some(tiny_skia::Color::TRANSPARENT),
        "red" => return tiny_skia::Color::from_rgba8(255, 0, 0, 255).into(),
        "green" => return tiny_skia::Color::from_rgba8(0, 128, 0, 255).into(),
        "blue" => return tiny_skia::Color::from_rgba8(0, 0, 255, 255).into(),
        _ => {}
    }

    // Hex colors
    if let Some(hex) = color.strip_prefix('#') {
        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                return tiny_skia::Color::from_rgba8(r, g, b, 255).into();
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return tiny_skia::Color::from_rgba8(r, g, b, 255).into();
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                return tiny_skia::Color::from_rgba8(r, g, b, a).into();
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_renderer_name() {
        let renderer = NativeRenderer::new();
        assert_eq!(renderer.name(), "native");
    }

    #[test]
    fn test_native_renderer_supports_svgbob() {
        let renderer = NativeRenderer::new();
        assert!(renderer.supports(DiagramType::Svgbob));
        assert!(!renderer.supports(DiagramType::Mermaid));
        assert!(!renderer.supports(DiagramType::PlantUml));
    }

    #[test]
    fn test_native_renderer_supports_formats() {
        let renderer = NativeRenderer::new();
        assert!(renderer.supports_format(OutputFormat::Png));
        assert!(renderer.supports_format(OutputFormat::Svg));
        assert!(!renderer.supports_format(OutputFormat::Pdf));
    }

    #[test]
    fn test_native_renderer_always_available() {
        let renderer = NativeRenderer::new();
        assert!(renderer.is_available());
    }

    #[test]
    fn test_render_svgbob_to_svg() {
        let renderer = NativeRenderer::new();
        let source = r#"
        +------+
        | Test |
        +------+
        "#;

        let result = renderer.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Svg,
            &RenderOptions::default(),
        );

        assert!(result.is_ok());
        let svg = result.unwrap();
        let svg_str = String::from_utf8_lossy(&svg);
        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("</svg>"));
    }

    #[test]
    fn test_render_svgbob_to_png() {
        let renderer = NativeRenderer::new();
        let source = r#"
        +---+
        | A |
        +---+
        "#;

        let result = renderer.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(result.is_ok());
        let png = result.unwrap();
        // PNG magic bytes
        assert!(png.len() > 8);
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_render_with_width() {
        let renderer = NativeRenderer::new();
        let source = "+--+";

        let result = renderer.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::new().with_width(400),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_render_empty_source_fails() {
        let renderer = NativeRenderer::new();

        let result = renderer.render(
            "",
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(matches!(result, Err(RenderError::InvalidSource(_))));
    }

    #[test]
    fn test_render_unsupported_type_fails() {
        let renderer = NativeRenderer::new();

        let result = renderer.render(
            "graph TD; A-->B;",
            DiagramType::Mermaid,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(matches!(result, Err(RenderError::UnsupportedType(_))));
    }

    #[test]
    fn test_parse_color() {
        assert!(parse_color("white").is_some());
        assert!(parse_color("black").is_some());
        assert!(parse_color("#fff").is_some());
        assert!(parse_color("#ffffff").is_some());
        assert!(parse_color("#ffffffff").is_some());
        assert!(parse_color("invalid").is_none());
    }

    #[test]
    fn test_render_with_background() {
        let renderer = NativeRenderer::new();
        let source = "+--+";

        let result = renderer.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::new().with_background("white"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_svgbob_diagram() {
        let renderer = NativeRenderer::new();
        let source = r#"
                         .-,(  ),-.
          ___  _      .-(          )-.
         [___]|=| -->(    Internet    )
         /::/ |move  '-(googol.com) -'
                        '-(googol).-'
        "#;

        let result = renderer.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(result.is_ok());
    }
}
