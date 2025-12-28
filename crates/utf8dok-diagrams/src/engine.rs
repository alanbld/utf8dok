//! Diagram rendering engine with fallback chain
//!
//! This module provides the main entry point for diagram rendering,
//! orchestrating multiple renderers with priority-based fallback.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     DiagramEngine                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Priority Order:                                             │
//! │  1. NativeRenderer (svgbob) - offline, fast                 │
//! │  2. KrokiRenderer - full diagram support                    │
//! │  3. Future: DenoRenderer - embedded Mermaid.js              │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use sha2::{Digest, Sha256};

use crate::renderer::{DiagramRenderer, RenderError, RenderOptions, RenderResult, RenderedDiagram};
use crate::types::{DiagramType, OutputFormat};

/// Diagram rendering engine with fallback chain
///
/// The engine manages multiple renderers and routes rendering requests
/// to the most appropriate one based on:
/// 1. Diagram type support
/// 2. Renderer availability
/// 3. Priority order (native → cloud)
///
/// # Example
///
/// ```ignore
/// use utf8dok_diagrams::{DiagramEngine, DiagramType, OutputFormat, RenderOptions};
///
/// let engine = DiagramEngine::new();
///
/// // Render svgbob natively (offline)
/// let png = engine.render(
///     "+---+\n| A |\n+---+",
///     DiagramType::Svgbob,
///     OutputFormat::Png,
///     &RenderOptions::default(),
/// )?;
///
/// // Render Mermaid via Kroki (requires network)
/// let svg = engine.render(
///     "graph TD; A-->B;",
///     DiagramType::Mermaid,
///     OutputFormat::Svg,
///     &RenderOptions::default(),
/// )?;
/// ```
pub struct DiagramEngine {
    /// Registered renderers in priority order
    renderers: Vec<Box<dyn DiagramRenderer>>,
}

impl Default for DiagramEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagramEngine {
    /// Create a new diagram engine with default renderers
    ///
    /// Renderers are added based on enabled features:
    /// - `native`: NativeRenderer for svgbob
    /// - `kroki`: KrokiRenderer for cloud rendering
    pub fn new() -> Self {
        let mut renderers: Vec<Box<dyn DiagramRenderer>> = Vec::new();

        // Priority 1: Native renderer (always preferred for supported types)
        #[cfg(feature = "native")]
        {
            renderers.push(Box::new(crate::native::NativeRenderer::new()));
            log::debug!("Registered native renderer");
        }

        // Priority 2: Kroki renderer (fallback for network-available rendering)
        #[cfg(feature = "kroki")]
        {
            match crate::kroki::KrokiRenderer::new() {
                Ok(kroki) => {
                    renderers.push(Box::new(kroki));
                    log::debug!("Registered Kroki renderer");
                }
                Err(e) => {
                    log::warn!("Failed to initialize Kroki renderer: {}", e);
                }
            }
        }

        Self { renderers }
    }

    /// Create an engine with no renderers (for testing)
    pub fn empty() -> Self {
        Self {
            renderers: Vec::new(),
        }
    }

    /// Add a custom renderer to the engine
    ///
    /// Renderers are tried in the order they were added.
    pub fn add_renderer(&mut self, renderer: Box<dyn DiagramRenderer>) {
        log::debug!("Added renderer: {}", renderer.name());
        self.renderers.push(renderer);
    }

    /// Insert a renderer at a specific priority position
    ///
    /// Lower indices = higher priority.
    pub fn insert_renderer(&mut self, index: usize, renderer: Box<dyn DiagramRenderer>) {
        log::debug!("Inserted renderer at position {}: {}", index, renderer.name());
        self.renderers.insert(index.min(self.renderers.len()), renderer);
    }

