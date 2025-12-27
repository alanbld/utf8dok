//! Error types for OOXML operations

use thiserror::Error;

/// Errors that can occur during OOXML operations
#[derive(Error, Debug)]
pub enum OoxmlError {
    /// Error reading or writing the ZIP archive
    #[error("Archive error: {0}")]
    Archive(#[from] zip::result::ZipError),

    /// Error reading or writing files
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing XML content
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Error deserializing XML
    #[error("XML deserialization error: {0}")]
    XmlDeserialize(#[from] quick_xml::DeError),

    /// Required file not found in archive
    #[error("Required file not found: {0}")]
    MissingFile(String),

    /// Invalid document structure
    #[error("Invalid document structure: {0}")]
    InvalidStructure(String),

    /// Style not found
    #[error("Style not found: {0}")]
    StyleNotFound(String),

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

/// Result type for OOXML operations
pub type Result<T> = std::result::Result<T, OoxmlError>;
