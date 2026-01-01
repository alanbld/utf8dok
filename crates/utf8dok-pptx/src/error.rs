//! Error types for PPTX generation.

use thiserror::Error;

/// Result type for PPTX operations
pub type Result<T> = std::result::Result<T, PptxError>;

/// Errors that can occur during PPTX generation
#[derive(Error, Debug)]
pub enum PptxError {
    /// Template file not found or inaccessible
    #[error("Template not found: {path}")]
    TemplateNotFound { path: String },

    /// Template is invalid or corrupted
    #[error("Invalid template: {reason}")]
    InvalidTemplate { reason: String },

    /// SlideContract configuration error
    #[error("SlideContract error: {reason}")]
    ContractError { reason: String },

    /// Invalid layout index in SlideContract
    #[error("Invalid slide layout index {index}: {reason}")]
    InvalidLayoutIndex { index: u32, reason: String },

    /// Missing required placeholder in layout
    #[error("Missing placeholder '{placeholder}' in layout {layout}")]
    MissingPlaceholder { placeholder: String, layout: String },

    /// Image processing error
    #[error("Image error: {reason}")]
    ImageError { reason: String },

    /// XML generation or parsing error
    #[error("XML error: {0}")]
    XmlError(#[from] quick_xml::Error),

    /// ZIP archive error
    #[error("Archive error: {0}")]
    ZipError(#[from] zip::result::ZipError),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// TOML parsing error (for SlideContract)
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Content exceeds slide capacity
    #[error("Content overflow: {reason}")]
    ContentOverflow { reason: String },

    /// Unsupported feature requested
    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },

    /// Speaker notes without slide context
    #[error("Orphan speaker notes at line {line}: notes must follow a slide")]
    OrphanSpeakerNotes { line: usize },

    /// Nested slides block detected
    #[error("Nested [slides] block at line {line}: slides blocks cannot be nested")]
    NestedSlidesBlock { line: usize },
}

impl PptxError {
    /// Create a template not found error
    pub fn template_not_found(path: impl Into<String>) -> Self {
        Self::TemplateNotFound { path: path.into() }
    }

    /// Create an invalid template error
    pub fn invalid_template(reason: impl Into<String>) -> Self {
        Self::InvalidTemplate {
            reason: reason.into(),
        }
    }

    /// Create a contract error
    pub fn contract_error(reason: impl Into<String>) -> Self {
        Self::ContractError {
            reason: reason.into(),
        }
    }

    /// Create an invalid layout index error
    pub fn invalid_layout(index: u32, reason: impl Into<String>) -> Self {
        Self::InvalidLayoutIndex {
            index,
            reason: reason.into(),
        }
    }

    /// Create a missing placeholder error
    pub fn missing_placeholder(placeholder: impl Into<String>, layout: impl Into<String>) -> Self {
        Self::MissingPlaceholder {
            placeholder: placeholder.into(),
            layout: layout.into(),
        }
    }

    /// Create an image error
    pub fn image_error(reason: impl Into<String>) -> Self {
        Self::ImageError {
            reason: reason.into(),
        }
    }

    /// Create a content overflow error
    pub fn content_overflow(reason: impl Into<String>) -> Self {
        Self::ContentOverflow {
            reason: reason.into(),
        }
    }

    /// Create an unsupported feature error
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature {
            feature: feature.into(),
        }
    }

    /// Get the error code for diagnostics
    pub fn code(&self) -> &'static str {
        match self {
            Self::TemplateNotFound { .. } => "PPTX001",
            Self::InvalidTemplate { .. } => "PPTX002",
            Self::ContractError { .. } => "PPTX003",
            Self::InvalidLayoutIndex { .. } => "PPTX004",
            Self::MissingPlaceholder { .. } => "PPTX005",
            Self::ImageError { .. } => "PPTX006",
            Self::XmlError(_) => "PPTX007",
            Self::ZipError(_) => "PPTX008",
            Self::IoError(_) => "PPTX009",
            Self::TomlError(_) => "PPTX010",
            Self::ContentOverflow { .. } => "PPTX011",
            Self::UnsupportedFeature { .. } => "PPTX012",
            Self::OrphanSpeakerNotes { .. } => "PPTX013",
            Self::NestedSlidesBlock { .. } => "PPTX014",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = PptxError::template_not_found("test.potx");
        assert_eq!(err.code(), "PPTX001");
        assert!(err.to_string().contains("test.potx"));

        let err = PptxError::invalid_layout(99, "layout does not exist");
        assert_eq!(err.code(), "PPTX004");
        assert!(err.to_string().contains("99"));
    }

    #[test]
    fn test_error_display() {
        let err = PptxError::missing_placeholder("title", "Title Slide");
        assert!(err.to_string().contains("title"));
        assert!(err.to_string().contains("Title Slide"));

        let err = PptxError::OrphanSpeakerNotes { line: 42 };
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_error_constructors() {
        // Test all constructor helpers compile and work
        let _ = PptxError::template_not_found("path");
        let _ = PptxError::invalid_template("reason");
        let _ = PptxError::contract_error("reason");
        let _ = PptxError::invalid_layout(1, "reason");
        let _ = PptxError::missing_placeholder("ph", "layout");
        let _ = PptxError::image_error("reason");
        let _ = PptxError::content_overflow("reason");
        let _ = PptxError::unsupported("feature");
    }
}
