//! Error types for diagram operations
//!
//! This module provides error types that work across all feature configurations.

use thiserror::Error;

/// Errors that can occur during diagram operations
///
/// This is the legacy error type maintained for backward compatibility.
/// New code should prefer `RenderError` from the `renderer` module.
#[derive(Error, Debug)]
pub enum DiagramError {
    /// Unsupported diagram type
    #[error("Unsupported diagram type: {0}")]
    UnsupportedType(String),

    /// Unsupported output format
    #[error("Unsupported output format: {0}")]
    UnsupportedFormat(String),

    /// HTTP request error (only available with kroki feature)
    #[cfg(feature = "kroki")]
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Diagram rendering failed
    #[error("Rendering failed: {0}")]
    RenderFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid diagram source
    #[error("Invalid diagram source: {0}")]
    InvalidSource(String),

    /// Server returned an error
    #[error("Server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    /// Renderer not available
    #[error("Renderer unavailable: {0}")]
    Unavailable(String),
}

/// Result type for diagram operations
pub type Result<T> = std::result::Result<T, DiagramError>;

/// Convert from RenderError to DiagramError for backward compatibility
impl From<crate::renderer::RenderError> for DiagramError {
    fn from(err: crate::renderer::RenderError) -> Self {
        use crate::renderer::RenderError;

        match err {
            RenderError::UnsupportedType(t) => DiagramError::UnsupportedType(t.to_string()),
            RenderError::UnsupportedFormat(f) => DiagramError::UnsupportedFormat(f.to_string()),
            RenderError::Unavailable(msg) => DiagramError::Unavailable(msg),
            RenderError::InvalidSource(msg) => DiagramError::InvalidSource(msg),
            RenderError::RenderFailed(msg) => DiagramError::RenderFailed(msg),
            RenderError::Panic(msg) => DiagramError::RenderFailed(format!("Panic: {}", msg)),
            RenderError::Network(msg) => DiagramError::RenderFailed(format!("Network: {}", msg)),
            RenderError::Io(e) => DiagramError::Io(e),
        }
    }
}
