//! Error types for PDF generation

use thiserror::Error;

/// Result type for PDF operations
pub type Result<T> = std::result::Result<T, PdfError>;

/// Errors that can occur during PDF generation
#[derive(Error, Debug)]
pub enum PdfError {
    /// Typst compilation error
    #[error("Typst compilation failed: {0}")]
    Compilation(String),

    /// Font loading error
    #[error("Font error: {0}")]
    Font(String),

    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}
