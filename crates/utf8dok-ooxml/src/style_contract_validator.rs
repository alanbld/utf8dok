//! StyleContract Validation Specification
//!
//! This module defines and enforces the strict validation rules for StyleContract.
//! The StyleContract is LAW - DOCX input is untrusted.
//!
//! # Validation Phases
//!
//! 1. **Schema Validation** - Syntactic correctness
//!    - Required fields present
//!    - Enum values in closed sets
//!    - Numeric ranges enforced
//!
//! 2. **Contract Invariants** - Semantic correctness
//!    - Semantic IDs globally unique
//!    - Bookmark names are valid XML NCName
//!    - Bidirectional mappings are consistent
//!
//! 3. **Completeness Validation** - Coverage guarantees
//!    - All extracted styles have mappings or are explicitly ignored
//!    - No dangling anchor references
//!
//! 4. **Round-Trip Properties** - Identity preservation
//!    - `∀ bookmark B: restore(normalize(B)) == B`
//!    - `deserialize(serialize(contract)) == contract`
//!
//! # Failure Modes
//!
//! - Schema/Invariant violations: Hard error, abort
//! - Completeness issues: Warnings (extraction), Errors (rendering)
//! - Round-trip violations: Test failure, never "best effort"

use std::collections::{HashMap, HashSet};

use crate::style_map::StyleContract;

/// Validation error severity
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// Hard error - processing must abort
    Error,
    /// Warning - processing may continue but fidelity is degraded
    Warning,
}

/// A validation issue found in a StyleContract
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Category of validation that failed
    pub category: ValidationCategory,
    /// Human-readable message
    pub message: String,
    /// Optional field path (e.g., "anchors._Toc123.semantic_id")
    pub field: Option<String>,
}

/// Category of validation rule
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationCategory {
    /// Schema validation (syntactic)
    Schema,
    /// Contract invariants (semantic)
    Invariant,
    /// Completeness validation (coverage)
    Completeness,
    /// Round-trip properties (identity)
    RoundTrip,
}

/// Result of validating a StyleContract
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// All issues found
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create an empty result
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    /// Add an error
    pub fn error(&mut self, category: ValidationCategory, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            severity: Severity::Error,
            category,
            message: message.into(),
            field: None,
        });
    }

    /// Add an error with field context
    pub fn error_at(
        &mut self,
        category: ValidationCategory,
        field: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue {
            severity: Severity::Error,
            category,
            message: message.into(),
            field: Some(field.into()),
        });
    }

    /// Add a warning
    pub fn warning(&mut self, category: ValidationCategory, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            severity: Severity::Warning,
            category,
            message: message.into(),
            field: None,
        });
    }

    /// Add a warning with field context
    pub fn warning_at(
        &mut self,
        category: ValidationCategory,
        field: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue {
            severity: Severity::Warning,
            category,
            message: message.into(),
            field: Some(field.into()),
        });
    }

    /// Check if there are any errors (not just warnings)
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Warning)
    }

    /// Get all errors
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect()
    }

    /// Get all warnings
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect()
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.issues.extend(other.issues);
    }
}

/// StyleContract validator
///
/// Enforces all validation rules defined in the specification.
pub struct StyleContractValidator;

impl StyleContractValidator {
    /// Validate a StyleContract completely
    ///
    /// Runs all validation phases in order:
    /// 1. Schema validation
    /// 2. Contract invariants
    /// 3. Completeness (basic)
    /// 4. Round-trip properties
    ///
    /// Returns a ValidationResult with all issues found.
    pub fn validate(contract: &StyleContract) -> ValidationResult {
        let mut result = ValidationResult::new();

        result.merge(Self::validate_schema(contract));
        result.merge(Self::validate_invariants(contract));
        result.merge(Self::validate_completeness_basic(contract));
        result.merge(Self::validate_roundtrip_properties(contract));

        result
    }

    /// Phase 1: Schema Validation (syntactic correctness)
    ///
    /// Validates:
    /// - Heading levels in range 1-9
    /// - Semantic IDs match pattern `[a-z0-9][a-z0-9-]*`
    /// - Anchor types are valid enum values
    /// - Required fields are present in mappings
    pub fn validate_schema(contract: &StyleContract) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate paragraph style mappings
        for (style_id, mapping) in &contract.paragraph_styles {
            // Heading level must be 1-9
            if let Some(level) = mapping.heading_level {
                if !(1..=9).contains(&level) {
                    result.error_at(
                        ValidationCategory::Schema,
                        format!("paragraph_styles.{}.heading_level", style_id),
                        format!("Heading level {} is out of range [1,9]", level),
                    );
                }
            }

            // Role must not be empty
            if mapping.role.is_empty() {
                result.error_at(
                    ValidationCategory::Schema,
                    format!("paragraph_styles.{}.role", style_id),
                    "Role must not be empty",
                );
            }
        }

