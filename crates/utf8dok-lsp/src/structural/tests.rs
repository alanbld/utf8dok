//! TDD tests for structural intelligence
//!
//! These tests define the expected behavior BEFORE implementation.
//! All tests should FAIL initially (RED phase).

use super::folding::FoldingAnalyzer;
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

// ============================================================================
// FOLDING RANGE TESTS
// ============================================================================

/// TEST 1: Attribute Grouping (Killer Feature)
/// Consecutive attributes should fold together as "Imports" kind
#[test]
fn test_attribute_grouping() {
    let text = "\
:author: Alan
:version: 1.0
:date: 2025-01-01

== Next Section";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Expect exactly one fold for lines 0-2 (3 attributes)
    let attr_folds: Vec<&FoldingRange> = ranges
        .iter()
        .filter(|r| r.kind == Some(FoldingRangeKind::Imports))
        .collect();

    assert_eq!(attr_folds.len(), 1, "Should have exactly one attribute folding range");

    let fold = attr_folds[0];
    assert_eq!(fold.start_line, 0, "Should start at line 0");
    assert_eq!(fold.end_line, 2, "Should end at line 2");
}

/// TEST 2: Single Attribute No Fold
/// A single attribute should NOT create a fold (not useful)
#[test]
fn test_single_attribute_no_fold() {
    let text = ":author: Alan\n\n== Section";
    let ranges = FoldingAnalyzer::generate_ranges(text);

    let attr_folds: Vec<&FoldingRange> = ranges
        .iter()
        .filter(|r| r.kind == Some(FoldingRangeKind::Imports))
        .collect();

    assert!(attr_folds.is_empty(), "Single attribute should not fold");
}

/// TEST 3: Header Hierarchy Folding
/// Headers should fold their content until the next same-or-higher level header
#[test]
fn test_header_hierarchy() {
    let text = "\
= Title

== Section A
Content A1
Content A2

=== Subsection B
Content B

== Section C
Content C";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Find Section A fold (line 2, should end before Section C at line 9)
    let section_a = ranges
        .iter()
        .find(|r| r.start_line == 2 && r.kind == Some(FoldingRangeKind::Region));
    assert!(section_a.is_some(), "Section A should be foldable");
    assert_eq!(
        section_a.unwrap().end_line, 8,
        "Section A should fold until line 8 (before Section C)"
    );

    // Find Subsection B fold (line 6)
    let subsection_b = ranges
        .iter()
        .find(|r| r.start_line == 6 && r.kind == Some(FoldingRangeKind::Region));
    assert!(subsection_b.is_some(), "Subsection B should be foldable");
}

/// TEST 4: Block Delimiter Folding
/// Delimited blocks (----, ...., etc.) should fold
#[test]
fn test_block_folding() {
    let text = "\
Preamble

----
Code line 1
Code line 2
----

Epilogue";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Block starts at line 2, ends at line 5
    let block_fold = ranges
        .iter()
        .find(|r| r.start_line == 2 && r.kind == Some(FoldingRangeKind::Region));

    assert!(block_fold.is_some(), "Block should be foldable");
    assert_eq!(
        block_fold.unwrap().end_line, 5,
        "Should fold entire block including delimiters"
    );
}

/// TEST 5: Mixed Content Edge Case
/// Only CONTIGUOUS attributes should fold together
#[test]
fn test_mixed_attribute_blocks() {
    let text = "\
:attr1: value
Text breaks the chain
:attr2: value
:attr3: value

== Section";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Should have ONE fold for lines 2-3 (attr2 + attr3)
    let attribute_folds: Vec<&FoldingRange> = ranges
        .iter()
        .filter(|r| r.kind == Some(FoldingRangeKind::Imports))
        .collect();

    assert_eq!(
        attribute_folds.len(),
        1,
        "Should fold only contiguous attributes"
    );
    assert_eq!(attribute_folds[0].start_line, 2, "Should start at attr2");
    assert_eq!(attribute_folds[0].end_line, 3, "Should end at attr3");
}

/// TEST 6: Empty Document
/// Empty document should have no folds
#[test]
fn test_empty_document() {
    let ranges = FoldingAnalyzer::generate_ranges("");
    assert!(ranges.is_empty(), "Empty document should have no folds");
}

