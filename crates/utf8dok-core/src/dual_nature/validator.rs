//! Validator for dual-nature document consistency
//!
//! Ensures that slide and document content are consistent and complete.

use super::types::*;

/// Dual-nature document validator
pub struct DualNatureValidator;

impl DualNatureValidator {
    /// Validate a dual-nature document for consistency
    pub fn validate(doc: &DualNatureDocument) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check for orphaned slide-only content (no corresponding document content)
        Self::check_slide_document_balance(doc, &mut result);

        // Check image references exist in both formats
        Self::check_image_consistency(doc, &mut result);

        // Check cross-references resolve in both formats
        Self::check_cross_references(doc, &mut result);

        // Check that slide bullets don't exceed limits
        Self::check_bullet_limits(doc, &mut result);

        // Check for structural completeness
        Self::check_structural_completeness(doc, &mut result);

        result
    }

    /// Check that slide and document content are balanced
    fn check_slide_document_balance(doc: &DualNatureDocument, result: &mut ValidationResult) {
        let slide_only_sections = Self::count_sections_with_selector(doc, ContentSelector::SlideOnly);
        let doc_only_sections = Self::count_sections_with_selector(doc, ContentSelector::DocumentOnly);

        // Warn if there's a large imbalance
        if slide_only_sections > 0 && doc_only_sections == 0 {
            result.add_warning(
                ValidationWarning::new(
                    "DUAL001",
                    "Slide-only sections have no corresponding document sections",
                )
                .with_suggestion("Consider adding [.document-only] sections with detailed content"),
            );
        }

        if doc_only_sections > 0 && slide_only_sections == 0 {
            result.add_info(
                ValidationInfo::new(
                    "DUAL002",
                    "Document-only sections exist without slide summaries",
                )
                .with_suggestion("Consider adding [.slide] sections for executive summaries"),
            );
        }
    }

    /// Count sections with a specific selector
    fn count_sections_with_selector(doc: &DualNatureDocument, selector: ContentSelector) -> usize {
        doc.blocks.iter()
            .filter(|b| b.selector == selector)
            .filter(|b| matches!(b.content, BlockContent::Section(_)))
            .count()
    }

    /// Check image consistency between formats
    fn check_image_consistency(doc: &DualNatureDocument, result: &mut ValidationResult) {
        for block in &doc.blocks {
            if let BlockContent::Image(img) = &block.content {
                // If slide-specific image is specified, it should exist
                if let Some(ref slide_path) = img.slide_path {
                    if *slide_path == img.path {
                        result.add_info(
                            ValidationInfo::new(
                                "DUAL003",
                                format!("slide_path is same as path for image '{}'", img.path),
                            )
                            .at_line(block.source_line),
                        );
                    }
                }

                // Images in slide-only blocks should be optimized for presentations
                if matches!(block.selector, ContentSelector::SlideOnly | ContentSelector::Slide)
                    && img.width.is_none()
                {
                    result.add_warning(
                        ValidationWarning::new(
                            "DUAL004",
                            format!("Slide image '{}' has no width specified", img.path),
                        )
                        .at_line(block.source_line)
                        .with_suggestion("Add width attribute for consistent slide layout"),
                    );
                }
            }
        }
    }

    /// Check cross-references resolve in both formats
    fn check_cross_references(doc: &DualNatureDocument, result: &mut ValidationResult) {
        // Collect all defined IDs
        let defined_ids: Vec<String> = doc.blocks.iter()
            .filter_map(|b| {
                if let BlockContent::Section(section) = &b.content {
                    section.id.clone()
                } else {
                    None
                }
            })
            .collect();

        // Check for references (simplified - in practice we'd parse text for <<id>> patterns)
        for block in &doc.blocks {
            if let BlockContent::Paragraph(text) = &block.content {
                // Look for AsciiDoc xref pattern <<id>>
                for part in text.split("<<") {
                    if let Some(end) = part.find(">>") {
                        let ref_id = &part[..end];
                        let ref_id = ref_id.split(',').next().unwrap_or(ref_id);

                        if !ref_id.is_empty() && !defined_ids.contains(&ref_id.to_string()) {
                            // Check if it's a file reference
                            if !ref_id.contains('.') {
                                result.add_warning(
                                    ValidationWarning::new(
                                        "DUAL005",
                                        format!("Cross-reference '{}' may not be defined", ref_id),
                                    )
                                    .at_line(block.source_line),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check bullet limits are respected
    fn check_bullet_limits(doc: &DualNatureDocument, result: &mut ValidationResult) {
        let default_limit = doc.attributes.slide.default_bullets.unwrap_or(5);

        for block in &doc.blocks {
            // Only check slide-relevant blocks
            if !block.selector.matches_format(OutputFormat::Slide) {
                continue;
            }

            if let BlockContent::BulletList(items) = &block.content {
                let limit = block.overrides.slide_bullets.unwrap_or(default_limit);

                if items.len() > limit {
                    result.add_warning(
                        ValidationWarning::new(
                            "DUAL006",
                            format!(
                                "Bullet list has {} items but slide limit is {}",
                                items.len(),
                                limit
                            ),
                        )
                        .at_line(block.source_line)
                        .with_suggestion("Content will be truncated in slide view"),
                    );
                }
            }
        }
    }

    /// Check structural completeness
    fn check_structural_completeness(doc: &DualNatureDocument, result: &mut ValidationResult) {
        // Check for title
        if doc.title.is_none() {
            result.add_warning(
                ValidationWarning::new(
                    "DUAL007",
                    "Document has no title",
                )
                .with_suggestion("Add a title with '= Document Title'"),
            );
        }

        // Check for slide template if there are slide blocks
        let has_slide_content = doc.blocks.iter()
            .any(|b| matches!(b.selector, ContentSelector::Slide | ContentSelector::SlideOnly));

        if has_slide_content && doc.attributes.slide.template.is_none() {
            result.add_info(
                ValidationInfo::new(
                    "DUAL008",
                    "Slide content exists but no template specified",
                )
                .with_suggestion("Add :template: attribute for consistent branding"),
            );
        }
    }
}

/// Validation result containing errors, warnings, and info
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Validation errors (document cannot be rendered)
    pub errors: Vec<ValidationError>,
    /// Validation warnings (document can render but may have issues)
    pub warnings: Vec<ValidationWarning>,
    /// Informational messages (suggestions for improvement)
    pub info: Vec<ValidationInfo>,
    /// Overall validity
    pub is_valid: bool,
}

impl ValidationResult {
    /// Create a new valid result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
            is_valid: true,
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Add an info message
    pub fn add_info(&mut self, info: ValidationInfo) {
        self.info.push(info);
    }

    /// Check if there are any issues
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    /// Get total issue count
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Source line (if applicable)
    pub line: Option<usize>,
    /// Suggestion for fixing
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            line: None,
            suggestion: None,
        }
    }

    /// Set source line
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// Source line (if applicable)
    pub line: Option<usize>,
    /// Suggestion for fixing
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            line: None,
            suggestion: None,
        }
    }

    /// Set source line
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Validation info message
#[derive(Debug, Clone)]
pub struct ValidationInfo {
    /// Info code
    pub code: String,
    /// Info message
    pub message: String,
    /// Source line (if applicable)
    pub line: Option<usize>,
    /// Suggestion
    pub suggestion: Option<String>,
}

impl ValidationInfo {
    /// Create a new validation info
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            line: None,
            suggestion: None,
        }
    }

    /// Set source line
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_nature::DualNatureParser;

    #[test]
    fn test_validate_balanced_document() {
        let content = r#"= Balanced Document

[.slide]
== Executive Summary
* Key point 1
* Key point 2

[.document]
== Detailed Analysis
Full explanation here.
"#;
        let doc = DualNatureParser::parse(content);
        let result = DualNatureValidator::validate(&doc);

        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_slide_only_warning() {
        let content = r#"= Unbalanced Document

[.slide-only]
== Slides Only
* Point 1
"#;
        let doc = DualNatureParser::parse(content);
        let result = DualNatureValidator::validate(&doc);

        assert!(result.warnings.iter().any(|w| w.code == "DUAL001"));
    }

    #[test]
    fn test_validate_bullet_limit() {
        let content = r#"= Over Limit
:slide-bullets: 3

[.slide]
== Too Many Points
* Point 1
* Point 2
* Point 3
* Point 4
* Point 5
"#;
        let doc = DualNatureParser::parse(content);
        let result = DualNatureValidator::validate(&doc);

        assert!(result.warnings.iter().any(|w| w.code == "DUAL006"));
    }

    #[test]
    fn test_validate_no_title_warning() {
        let content = r#"
== Section Without Title
Content here.
"#;
        let doc = DualNatureParser::parse(content);
        let result = DualNatureValidator::validate(&doc);

        assert!(result.warnings.iter().any(|w| w.code == "DUAL007"));
    }

    #[test]
    fn test_validation_result_helpers() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid);
        assert!(!result.has_issues());

        result.add_warning(ValidationWarning::new("TEST", "test warning"));
        assert!(result.is_valid); // Warnings don't invalidate
        assert!(result.has_issues());

        result.add_error(ValidationError::new("ERR", "test error"));
        assert!(!result.is_valid); // Errors invalidate
        assert_eq!(result.issue_count(), 2);
    }
}
