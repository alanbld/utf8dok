//! # utf8dok-diagrams
//!
//! Diagram rendering library for utf8dok, supporting multiple diagram types
//! via the [Kroki](https://kroki.io) service.
//!
//! ## Supported Diagram Types
//!
//! - Mermaid (flowcharts, sequence diagrams, class diagrams, etc.)
//! - PlantUML (UML diagrams)
//! - GraphViz (DOT language)
//! - D2 (declarative diagrams)
//! - And many more...
//!
//! ## Example
//!
//! ```no_run
//! use utf8dok_diagrams::{KrokiClient, DiagramType, OutputFormat};
//!
//! let client = KrokiClient::new();
//!
//! // Render a Mermaid diagram to SVG
//! let svg = client.render(
//!     "graph TD; A-->B; B-->C;",
//!     DiagramType::Mermaid,
//!     OutputFormat::Svg,
//! )?;
//!
//! // Or get a URL for embedding
//! let url = client.diagram_url(
//!     "graph TD; A-->B;",
//!     DiagramType::Mermaid,
//!     OutputFormat::Svg,
//! )?;
//! # Ok::<(), utf8dok_diagrams::DiagramError>(())
//! ```

pub mod error;
pub mod kroki;
pub mod types;

pub use error::{DiagramError, Result};
pub use kroki::{content_hash, KrokiClient, RenderedDiagram, DEFAULT_KROKI_URL};
pub use types::{DiagramType, OutputFormat};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
