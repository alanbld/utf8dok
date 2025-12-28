//! # utf8dok-diagrams
//!
//! Diagram rendering library for utf8dok with pluggable backends.
//!
//! ## Features
//!
//! - **Native Rendering** (`native` feature): Pure-Rust rendering for svgbob diagrams
//! - **Kroki Fallback** (`kroki` feature): Cloud/local rendering for all diagram types
//! - **Fallback Chain**: Automatic fallback from native → cloud
//!
//! ## Quick Start
//!
//! ```ignore
//! use utf8dok_diagrams::{DiagramEngine, DiagramType, OutputFormat, RenderOptions};
//!
//! // Create engine (uses all available renderers)
//! let engine = DiagramEngine::new();
//!
//! // Render svgbob natively (offline, fast)
//! let png = engine.render_png("+---+\n| A |\n+---+", DiagramType::Svgbob)?;
//!
//! // Render Mermaid via Kroki (requires network)
//! let svg = engine.render_svg("graph TD; A-->B;", DiagramType::Mermaid)?;
//! ```
//!
//! ## Feature Flags
//!
//! | Feature | Description | Default |
//! |---------|-------------|---------|
//! | `native` | Native svgbob rendering (offline) | ✓ |
//! | `kroki` | Kroki cloud/local rendering | ✓ |
//! | `minimal` | Native only, no network | |
//! | `full` | All features enabled | |
//!
//! ## Build Configurations
//!
//! ```bash
//! # Default: native + kroki fallback
//! cargo build
//!
//! # Minimal: native only, no network dependencies
//! cargo build --no-default-features --features minimal
//!
//! # Kroki only: no native rendering
//! cargo build --no-default-features --features kroki
//! ```

// Re-export error types (always available)
pub mod error;

// Core types (always available)
pub mod types;

// Renderer trait and types (always available)
pub mod renderer;

// Diagram engine (always available)
pub mod engine;

// Native renderer (requires feature)
#[cfg(feature = "native")]
pub mod native;

// Kroki renderer (requires feature)
#[cfg(feature = "kroki")]
pub mod kroki;

// ============================================================================
// Public API exports
// ============================================================================

// Core types
pub use types::{DiagramType, OutputFormat};

// Renderer trait and types
pub use renderer::{DiagramRenderer, RenderError, RenderOptions, RenderResult, RenderedDiagram};

// Engine
pub use engine::DiagramEngine;

// Native renderer
#[cfg(feature = "native")]
pub use native::NativeRenderer;

// Kroki renderer and utilities
#[cfg(feature = "kroki")]
pub use kroki::{content_hash, KrokiRenderer, DEFAULT_KROKI_URL, LOCAL_KROKI_URL};

// Legacy compatibility: re-export KrokiClient as alias
#[cfg(feature = "kroki")]
pub use kroki::KrokiRenderer as KrokiClient;

// Error types from error module (for backward compatibility)
pub use error::{DiagramError, Result};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Convenience functions
// ============================================================================

/// Create a new diagram engine with default configuration
///
/// This is equivalent to `DiagramEngine::new()`.
pub fn create_engine() -> DiagramEngine {
    DiagramEngine::new()
}

/// Render a diagram to PNG using the default engine
///
/// # Example
///
/// ```ignore
/// use utf8dok_diagrams::{render_png, DiagramType};
///
/// let png = render_png("+---+", DiagramType::Svgbob)?;
/// ```
pub fn render_png(source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
    DiagramEngine::new().render_png(source, diagram_type)
}

/// Render a diagram to SVG using the default engine
pub fn render_svg(source: &str, diagram_type: DiagramType) -> RenderResult<Vec<u8>> {
    DiagramEngine::new().render_svg(source, diagram_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_create_engine() {
        let engine = create_engine();
        let _ = engine.renderer_names();
    }

    #[test]
    fn test_diagram_type_parsing() {
        assert_eq!(
            "mermaid".parse::<DiagramType>().unwrap(),
            DiagramType::Mermaid
        );
        assert_eq!(
            "svgbob".parse::<DiagramType>().unwrap(),
            DiagramType::Svgbob
        );
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_render_png_convenience() {
        let result = render_png("+--+", DiagramType::Svgbob);
        assert!(result.is_ok());
    }

    #[cfg(feature = "native")]
    #[test]
    fn test_render_svg_convenience() {
        let result = render_svg("+--+", DiagramType::Svgbob);
        assert!(result.is_ok());
    }
}
