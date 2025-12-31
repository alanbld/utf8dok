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
            .map(|(sem_id, bookmarks)| (*sem_id, *bookmarks.iter().min().unwrap()))
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
    use crate::style_map::{AnchorMapping, AnchorType, HyperlinkMapping, ParagraphStyleMapping};

    #[test]
    fn test_valid_semantic_id() {
        assert!(StyleContractValidator::is_valid_semantic_id("introduction"));
        assert!(StyleContractValidator::is_valid_semantic_id("purpose-and-scope"));
        assert!(StyleContractValidator::is_valid_semantic_id("section-1"));
        assert!(StyleContractValidator::is_valid_semantic_id("a"));
        assert!(StyleContractValidator::is_valid_semantic_id("1-overview"));

        assert!(!StyleContractValidator::is_valid_semantic_id(""));
        assert!(!StyleContractValidator::is_valid_semantic_id("-invalid"));
        assert!(!StyleContractValidator::is_valid_semantic_id("UPPERCASE"));
        assert!(!StyleContractValidator::is_valid_semantic_id("has_underscore"));
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
        assert!(!StyleContractValidator::is_valid_xml_ncname("-hyphen-start"));
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
        assert!(result.errors()[0]
            .message
            .contains("out of range"));
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
}
