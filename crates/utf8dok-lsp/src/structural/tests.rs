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

// ============================================================================
// DOCUMENT SYMBOL TESTS (Week 2)
// ============================================================================

use super::symbols::SymbolAnalyzer;
use tower_lsp::lsp_types::SymbolKind;

/// TEST S1: Basic Header Hierarchy
/// Headers should create nested symbols with correct kinds
#[test]
fn test_symbol_basic_header_hierarchy() {
    let text = "\
= Main Title

== Section One
Content here.

=== Subsection
More content.

== Section Two
Final content.";

    let symbols = SymbolAnalyzer::extract_symbols(text);

    // Should have root symbol
    assert!(!symbols.is_empty(), "Should extract at least one symbol");

    // Root should be "Main Title"
    let root = &symbols[0];
    assert_eq!(root.name, "Main Title");
    assert_eq!(root.kind, SymbolKind::MODULE);

    // Should have 2 children: Section One, Section Two
    let children = root.children.as_ref().expect("Root should have children");
    assert_eq!(children.len(), 2, "Root should have 2 section children");

    let section_one = &children[0];
    assert_eq!(section_one.name, "Section One");
    assert_eq!(section_one.kind, SymbolKind::NAMESPACE);

    // Section One should have 1 child: Subsection
    let section_one_children = section_one.children.as_ref().expect("Section One should have children");
    assert_eq!(section_one_children.len(), 1);
    let subsection = &section_one_children[0];
    assert_eq!(subsection.name, "Subsection");
    assert_eq!(subsection.kind, SymbolKind::CLASS);
}

/// TEST S2: Attributes as Variables
/// Document attributes should appear as VARIABLE symbols with values
#[test]
fn test_symbol_attributes_as_variables() {
    let text = "\
:author: Alan
:version: 1.0
:status: draft

= Document Title

== Section";

    let symbols = SymbolAnalyzer::extract_symbols(text);

    // Should have document title as root
    assert!(!symbols.is_empty(), "Should have symbols");

    // Find attribute symbols (they should be at root level before the title)
    let attr_symbols: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::VARIABLE)
        .collect();

    assert_eq!(attr_symbols.len(), 3, "Should have 3 attribute symbols");

    // Check first attribute
    assert_eq!(attr_symbols[0].name, "author");
    assert_eq!(attr_symbols[0].detail.as_deref(), Some("Alan"));

    assert_eq!(attr_symbols[1].name, "version");
    assert_eq!(attr_symbols[1].detail.as_deref(), Some("1.0"));
}

/// TEST S3: Blocks as Objects
/// Code blocks should appear as OBJECT symbols with line count
#[test]
fn test_symbol_blocks_as_objects() {
    let text = "\
== Section

Some text.

----
function hello() {
    return 'world';
}
----

More text.";

    let symbols = SymbolAnalyzer::extract_symbols(text);

    // Should have section
    assert!(!symbols.is_empty(), "Should have section symbol");
    let section = &symbols[0];
    assert_eq!(section.name, "Section");

    // Section should have a block child
    let children = section.children.as_ref();
    assert!(children.is_some(), "Section should have children");

    let block_symbols: Vec<_> = children
        .unwrap()
        .iter()
        .filter(|s| s.kind == SymbolKind::OBJECT)
        .collect();

    assert_eq!(block_symbols.len(), 1, "Should have 1 block symbol");
    assert!(
        block_symbols[0].name.contains("block"),
        "Block name should contain 'block'"
    );
}

/// TEST S4: Empty Document
/// Empty document should have no symbols
#[test]
fn test_symbol_empty_document() {
    let symbols = SymbolAnalyzer::extract_symbols("");
    assert!(symbols.is_empty(), "Empty document should have no symbols");
}

/// TEST S5: Real ADR Example
/// Tests a realistic Architecture Decision Record document
#[test]
fn test_symbol_real_adr_example() {
    let text = "\
:author: Architecture Team
:date: 2025-12-29
:status: proposed

= ADR-001: API Gateway Implementation

== Context
Our monolith is hard to scale.

== Decision
We will use Kong as API Gateway.

Technical details:

----
upstreams:
  - user-service:8080
  - order-service:8081
----

== Consequences
Positive: Scalability.
Negative: Complexity.";

    let symbols = SymbolAnalyzer::extract_symbols(text);

    // Should have attributes + document structure
    assert!(!symbols.is_empty(), "Should have symbols");

    // Count attributes
    let attr_count = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::VARIABLE)
        .count();
    assert_eq!(attr_count, 3, "Should have 3 attribute symbols");

    // Find main title
    let title = symbols
        .iter()
        .find(|s| s.kind == SymbolKind::MODULE);
    assert!(title.is_some(), "Should have title symbol");
    assert_eq!(title.unwrap().name, "ADR-001: API Gateway Implementation");

    // Title should have section children
    let title_children = title.unwrap().children.as_ref();
    assert!(title_children.is_some(), "Title should have children");

    let sections: Vec<_> = title_children
        .unwrap()
        .iter()
        .filter(|s| s.kind == SymbolKind::NAMESPACE)
        .collect();
    assert_eq!(sections.len(), 3, "Should have 3 section symbols (Context, Decision, Consequences)");
}

/// TEST S6: Document without title
/// Document starting with == should still work
#[test]
fn test_symbol_no_title() {
    let text = "\
== Section One
Content.

== Section Two
More content.";

    let symbols = SymbolAnalyzer::extract_symbols(text);

    // Should have 2 top-level section symbols
    let section_symbols: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::NAMESPACE)
        .collect();

    assert_eq!(section_symbols.len(), 2, "Should have 2 section symbols");
    assert_eq!(section_symbols[0].name, "Section One");
    assert_eq!(section_symbols[1].name, "Section Two");
}

/// TEST S7: Performance - Large Document
/// Should process 1000+ lines quickly
#[test]
fn test_symbol_large_document_performance() {
    // Build a large document
    let mut text = String::new();
    text.push_str("= Large Document\n\n");

    for i in 0..20 {
        text.push_str(&format!("== Section {}\n\n", i));
        for j in 0..5 {
            text.push_str(&format!("=== Subsection {}.{}\n\n", i, j));
            text.push_str("Some content here.\n\n");
        }
    }

    // Should complete in reasonable time
    let start = std::time::Instant::now();
    let symbols = SymbolAnalyzer::extract_symbols(&text);
    let duration = start.elapsed();

    assert!(!symbols.is_empty(), "Should extract symbols");
    assert!(
        duration < std::time::Duration::from_millis(100),
        "Should process large document in <100ms, took {:?}",
        duration
    );
}