    /// Get the names of all registered renderers
    pub fn renderer_names(&self) -> Vec<&'static str> {
        self.renderers.iter().map(|r| r.name()).collect()
    }

    /// Get all diagram types supported by any renderer
    pub fn supported_types(&self) -> Vec<DiagramType> {
        let mut types = Vec::new();
        for dtype in DiagramType::all() {
            if self.renderers.iter().any(|r| r.supports(*dtype)) {
                types.push(*dtype);
            }
        }
        types
    }

    /// Check if a specific diagram type is supported
    pub fn supports(&self, diagram_type: DiagramType) -> bool {
        self.renderers.iter().any(|r| r.supports(diagram_type))
    }

    /// Render a diagram using the best available renderer
    ///
    /// Tries renderers in priority order until one succeeds.
    /// Returns an error only if all renderers fail.
    pub fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        options: &RenderOptions,
    ) -> RenderResult<Vec<u8>> {
        if self.renderers.is_empty() {
            return Err(RenderError::Unavailable(
                "No renderers available. Enable 'native' or 'kroki' feature.".to_string(),
            ));
        }

        let mut last_error = None;

        for renderer in &self.renderers {
            // Check if renderer supports this diagram type
            if !renderer.supports(diagram_type) {
                continue;
            }

            // Check if renderer supports this format
            if !renderer.supports_format(format) {
                continue;
            }

            // Check if renderer is available
            if !renderer.is_available() {
                log::debug!("Renderer {} is not available, skipping", renderer.name());
                continue;
            }

            // Try to render
            match renderer.render(source, diagram_type, format, options) {
                Ok(data) => {
                    log::debug!(
                        "Rendered {:?} diagram with {} ({} bytes)",
                        diagram_type,
                        renderer.name(),
                        data.len()
                    );
                    return Ok(data);
                }
                Err(e) => {
                    log::warn!("Renderer {} failed: {}", renderer.name(), e);
                    last_error = Some(e);
                    // Continue to next renderer
                }
            }
        }

        // All renderers failed
        Err(last_error.unwrap_or(RenderError::UnsupportedType(diagram_type)))
    }

    /// Render a diagram and return it with metadata
    pub fn render_with_metadata(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        options: &RenderOptions,
    ) -> RenderResult<RenderedDiagram> {
        let data = self.render(source, diagram_type, format, options)?;
        let source_hash = compute_source_hash(source);
        let renderer = self.find_renderer_name(diagram_type, format);

        Ok(RenderedDiagram::new(
            data,
            diagram_type,
            format,
            source_hash,
            renderer,
        ))
    }

    /// Find the name of the renderer that would handle this request
    fn find_renderer_name(&self, diagram_type: DiagramType, format: OutputFormat) -> String {
        for renderer in &self.renderers {
            if renderer.supports(diagram_type)
                && renderer.supports_format(format)
                && renderer.is_available()
            {
                return renderer.name().to_string();
            }
        }
        "unknown".to_string()
    }

    /// Render to PNG with default options (convenience method)
    pub fn render_png(&self, source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
        self.render(source, diagram_type, OutputFormat::Png, &RenderOptions::default())
    }

    /// Render to SVG with default options (convenience method)
    pub fn render_svg(&self, source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
        self.render(source, diagram_type, OutputFormat::Svg, &RenderOptions::default())
    }
}

/// Compute SHA-256 hash of source for caching/tracking
fn compute_source_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let result = hasher.finalize();
    format!(
        "sha256:{}",
        result.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = DiagramEngine::new();
        let names = engine.renderer_names();

        #[cfg(feature = "native")]
        assert!(names.contains(&"native"));

        // Kroki may or may not be available depending on network
        #[cfg(feature = "kroki")]
        {
            // Just check that kroki feature compiles
            let _ = names.contains(&"kroki");
        }
    }

    #[test]
    fn test_empty_engine() {
        let engine = DiagramEngine::empty();
        assert!(engine.renderer_names().is_empty());

        let result = engine.render(
            "test",
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );
        assert!(matches!(result, Err(RenderError::Unavailable(_))));
    }

    #[test]
    fn test_supported_types() {
        let engine = DiagramEngine::new();
        let types = engine.supported_types();

        #[cfg(feature = "native")]
        assert!(types.contains(&DiagramType::Svgbob));

        #[cfg(feature = "kroki")]
        {
            // Kroki supports many types
            // Just verify the list is not empty
            assert!(!types.is_empty() || engine.renderer_names().is_empty());
        }
    }

    #[test]
    fn test_compute_source_hash() {
        let hash = compute_source_hash("test content");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars

        // Same input = same hash
        let hash2 = compute_source_hash("test content");
        assert_eq!(hash, hash2);

        // Different input = different hash
        let hash3 = compute_source_hash("different content");
        assert_ne!(hash, hash3);
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_render_svgbob_native() {
        let engine = DiagramEngine::new();

        let source = r#"
        +------+
        | Test |
        +------+
        "#;

        let result = engine.render(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(result.is_ok());
        let png = result.unwrap();
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_render_with_metadata() {
        let engine = DiagramEngine::new();

        let source = "+--+";
        let result = engine.render_with_metadata(
            source,
            DiagramType::Svgbob,
            OutputFormat::Png,
            &RenderOptions::default(),
        );

        assert!(result.is_ok());
        let diagram = result.unwrap();
        assert_eq!(diagram.diagram_type, DiagramType::Svgbob);
        assert_eq!(diagram.format, OutputFormat::Png);
        assert!(diagram.source_hash.starts_with("sha256:"));
        assert_eq!(diagram.renderer, "native");
        assert!(diagram.is_valid_png());
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_convenience_methods() {
        let engine = DiagramEngine::new();
        let source = "+--+";

        let png_result = engine.render_png(source, DiagramType::Svgbob);
        assert!(png_result.is_ok());

        let svg_result = engine.render_svg(source, DiagramType::Svgbob);
        assert!(svg_result.is_ok());
    }

    #[test]
    fn test_add_custom_renderer() {
        use crate::renderer::DiagramRenderer;

        struct MockRenderer;

        impl DiagramRenderer for MockRenderer {
            fn name(&self) -> &'static str {
                "mock"
            }

            fn supports(&self, _: DiagramType) -> bool {
                false
            }

            fn render(
                &self,
                _source: &str,
                _diagram_type: DiagramType,
                _format: OutputFormat,
                _options: &RenderOptions,
            ) -> RenderResult<Vec<u8>> {
                Err(RenderError::Unavailable("mock".to_string()))
            }
        }

        let mut engine = DiagramEngine::empty();
        engine.add_renderer(Box::new(MockRenderer));
        assert!(engine.renderer_names().contains(&"mock"));
    }
}
