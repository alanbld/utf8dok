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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Sprint 10: Error Variant Tests ====================

    #[test]
    fn test_missing_file_error_message() {
        let err = OoxmlError::MissingFile("word/document.xml".to_string());
        assert_eq!(
            err.to_string(),
            "Required file not found: word/document.xml"
        );
    }

    #[test]
    fn test_invalid_structure_error_message() {
        let err = OoxmlError::InvalidStructure("Missing body element".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid document structure: Missing body element"
        );
    }

    #[test]
    fn test_style_not_found_error_message() {
        let err = OoxmlError::StyleNotFound("Heading1".to_string());
        assert_eq!(err.to_string(), "Style not found: Heading1");
    }

    #[test]
    fn test_unsupported_error_message() {
        let err = OoxmlError::Unsupported("SmartArt diagrams".to_string());
        assert_eq!(err.to_string(), "Unsupported feature: SmartArt diagrams");
    }

    #[test]
    fn test_other_error_message() {
        let err = OoxmlError::Other("Custom error message".to_string());
        assert_eq!(err.to_string(), "Custom error message");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ooxml_err: OoxmlError = io_err.into();
        assert!(matches!(ooxml_err, OoxmlError::Io(_)));
        assert!(ooxml_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_json_error_conversion() {
        // Create a JSON parsing error
        let json_result: std::result::Result<serde_json::Value, _> =
            serde_json::from_str("invalid json {");
        let json_err = json_result.unwrap_err();
        let ooxml_err: OoxmlError = json_err.into();
        assert!(matches!(ooxml_err, OoxmlError::Json(_)));
        assert!(ooxml_err.to_string().starts_with("JSON error:"));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(OoxmlError::Other("test".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_debug_format() {
        let err = OoxmlError::MissingFile("test.xml".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("MissingFile"));
        assert!(debug_str.contains("test.xml"));
    }

    #[test]
    fn test_all_error_variants_are_distinct() {
        // Ensure each variant produces a unique error message prefix
        let errors = vec![
            OoxmlError::MissingFile("x".to_string()),
            OoxmlError::InvalidStructure("x".to_string()),
            OoxmlError::StyleNotFound("x".to_string()),
            OoxmlError::Unsupported("x".to_string()),
            OoxmlError::Other("x".to_string()),
        ];

        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();

        // Check all messages are unique (except Other which is just "x")
        for (i, msg1) in messages.iter().enumerate() {
            for (j, msg2) in messages.iter().enumerate() {
                if i != j && i < 4 && j < 4 {
                    // Skip "Other" comparison
                    assert_ne!(msg1, msg2, "Error messages should be unique");
                }
            }
        }
    }
}