/// TEST 7: Complex Real-World ADR Example
/// Tests a realistic Architecture Decision Record document
#[test]
fn test_real_world_adr_example() {
    let text = "\
:author: Architecture Team
:date: 2025-12-29
:status: proposed
:tags: [microservices, api-gateway]

= ADR-001: API Gateway Implementation

== Context
Our monolithic backend is becoming unmaintainable.

== Decision
We will implement an API Gateway pattern using Kong.

Technical details:

----
upstreams:
  - user-service:8080
  - order-service:8081
----

== Consequences
Positive: Better scalability.
Negative: Adds deployment complexity.";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Should have:
    // 1. Attribute group fold (lines 0-3)
    // 2. Title fold (line 5, optional)
    // 3. Context section fold
    // 4. Decision section fold
    // 5. Code block fold
    // 6. Consequences section fold

    // Verify attribute fold exists and is 'Imports' kind
    let attr_fold = ranges
        .iter()
        .find(|r| r.start_line == 0 && r.kind == Some(FoldingRangeKind::Imports));
    assert!(attr_fold.is_some(), "Should have attribute group fold");
    assert_eq!(attr_fold.unwrap().end_line, 3, "Attribute fold should end at line 3");

    // Verify we have multiple folds (headers + block)
    assert!(
        ranges.len() >= 4,
        "Should have at least 4 folding ranges (attrs + sections + block), got {}",
        ranges.len()
    );
}

/// TEST 8: Nested blocks inside headers
#[test]
fn test_nested_blocks_in_headers() {
    let text = "\
== Section

----
code
----

More content";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Should have both header fold and block fold
    let header_fold = ranges
        .iter()
        .find(|r| r.start_line == 0 && r.kind == Some(FoldingRangeKind::Region));
    let block_fold = ranges.iter().find(|r| r.start_line == 2);

    assert!(header_fold.is_some(), "Header should be foldable");
    assert!(block_fold.is_some(), "Nested block should be foldable");
}

/// TEST 9: Different block delimiter types
#[test]
fn test_different_block_delimiters() {
    let text = "\
....
literal block
....

====
example block
====

****
sidebar
****";

    let ranges = FoldingAnalyzer::generate_ranges(text);

    // Should have 3 block folds
    let block_folds: Vec<&FoldingRange> = ranges
        .iter()
        .filter(|r| r.kind == Some(FoldingRangeKind::Region))
        .collect();

    assert_eq!(block_folds.len(), 3, "Should have 3 block folds");
}

// ============================================================================
// SCANNER UNIT TESTS
// ============================================================================

use super::scanner::{LineType, StructuralScanner};

#[test]
fn test_scanner_header_detection() {
    assert_eq!(StructuralScanner::scan("= Title"), LineType::Header(1));
    assert_eq!(StructuralScanner::scan("== Section"), LineType::Header(2));
    assert_eq!(StructuralScanner::scan("=== Subsection"), LineType::Header(3));
    assert_eq!(StructuralScanner::scan("==== Level 4"), LineType::Header(4));
    assert_eq!(StructuralScanner::scan("===== Level 5"), LineType::Header(5));
    assert_eq!(StructuralScanner::scan("====== Level 6"), LineType::Header(6));
    // 7+ equals is not a valid header
    assert_eq!(StructuralScanner::scan("======= Too Deep"), LineType::Other);
}

#[test]
fn test_scanner_attribute_detection() {
    assert_eq!(StructuralScanner::scan(":author: Alan"), LineType::Attribute);
    assert_eq!(
        StructuralScanner::scan(":some-attr: value"),
        LineType::Attribute
    );
    assert_eq!(
        StructuralScanner::scan(":attr:: escaped"),
        LineType::Attribute
    );
    // Not attributes
    assert_eq!(StructuralScanner::scan("not:attribute"), LineType::Other);
    assert_eq!(StructuralScanner::scan(": missing name"), LineType::Other);
}

#[test]
fn test_scanner_block_delimiter_detection() {
    assert_eq!(StructuralScanner::scan("----"), LineType::BlockDelimiter);
    assert_eq!(StructuralScanner::scan("...."), LineType::BlockDelimiter);
    assert_eq!(StructuralScanner::scan("===="), LineType::BlockDelimiter);
    assert_eq!(StructuralScanner::scan("****"), LineType::BlockDelimiter);
    assert_eq!(StructuralScanner::scan("____"), LineType::BlockDelimiter);
    assert_eq!(StructuralScanner::scan("------"), LineType::BlockDelimiter);
    // Too short (must be 4+)
    assert_eq!(StructuralScanner::scan("---"), LineType::Other);
    assert_eq!(StructuralScanner::scan("..."), LineType::Other);
}

#[test]
fn test_scanner_other_lines() {
    assert_eq!(StructuralScanner::scan(""), LineType::Other);
    assert_eq!(StructuralScanner::scan("   "), LineType::Other);
    assert_eq!(StructuralScanner::scan("Regular text"), LineType::Other);
    assert_eq!(StructuralScanner::scan("* List item"), LineType::Other);
}