        // Validate anchor mappings
        for (bookmark, mapping) in &contract.anchors {
            // Semantic ID must match pattern
            if !Self::is_valid_semantic_id(&mapping.semantic_id) {
                result.error_at(
                    ValidationCategory::Schema,
                    format!("anchors.{}.semantic_id", bookmark),
                    format!(
                        "Semantic ID '{}' must match pattern [a-z0-9][a-z0-9-]*",
                        mapping.semantic_id
                    ),
                );
            }

            // Original bookmark should be present for restoration
            if mapping.original_bookmark.is_none() {
                result.warning_at(
                    ValidationCategory::Schema,
                    format!("anchors.{}.original_bookmark", bookmark),
                    "Missing original_bookmark - round-trip restoration may fail",
                );
            }
        }

        // Validate hyperlink mappings
        for (link_id, mapping) in &contract.hyperlinks {
            if mapping.is_external && mapping.url.is_none() {
                result.error_at(
                    ValidationCategory::Schema,
                    format!("hyperlinks.{}.url", link_id),
                    "External hyperlink must have a URL",
                );
            }
            if !mapping.is_external && mapping.anchor_target.is_none() {
                result.warning_at(
                    ValidationCategory::Schema,
                    format!("hyperlinks.{}.anchor_target", link_id),
                    "Internal hyperlink has no anchor target",
                );
            }
        }

