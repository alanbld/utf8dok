//! Error types for diagram operations

use thiserror::Error;

/// Errors that can occur during diagram operations
#[derive(Error, Debug)]
pub enum DiagramError {
    /// Unsupported diagram type
    #[error("Unsupported diagram type: {0}")]
    UnsupportedType(String),

    /// Unsupported output format
    #[error("Unsupported output format: {0}")]
    UnsupportedFormat(String),

    /// HTTP request error
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
    ServerError {
        status: u16,
        message: String,
    },
}

/// Result type for diagram operations
pub type Result<T> = std::result::Result<T, DiagramError>;
