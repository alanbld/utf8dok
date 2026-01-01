//! Error types for the data engine.

use thiserror::Error;

/// Result type for data operations
pub type Result<T> = std::result::Result<T, DataError>;

/// Errors that can occur during data source operations
#[derive(Debug, Error)]
pub enum DataError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Failed to open workbook
    #[error("Failed to open workbook: {0}")]
    WorkbookOpen(String),

    /// Sheet not found in workbook
    #[error("Sheet not found: {0}")]
    SheetNotFound(String),

    /// Invalid range specification
    #[error("Invalid range: {0}")]
    InvalidRange(String),

    /// Range out of bounds
    #[error("Range out of bounds: {0}")]
    RangeOutOfBounds(String),

    /// Unsupported cell type
    #[error("Unsupported cell type at {0}")]
    UnsupportedCellType(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Calamine error
    #[error("Excel error: {0}")]
    Calamine(String),
}

impl From<calamine::Error> for DataError {
    fn from(err: calamine::Error) -> Self {
        DataError::Calamine(err.to_string())
    }
}

impl From<calamine::XlsxError> for DataError {
    fn from(err: calamine::XlsxError) -> Self {
        DataError::Calamine(err.to_string())
    }
}