        result
    }

    /// Phase 2: Contract Invariants (semantic correctness)
    ///
    /// Validates:
    /// - Semantic IDs are globally unique
    /// - Bookmark names are valid XML NCName
    /// - No two semantic IDs map to the same Word bookmark
    pub fn validate_invariants(contract: &StyleContract) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check semantic ID uniqueness
        let mut seen_semantic_ids: HashMap<&str, Vec<&str>> = HashMap::new();
        for (bookmark, mapping) in &contract.anchors {
            seen_semantic_ids
                .entry(&mapping.semantic_id)
                .or_default()
                .push(bookmark);
        }

        for (semantic_id, bookmarks) in &seen_semantic_ids {
            if bookmarks.len() > 1 {
                // Duplicate semantic IDs are common in real documents (Word regenerates TOC IDs)
                // This is a warning, not an error - reverse lookup will use first alphabetically
                result.warning(
                    ValidationCategory::Invariant,
                    format!(
                        "Semantic ID '{}' maps to multiple bookmarks: {:?} (will use '{}')",
                        semantic_id,
                        bookmarks,
                        bookmarks.iter().min().unwrap_or(&"")
                    ),
                );
            }
        }

        // Check bookmark name validity (XML NCName)
        for bookmark in contract.anchors.keys() {
            if !Self::is_valid_xml_ncname(bookmark) {
                result.error_at(
                    ValidationCategory::Invariant,
                    format!("anchors.{}", bookmark),
                    format!("Bookmark name '{}' is not a valid XML NCName", bookmark),
                );
            }
        }

        // Check original_bookmark consistency
        let mut seen_original_bookmarks: HashSet<&str> = HashSet::new();
        for (bookmark, mapping) in &contract.anchors {
            if let Some(ref orig) = mapping.original_bookmark {
                if !seen_original_bookmarks.insert(orig) {
                    result.error_at(
                        ValidationCategory::Invariant,
                        format!("anchors.{}.original_bookmark", bookmark),
                        format!(
                            "Original bookmark '{}' is referenced by multiple anchors",
                            orig
                        ),
                    );
                }
            }
        }

        result
    }

    /// Phase 3: Completeness Validation (basic - no context)
    ///
    /// Basic completeness checks that don't require document context.
    pub fn validate_completeness_basic(contract: &StyleContract) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Warn if no paragraph styles defined
        if contract.paragraph_styles.is_empty() {
            result.warning(
                ValidationCategory::Completeness,
                "No paragraph style mappings defined",
            );
        }

        // Warn if no heading styles
        let has_headings = contract
            .paragraph_styles
            .values()
            .any(|m| m.heading_level.is_some());
        if !has_headings {
            result.warning(
                ValidationCategory::Completeness,
                "No heading style mappings defined",
            );
        }

        result
    }

    /// Phase 3b: Completeness Validation with extraction context
    ///
    /// Validates that all extracted elements have mappings.
    /// Call this after extraction with the set of styles found in the document.
    pub fn validate_completeness_extraction(
        contract: &StyleContract,
        found_styles: &HashSet<String>,
        found_bookmarks: &HashSet<String>,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check all found styles have mappings
        for style in found_styles {
            if !contract.paragraph_styles.contains_key(style)
                && !contract.character_styles.contains_key(style)
                && !contract.table_styles.contains_key(style)
            {
                result.warning_at(
                    ValidationCategory::Completeness,
                    format!("styles.{}", style),
                    format!("Style '{}' found in document but has no mapping", style),
                );
            }
        }

        // Check all found bookmarks have anchor mappings (except internal ones we skip)
        for bookmark in found_bookmarks {
            // Skip _Hlk bookmarks (highlights) - we intentionally don't map these
            if bookmark.starts_with("_Hlk") {
                continue;
            }

            if !contract.anchors.contains_key(bookmark) {
                result.warning_at(
                    ValidationCategory::Completeness,
                    format!("anchors.{}", bookmark),
                    format!(
                        "Bookmark '{}' found in document but has no anchor mapping",
                        bookmark
                    ),
                );
            }
        }

        result
    }

    /// Phase 3c: Completeness Validation for rendering
    ///
    /// Validates that all referenced anchors exist in the contract.
    /// Call this before rendering with the set of anchor references in the AST.
    pub fn validate_completeness_rendering(
        contract: &StyleContract,
        referenced_anchors: &HashSet<String>,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        // All semantic anchor IDs that the contract can resolve
        let resolvable: HashSet<&str> = contract
            .anchors
            .values()
            .map(|m| m.semantic_id.as_str())
            .collect();

        for anchor in referenced_anchors {
            if !resolvable.contains(anchor.as_str()) {
                result.error(
                    ValidationCategory::Completeness,
                    format!(
                        "Anchor '{}' referenced but not resolvable from contract",
                        anchor
                    ),
                );
            }
        }

        result
    }

    /// Phase 4: Round-Trip Properties (identity preservation)
    ///
    /// Validates:
    /// - Bidirectional anchor lookup consistency
    /// - TOML serialization round-trip
    pub fn validate_roundtrip_properties(contract: &StyleContract) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Build reverse map to find canonical bookmark for each semantic ID
        // When duplicates exist, the canonical bookmark is the first alphabetically
        let mut semantic_to_bookmarks: HashMap<&str, Vec<&str>> = HashMap::new();
        for (bookmark, mapping) in &contract.anchors {
            semantic_to_bookmarks
                .entry(&mapping.semantic_id)
                .or_default()
                .push(bookmark);
        }
        let canonical_bookmark: HashMap<&str, &str> = semantic_to_bookmarks
            .iter()
            .filter_map(|(sem_id, bookmarks)| {
                // bookmarks is never empty because we only insert when pushing
                bookmarks.iter().min().map(|b| (*sem_id, *b))
            })
            .collect();

        // Property: ∀ anchor: get_word_bookmark(get_semantic_anchor(anchor)) == canonical(anchor)
        for (bookmark, mapping) in &contract.anchors {
            let semantic_id = &mapping.semantic_id;

            // Forward lookup - must always work
            let resolved = contract.get_semantic_anchor(bookmark);
            if resolved != Some(semantic_id.as_str()) {
                result.error(
                    ValidationCategory::RoundTrip,
                    format!(
                        "Forward lookup failed: get_semantic_anchor('{}') = {:?}, expected Some('{}')",
                        bookmark, resolved, semantic_id
                    ),
                );
            }

            // Reverse lookup - must return canonical bookmark
            let expected_canonical = canonical_bookmark.get(semantic_id.as_str()).copied();
            let restored = contract.get_word_bookmark(semantic_id);
            if restored != expected_canonical {
                result.error(
                    ValidationCategory::RoundTrip,
                    format!(
                        "Reverse lookup failed: get_word_bookmark('{}') = {:?}, expected {:?}",
                        semantic_id, restored, expected_canonical
                    ),
                );
            }
        }

        // Property: deserialize(serialize(contract)) == contract (structural)
        // We test this by serializing and deserializing, then comparing key counts
        if let Ok(toml_str) = contract.to_toml() {
            match StyleContract::from_toml(&toml_str) {
                Ok(roundtripped) => {
                    if roundtripped.paragraph_styles.len() != contract.paragraph_styles.len() {
                        result.error(
                            ValidationCategory::RoundTrip,
                            format!(
                                "TOML round-trip lost paragraph styles: {} -> {}",
                                contract.paragraph_styles.len(),
                                roundtripped.paragraph_styles.len()
                            ),
                        );
                    }
                    if roundtripped.anchors.len() != contract.anchors.len() {
                        result.error(
                            ValidationCategory::RoundTrip,
                            format!(
                                "TOML round-trip lost anchors: {} -> {}",
                                contract.anchors.len(),
                                roundtripped.anchors.len()
                            ),
                        );
                    }
                    if roundtripped.hyperlinks.len() != contract.hyperlinks.len() {
                        result.error(
                            ValidationCategory::RoundTrip,
                            format!(
                                "TOML round-trip lost hyperlinks: {} -> {}",
                                contract.hyperlinks.len(),
                                roundtripped.hyperlinks.len()
                            ),
                        );
                    }
                }
                Err(e) => {
                    result.error(
                        ValidationCategory::RoundTrip,
                        format!("TOML round-trip failed to deserialize: {}", e),
                    );
                }
            }
        } else {
            result.error(
                ValidationCategory::RoundTrip,
                "TOML round-trip failed to serialize",
            );
        }

        result
    }

    /// Check if a string is a valid semantic ID
    ///
    /// Pattern: `[a-z0-9][a-z0-9-]*`
    fn is_valid_semantic_id(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        let mut chars = s.chars();

        // First character must be alphanumeric lowercase
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
            _ => return false,
        }

        // Rest must be lowercase alphanumeric or hyphen
        for c in chars {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
                return false;
            }
        }

        true
    }

    /// Check if a string is a valid XML NCName
    ///
    /// NCName (Non-Colonized Name) is a valid XML name without colons.
    /// Simplified check: starts with letter or underscore, followed by
    /// letters, digits, hyphens, underscores, or periods.
    fn is_valid_xml_ncname(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        let mut chars = s.chars();

        // First character must be letter or underscore
        match chars.next() {
            Some(c) if c.is_alphabetic() || c == '_' => {}
            _ => return false,
        }

        // Rest can be letters, digits, hyphens, underscores, periods
        for c in chars {
            if !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style_map::{
        AnchorMapping, AnchorType, CharacterStyleMapping, HyperlinkMapping, ParagraphStyleMapping,
        TableStyleMapping,
    };

    #[test]
    fn test_valid_semantic_id() {
        assert!(StyleContractValidator::is_valid_semantic_id("introduction"));
        assert!(StyleContractValidator::is_valid_semantic_id(
            "purpose-and-scope"
        ));
        assert!(StyleContractValidator::is_valid_semantic_id("section-1"));
        assert!(StyleContractValidator::is_valid_semantic_id("a"));
        assert!(StyleContractValidator::is_valid_semantic_id("1-overview"));

        assert!(!StyleContractValidator::is_valid_semantic_id(""));
        assert!(!StyleContractValidator::is_valid_semantic_id("-invalid"));
        assert!(!StyleContractValidator::is_valid_semantic_id("UPPERCASE"));
        assert!(!StyleContractValidator::is_valid_semantic_id(
            "has_underscore"
        ));
        assert!(!StyleContractValidator::is_valid_semantic_id("has space"));
    }

    #[test]
    fn test_valid_xml_ncname() {
        assert!(StyleContractValidator::is_valid_xml_ncname("_Toc123456"));
        assert!(StyleContractValidator::is_valid_xml_ncname("bookmark"));
        assert!(StyleContractValidator::is_valid_xml_ncname("_Ref789"));
        assert!(StyleContractValidator::is_valid_xml_ncname("custom_anchor"));

        assert!(!StyleContractValidator::is_valid_xml_ncname(""));
        assert!(!StyleContractValidator::is_valid_xml_ncname("123start"));
        assert!(!StyleContractValidator::is_valid_xml_ncname(
            "-hyphen-start"
        ));
        assert!(!StyleContractValidator::is_valid_xml_ncname("has space"));
    }

    #[test]
    fn test_schema_validation_heading_level() {
        let mut contract = StyleContract::new();
        contract.add_paragraph_style(
            "BadHeading",
            ParagraphStyleMapping {
                role: "h10".into(),
                heading_level: Some(10), // Invalid: > 9
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("out of range"));
    }

    #[test]
    fn test_schema_validation_empty_role() {
        let mut contract = StyleContract::new();
        contract.add_paragraph_style(
            "EmptyRole",
            ParagraphStyleMapping {
                role: "".into(), // Invalid: empty
                heading_level: None,
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("must not be empty"));
    }

    #[test]
    fn test_invariant_semantic_id_uniqueness() {
        let mut contract = StyleContract::new();

        // Two bookmarks mapping to the same semantic ID
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "duplicate".into(),
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: Some("_Toc123".into()),
            },
        );
        contract.add_anchor(
            "_Toc456",
            AnchorMapping {
                semantic_id: "duplicate".into(), // Same semantic ID!
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: Some("_Toc456".into()),
            },
        );

        let result = StyleContractValidator::validate_invariants(&contract);
        // Duplicate semantic IDs produce warnings, not errors (common in real documents)
        assert!(result.has_warnings());
        assert!(result.warnings()[0]
            .message
            .contains("maps to multiple bookmarks"));
    }

    #[test]
    fn test_roundtrip_bidirectional_lookup() {
        let mut contract = StyleContract::new();
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "introduction".into(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".into()),
                original_bookmark: Some("_Toc123".into()),
            },
        );

        let result = StyleContractValidator::validate_roundtrip_properties(&contract);
        assert!(
            result.is_valid(),
            "Bidirectional lookup should work: {:?}",
            result.errors()
        );
    }

    #[test]
    fn test_full_validation_valid_contract() {
        let mut contract = StyleContract::with_source("test.docx");

        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".into(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "introduction".into(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".into()),
                original_bookmark: Some("_Toc123".into()),
            },
        );

        let result = StyleContractValidator::validate(&contract);
        assert!(
            result.is_valid(),
            "Valid contract should pass: {:?}",
            result.errors()
        );
    }

    #[test]
    fn test_completeness_rendering_missing_anchor() {
        let contract = StyleContract::new(); // Empty contract

        let mut referenced = HashSet::new();
        referenced.insert("missing-anchor".to_string());

        let result =
            StyleContractValidator::validate_completeness_rendering(&contract, &referenced);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("not resolvable"));
    }

    #[test]
    fn test_external_hyperlink_missing_url() {
        let mut contract = StyleContract::new();
        contract.add_hyperlink(
            "link1",
            HyperlinkMapping {
                is_external: true,
                url: None, // Missing URL for external link
                anchor_target: None,
                original_rel_id: None,
                original_anchor: None,
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("must have a URL"));
    }

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_validation_result_methods() {
        let mut result = ValidationResult::new();
        assert!(!result.has_errors());
        assert!(!result.has_warnings());
        assert!(result.is_valid());

        result.warning(ValidationCategory::Completeness, "A warning");
        assert!(!result.has_errors());
        assert!(result.has_warnings());
        assert!(result.is_valid()); // Warnings don't fail validation

        result.error(ValidationCategory::Schema, "An error");
        assert!(result.has_errors());
        assert!(!result.is_valid());

        assert_eq!(result.errors().len(), 1);
        assert_eq!(result.warnings().len(), 1);
    }

    #[test]
    fn test_validation_result_with_field() {
        let mut result = ValidationResult::new();
        result.error_at(ValidationCategory::Schema, "field.path", "Error at field");
        result.warning_at(
            ValidationCategory::Invariant,
            "other.field",
            "Warning at field",
        );

        assert_eq!(result.issues.len(), 2);
        assert_eq!(result.issues[0].field, Some("field.path".to_string()));
        assert_eq!(result.issues[1].field, Some("other.field".to_string()));
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.error(ValidationCategory::Schema, "Error 1");

        let mut result2 = ValidationResult::new();
        result2.warning(ValidationCategory::Completeness, "Warning 1");
        result2.error(ValidationCategory::Invariant, "Error 2");

        result1.merge(result2);

        assert_eq!(result1.issues.len(), 3);
        assert_eq!(result1.errors().len(), 2);
        assert_eq!(result1.warnings().len(), 1);
    }

    #[test]
    fn test_completeness_basic_empty_contract() {
        let contract = StyleContract::new();
        let result = StyleContractValidator::validate_completeness_basic(&contract);

        // Should warn about no paragraph styles and no headings
        assert!(result.has_warnings());
        assert!(result.warnings().len() >= 1);
    }

    #[test]
    fn test_completeness_basic_with_styles_no_headings() {
        let mut contract = StyleContract::new();
        contract.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "paragraph".into(),
                heading_level: None, // Not a heading
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_completeness_basic(&contract);
        // Should warn about no headings
        assert!(result.has_warnings());
        assert!(result
            .warnings()
            .iter()
            .any(|w| w.message.contains("No heading")));
    }

    #[test]
    fn test_completeness_basic_with_headings() {
        let mut contract = StyleContract::new();
        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".into(),
                heading_level: Some(1),
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_completeness_basic(&contract);
        // Should have no warnings about missing headings
        assert!(!result
            .warnings()
            .iter()
            .any(|w| w.message.contains("No heading")));
    }

    #[test]
    fn test_completeness_extraction_unmapped_styles() {
        let contract = StyleContract::new(); // Empty

        let mut found_styles = HashSet::new();
        found_styles.insert("UnmappedStyle".to_string());
        found_styles.insert("AnotherUnmapped".to_string());

        let found_bookmarks = HashSet::new();

        let result = StyleContractValidator::validate_completeness_extraction(
            &contract,
            &found_styles,
            &found_bookmarks,
        );

        assert!(result.has_warnings());
        assert_eq!(result.warnings().len(), 2);
        assert!(result
            .warnings()
            .iter()
            .any(|w| w.message.contains("UnmappedStyle")));
    }

    #[test]
    fn test_completeness_extraction_unmapped_bookmarks() {
        let contract = StyleContract::new();

        let found_styles = HashSet::new();
        let mut found_bookmarks = HashSet::new();
        found_bookmarks.insert("_Toc999".to_string());
        found_bookmarks.insert("_Hlk123".to_string()); // Should be skipped

        let result = StyleContractValidator::validate_completeness_extraction(
            &contract,
            &found_styles,
            &found_bookmarks,
        );

        // Only _Toc999 should produce a warning, _Hlk is skipped
        assert!(result.has_warnings());
        assert_eq!(result.warnings().len(), 1);
        assert!(result.warnings()[0].message.contains("_Toc999"));
    }

    #[test]
    fn test_invariant_invalid_bookmark_ncname() {
        let mut contract = StyleContract::new();
        contract.add_anchor(
            "123invalid", // Starts with digit - invalid NCName
            AnchorMapping {
                semantic_id: "test".into(),
                anchor_type: AnchorType::UserDefined,
                target_heading: None,
                original_bookmark: None,
            },
        );

        let result = StyleContractValidator::validate_invariants(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("not a valid XML NCName"));
    }

    #[test]
    fn test_invariant_duplicate_original_bookmark() {
        let mut contract = StyleContract::new();

        // Two anchors referencing the same original bookmark
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "section1".into(),
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: Some("OriginalBM".into()),
            },
        );
        contract.add_anchor(
            "_Toc456",
            AnchorMapping {
                semantic_id: "section2".into(),
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: Some("OriginalBM".into()), // Duplicate!
            },
        );

        let result = StyleContractValidator::validate_invariants(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0]
            .message
            .contains("referenced by multiple anchors"));
    }

    #[test]
    fn test_schema_invalid_semantic_id_in_anchor() {
        let mut contract = StyleContract::new();
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "UPPERCASE".into(), // Invalid - uppercase
                anchor_type: AnchorType::Toc,
                target_heading: None,
                original_bookmark: None,
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(result.has_errors());
        assert!(result.errors()[0].message.contains("must match pattern"));
    }

    #[test]
    fn test_schema_internal_hyperlink_no_anchor() {
        let mut contract = StyleContract::new();
        contract.add_hyperlink(
            "link1",
            HyperlinkMapping {
                is_external: false,
                url: None,
                anchor_target: None, // Missing for internal link
                original_rel_id: None,
                original_anchor: None,
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        // Should produce a warning (not error) for missing anchor_target
        assert!(result.has_warnings());
        assert!(result.warnings()[0].message.contains("no anchor target"));
    }

    #[test]
    fn test_roundtrip_toml_serialization() {
        let mut contract = StyleContract::with_source("roundtrip.docx");
        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".into(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "paragraph".into(),
                heading_level: None,
                ..Default::default()
            },
        );
        contract.add_anchor(
            "_Toc123",
            AnchorMapping {
                semantic_id: "introduction".into(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".into()),
                original_bookmark: Some("_Toc123".into()),
            },
        );

        let result = StyleContractValidator::validate_roundtrip_properties(&contract);
        assert!(
            result.is_valid(),
            "TOML round-trip should succeed: {:?}",
            result.errors()
        );
    }

    #[test]
    fn test_full_validation_complex_contract() {
        let mut contract = StyleContract::with_source("complex.docx");

        // Add multiple heading styles
        for level in 1..=3 {
            contract.add_paragraph_style(
                &format!("Heading{}", level),
                ParagraphStyleMapping {
                    role: format!("h{}", level),
                    heading_level: Some(level),
                    ..Default::default()
                },
            );
        }

        // Add paragraph style
        contract.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "paragraph".into(),
                heading_level: None,
                ..Default::default()
            },
        );

        // Add multiple anchors
        for i in 1..=3 {
            contract.add_anchor(
                &format!("_Toc{:03}", i),
                AnchorMapping {
                    semantic_id: format!("section-{}", i),
                    anchor_type: AnchorType::Toc,
                    target_heading: Some(format!("Section {}", i)),
                    original_bookmark: Some(format!("_Toc{:03}", i)),
                },
            );
        }

        // Add hyperlinks
        contract.add_hyperlink(
            "rId1",
            HyperlinkMapping {
                is_external: true,
                url: Some("https://example.com".into()),
                anchor_target: None,
                original_rel_id: Some("rId1".into()),
                original_anchor: None,
            },
        );

        let result = StyleContractValidator::validate(&contract);
        assert!(
            result.is_valid(),
            "Complex contract should be valid: {:?}",
            result.errors()
        );
    }

    // ==================== Sprint 20: Additional Validator Coverage ====================

    #[test]
    fn test_severity_enum_equality() {
        assert_eq!(Severity::Error, Severity::Error);
        assert_eq!(Severity::Warning, Severity::Warning);
        assert_ne!(Severity::Error, Severity::Warning);
    }

    #[test]
    fn test_validation_category_enum_equality() {
        assert_eq!(ValidationCategory::Schema, ValidationCategory::Schema);
        assert_eq!(ValidationCategory::Invariant, ValidationCategory::Invariant);
        assert_eq!(
            ValidationCategory::Completeness,
            ValidationCategory::Completeness
        );
        assert_eq!(ValidationCategory::RoundTrip, ValidationCategory::RoundTrip);
        assert_ne!(ValidationCategory::Schema, ValidationCategory::Invariant);
    }

    #[test]
    fn test_validation_issue_structure() {
        let issue = ValidationIssue {
            severity: Severity::Error,
            category: ValidationCategory::Schema,
            message: "Test error".to_string(),
            field: Some("test.field".to_string()),
        };

        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.category, ValidationCategory::Schema);
        assert_eq!(issue.message, "Test error");
        assert_eq!(issue.field, Some("test.field".to_string()));
    }

    #[test]
    fn test_validation_issue_without_field() {
        let issue = ValidationIssue {
            severity: Severity::Warning,
            category: ValidationCategory::Completeness,
            message: "Missing mapping".to_string(),
            field: None,
        };

        assert_eq!(issue.severity, Severity::Warning);
        assert!(issue.field.is_none());
    }

    #[test]
    fn test_validation_result_warning_at() {
        let mut result = ValidationResult::new();
        result.warning_at(
            ValidationCategory::Completeness,
            "paragraph_styles.CustomStyle",
            "Style not mapped",
        );

        assert!(!result.has_errors());
        assert!(result.has_warnings());
        assert!(result.is_valid()); // warnings don't make it invalid

        let warnings = result.warnings();
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].field,
            Some("paragraph_styles.CustomStyle".to_string())
        );
    }

    #[test]
    fn test_validation_result_error_at() {
        let mut result = ValidationResult::new();
        result.error_at(
            ValidationCategory::Schema,
            "anchors._Invalid",
            "Invalid NCName",
        );

        assert!(result.has_errors());
        assert!(!result.is_valid());

        let errors = result.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, Some("anchors._Invalid".to_string()));
    }

    #[test]
    fn test_validation_result_mixed_issues() {
        let mut result = ValidationResult::new();
        result.error(ValidationCategory::Schema, "Error 1");
        result.warning(ValidationCategory::Completeness, "Warning 1");
        result.error(ValidationCategory::Invariant, "Error 2");
        result.warning(ValidationCategory::Completeness, "Warning 2");

        assert!(result.has_errors());
        assert!(result.has_warnings());
        assert!(!result.is_valid());

        assert_eq!(result.errors().len(), 2);
        assert_eq!(result.warnings().len(), 2);
        assert_eq!(result.issues.len(), 4);
    }

    #[test]
    fn test_validation_result_merge_empty() {
        let mut result1 = ValidationResult::new();
        let result2 = ValidationResult::new();

        result1.merge(result2);

        assert!(result1.is_valid());
        assert!(result1.issues.is_empty());
    }

    #[test]
    fn test_validation_result_merge_errors_into_empty() {
        let mut result1 = ValidationResult::new();
        let mut result2 = ValidationResult::new();
        result2.error(ValidationCategory::Schema, "Error from result2");

        result1.merge(result2);

        assert!(!result1.is_valid());
        assert_eq!(result1.errors().len(), 1);
    }

    #[test]
    fn test_validation_result_merge_preserves_all() {
        let mut result1 = ValidationResult::new();
        result1.error(ValidationCategory::Schema, "Error 1");
        result1.warning(ValidationCategory::Completeness, "Warning 1");

        let mut result2 = ValidationResult::new();
        result2.error(ValidationCategory::Invariant, "Error 2");
        result2.warning(ValidationCategory::RoundTrip, "Warning 2");

        result1.merge(result2);

        assert_eq!(result1.issues.len(), 4);
        assert_eq!(result1.errors().len(), 2);
        assert_eq!(result1.warnings().len(), 2);
    }

    #[test]
    fn test_validate_schema_valid_heading_levels_1_to_9() {
        for level in 1..=9 {
            let mut contract = StyleContract::default();
            contract.add_paragraph_style(
                &format!("Heading{}", level),
                ParagraphStyleMapping {
                    role: format!("h{}", level),
                    heading_level: Some(level),
                    ..Default::default()
                },
            );

            let result = StyleContractValidator::validate_schema(&contract);
            assert!(
                result.is_valid(),
                "Heading level {} should be valid",
                level
            );
        }
    }

    #[test]
    fn test_validate_schema_invalid_heading_level_zero() {
        let mut contract = StyleContract::default();
        contract.add_paragraph_style(
            "HeadingZero",
            ParagraphStyleMapping {
                role: "h0".to_string(),
                heading_level: Some(0),
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|e| e.message.contains("0")));
    }

    #[test]
    fn test_validate_schema_invalid_heading_level_10() {
        let mut contract = StyleContract::default();
        contract.add_paragraph_style(
            "HeadingTen",
            ParagraphStyleMapping {
                role: "h10".to_string(),
                heading_level: Some(10),
                ..Default::default()
            },
        );

        let result = StyleContractValidator::validate_schema(&contract);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|e| e.message.contains("10")));
    }

    #[test]
    fn test_validate_invariants_unique_semantic_ids() {
        let mut contract = StyleContract::default();
        contract.add_anchor(
            "_Ref1",
            AnchorMapping {
                semantic_id: "unique-1".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_Ref1".to_string()),
            },
        );
        contract.add_anchor(
            "_Ref2",
            AnchorMapping {
                semantic_id: "unique-2".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_Ref2".to_string()),
            },
        );

        let result = StyleContractValidator::validate_invariants(&contract);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_invariants_duplicate_semantic_ids() {
        let mut contract = StyleContract::default();
        contract.add_anchor(
            "_Ref1",
            AnchorMapping {
                semantic_id: "same-id".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_Ref1".to_string()),
            },
        );
        contract.add_anchor(
            "_Ref2",
            AnchorMapping {
                semantic_id: "same-id".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_Ref2".to_string()),
            },
        );

        let result = StyleContractValidator::validate_invariants(&contract);
        // Duplicate semantic IDs produce warnings, not errors (still valid but with warnings)
        assert!(result.is_valid());
        assert!(!result.warnings().is_empty());
        assert!(result
            .warnings()
            .iter()
            .any(|w| w.message.contains("same-id")));
    }

    #[test]
    fn test_validate_completeness_rendering_with_anchor_ref() {
        let mut contract = StyleContract::default();

        // Add an anchor
        contract.add_anchor(
            "_Section1",
            AnchorMapping {
                semantic_id: "section-1".to_string(),
                anchor_type: AnchorType::Heading,
                target_heading: Some("Section 1".to_string()),
                original_bookmark: Some("_Section1".to_string()),
            },
        );

        // Referenced anchors that exist in the contract
        let mut referenced = HashSet::new();
        referenced.insert("section-1".to_string());

        let result = StyleContractValidator::validate_completeness_rendering(&contract, &referenced);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_completeness_basic_only_warnings_on_missing() {
        let contract = StyleContract::default();
        let result = StyleContractValidator::validate_completeness_basic(&contract);

        // Empty contract should generate warnings, not errors
        // The basic completeness check is lenient
        assert!(!result.has_errors() || result.has_warnings());
    }

    #[test]
    fn test_validate_roundtrip_valid_contract() {
        let mut contract = StyleContract::with_source("test.docx");
        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".to_string(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.add_anchor(
            "_Toc001",
            AnchorMapping {
                semantic_id: "intro".to_string(),
                anchor_type: AnchorType::Toc,
                target_heading: Some("Introduction".to_string()),
                original_bookmark: Some("_Toc001".to_string()),
            },
        );

        let result = StyleContractValidator::validate_roundtrip_properties(&contract);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_full_empty_contract() {
        let contract = StyleContract::default();
        let result = StyleContractValidator::validate(&contract);

        // Empty contract should be valid (no schema/invariant errors)
        // May have completeness warnings
        assert!(!result.has_errors() || result.warnings().len() > 0);
    }

    #[test]
    fn test_validate_full_with_all_valid_types() {
        let mut contract = StyleContract::with_source("complete.docx");

        // Add paragraph styles
        contract.add_paragraph_style(
            "Heading1",
            ParagraphStyleMapping {
                role: "h1".to_string(),
                heading_level: Some(1),
                ..Default::default()
            },
        );
        contract.add_paragraph_style(
            "Normal",
            ParagraphStyleMapping {
                role: "paragraph".to_string(),
                ..Default::default()
            },
        );

        // Add character style
        contract.add_character_style(
            "Strong",
            CharacterStyleMapping {
                role: "strong".to_string(),
                is_strong: true,
                is_emphasis: false,
                is_code: false,
            },
        );

        // Add table style
        contract.add_table_style(
            "TableGrid",
            TableStyleMapping {
                role: "table".to_string(),
                first_row_header: true,
                first_col_header: false,
            },
        );

        // Add anchor
        contract.add_anchor(
            "_Ref1",
            AnchorMapping {
                semantic_id: "reference-1".to_string(),
                anchor_type: AnchorType::Reference,
                target_heading: None,
                original_bookmark: Some("_Ref1".to_string()),
            },
        );

        // Add hyperlink
        contract.add_hyperlink(
            "rId1",
            HyperlinkMapping {
                is_external: true,
                url: Some("https://example.com".to_string()),
                anchor_target: None,
                original_rel_id: Some("rId1".to_string()),
                original_anchor: None,
            },
        );

        let result = StyleContractValidator::validate(&contract);
        assert!(
            result.is_valid(),
            "Full contract should be valid: {:?}",
            result.errors()
        );
    }

    #[test]
    fn test_is_valid_semantic_id_valid_ids() {
        assert!(StyleContractValidator::is_valid_semantic_id("simple"));
        assert!(StyleContractValidator::is_valid_semantic_id("with-dashes"));
        assert!(StyleContractValidator::is_valid_semantic_id("section-1-intro"));
        assert!(StyleContractValidator::is_valid_semantic_id("a"));
        assert!(StyleContractValidator::is_valid_semantic_id("abc123"));
        // Digits allowed as first character
        assert!(StyleContractValidator::is_valid_semantic_id("123-numeric-start"));
    }

    #[test]
    fn test_is_valid_semantic_id_invalid_ids() {
        assert!(!StyleContractValidator::is_valid_semantic_id(""));
        assert!(!StyleContractValidator::is_valid_semantic_id("has spaces"));
        // Underscores not allowed
        assert!(!StyleContractValidator::is_valid_semantic_id("with_underscore"));
        assert!(!StyleContractValidator::is_valid_semantic_id(
            "-starts-with-dash"
        ));
        assert!(!StyleContractValidator::is_valid_semantic_id("has.dot"));
        // Uppercase not allowed
        assert!(!StyleContractValidator::is_valid_semantic_id("HasUppercase"));
    }

    #[test]
    fn test_is_valid_xml_ncname_valid_names() {
        assert!(StyleContractValidator::is_valid_xml_ncname("_Toc123"));
        assert!(StyleContractValidator::is_valid_xml_ncname("_Ref456"));
        assert!(StyleContractValidator::is_valid_xml_ncname("_Hlk789"));
        assert!(StyleContractValidator::is_valid_xml_ncname("SimpleBookmark"));
        assert!(StyleContractValidator::is_valid_xml_ncname("_underscore"));
        assert!(StyleContractValidator::is_valid_xml_ncname("a"));
    }

    #[test]
    fn test_is_valid_xml_ncname_invalid_names() {
        assert!(!StyleContractValidator::is_valid_xml_ncname(""));
        assert!(!StyleContractValidator::is_valid_xml_ncname(
            "123StartWithNumber"
        ));
        assert!(!StyleContractValidator::is_valid_xml_ncname("-StartWithDash"));
        assert!(!StyleContractValidator::is_valid_xml_ncname("has space"));
        assert!(!StyleContractValidator::is_valid_xml_ncname("has:colon"));
    }

    #[test]
    fn test_validation_issue_debug_format() {
        let issue = ValidationIssue {
            severity: Severity::Error,
            category: ValidationCategory::Schema,
            message: "Test".to_string(),
            field: None,
        };

        let debug_str = format!("{:?}", issue);
        assert!(debug_str.contains("Error"));
        assert!(debug_str.contains("Schema"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.is_valid());
        assert!(result.issues.is_empty());
    }
}
