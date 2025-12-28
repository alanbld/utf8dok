//! TDD tests for intelligence module
//!
//! These tests define the expected behavior BEFORE implementation.

use tower_lsp::lsp_types::{Position, Range};

fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Range {
    Range {
        start: pos(start_line, start_char),
        end: pos(end_line, end_char),
    }
}

// ============================================================================
// SELECTION RANGE TESTS
// ============================================================================

mod selection_tests {
    use super::*;
    use crate::intelligence::selection::SelectionAnalyzer;

    /// TEST 1: Basic Hierarchy Expansion
    /// Cursor in content should expand: word → line → paragraph → section
    #[test]
    fn test_selection_range_expansion() {
        let text = "\
= Title

== Section A
Line 1
Line 2";

        let analyzer = SelectionAnalyzer::new(text);
        let cursor = pos(3, 2); // On "Line 1" (line 3, char 2 = "n")

        let ranges = analyzer.get_selection_hierarchy(cursor);

        // Should have multiple selection levels
        assert!(ranges.len() >= 3, "Should have at least 3 selection levels, got {}", ranges.len());

        // Level 1: Word "Line"
        assert_eq!(ranges[0].range, range(3, 0, 3, 4), "Level 1: Word 'Line'");

        // Level 2: Entire line "Line 1"
        assert_eq!(ranges[1].range, range(3, 0, 3, 6), "Level 2: Line");
    }

    /// TEST 2: Attribute Selection
    /// Attributes should select: name → line → attribute group
    #[test]
    fn test_attribute_selection() {
        let text = "\
:author: Alan
:version: 1.0
:status: draft";

        let analyzer = SelectionAnalyzer::new(text);

        // Test selecting attribute name
        let cursor = pos(0, 3); // On "author" in ":author:"
        let ranges = analyzer.get_selection_hierarchy(cursor);

        assert!(ranges.len() >= 2, "Should have at least 2 selection levels");

        // First level should be the word "author"
        assert_eq!(ranges[0].range, range(0, 1, 0, 7), "Should select 'author'");

        // Should include entire attribute group
        let has_group = ranges.iter().any(|r|
            r.range.start.line == 0 && r.range.end.line == 2
        );
        assert!(has_group, "Should include entire attribute group");
    }

    /// TEST 3: Block Delimiter Selection
    /// Code blocks should be selectable as a unit
    #[test]
    fn test_block_selection() {
        let text = "\
Preamble

----
code line 1
code line 2
----

Epilogue";

        let analyzer = SelectionAnalyzer::new(text);

        // Test selecting inside code block
        let cursor = pos(3, 3); // Inside code block on "code line 1"
        let ranges = analyzer.get_selection_hierarchy(cursor);

        // Should include block as a selection level
        let has_block_range = ranges.iter()
            .any(|r| r.range.start.line == 2 && r.range.end.line == 5);
        assert!(has_block_range, "Should include entire block as selection");
    }

    /// TEST 4: Header Title Selection
    /// Header should expand: word → title → header line → section
    #[test]
    fn test_header_title_selection() {
        let text = "== Important Section Title\n\nContent";

        let analyzer = SelectionAnalyzer::new(text);

        // Test selecting part of header title
        let cursor = pos(0, 6); // On "Important"
        let ranges = analyzer.get_selection_hierarchy(cursor);

        assert!(ranges.len() >= 2, "Should have at least 2 levels");

        // Level 1: Word "Important"
        assert_eq!(ranges[0].range, range(0, 3, 0, 12), "Should select word 'Important'");
    }

    /// TEST 5: Empty/Whitespace Selection
    /// Empty lines should still expand to document
    #[test]
    fn test_whitespace_selection() {
        let text = "Line 1\n\nLine 3";

        let analyzer = SelectionAnalyzer::new(text);

        // Test selecting empty line
        let cursor = pos(1, 0); // Empty line
        let ranges = analyzer.get_selection_hierarchy(cursor);

        // Should have at least line + document
        assert!(ranges.len() >= 1, "Should have at least 1 selection level");

        // Last level should be document
        let last = ranges.last().unwrap();
        assert_eq!(last.range.start.line, 0, "Document should start at 0");
        assert_eq!(last.range.end.line, 2, "Document should end at line 2");
    }

    /// TEST 6: Performance - Large Document
    /// Should process 500+ lines quickly
    #[test]
    fn test_selection_performance() {
        // Build 500-line document
        let mut text = String::new();
        for i in 0..100 {
            text.push_str(&format!("== Section {}\n", i));
            for _ in 0..4 {
                text.push_str("Some content line here.\n");
            }
        }

        let start = std::time::Instant::now();
        let analyzer = SelectionAnalyzer::new(&text);
        let cursor = pos(250, 5);
        let _ranges = analyzer.get_selection_hierarchy(cursor);
        let duration = start.elapsed();

        assert!(
            duration < std::time::Duration::from_millis(50),
            "Selection should be <50ms, took {:?}",
            duration
        );
    }

    /// TEST 7: Cross-reference Selection
    /// <<ref>> should expand: id → xref → line
    #[test]
    fn test_xref_selection() {
        let text = "See <<my-section>> for details.";
        // Position breakdown: "See <<my-section>> for details."
        //                      0123456789...
        // "my-section" is at positions 6-16

        let analyzer = SelectionAnalyzer::new(text);
        let cursor = pos(0, 8); // On "my-section" (the '-' character)
        let ranges = analyzer.get_selection_hierarchy(cursor);

        assert!(ranges.len() >= 2, "Should have at least 2 levels");

        // Should select the xref id "my-section" (positions 6-16)
        let has_id = ranges.iter().any(|r|
            r.range == range(0, 6, 0, 16) // "my-section"
        );
        assert!(has_id, "Should select xref id 'my-section'");
    }
}

// ============================================================================
// RENAME REFACTORING TESTS
// ============================================================================

mod rename_tests {
    use super::*;
    use crate::intelligence::rename::RenameAnalyzer;

    /// TEST 1: Rename Attribute
    /// Renaming attribute should update definition and all usages
    #[test]
    fn test_rename_attribute() {
        let text = ":version: 1.0\nWe are on version {version}.";
        let analyzer = RenameAnalyzer::new(text);

        // Cursor on "version" in attribute definition
        let cursor = pos(0, 3);
        let new_name = "app_ver";

        let result = analyzer.rename_at_position(cursor, new_name).unwrap();

        // Should have 2 edits: definition + usage
        assert_eq!(result.edits.len(), 2, "Should have 2 edits");

        // Check definition edit
        let def_edit = result.edits.iter().find(|e| e.range.start.line == 0).unwrap();
        assert!(def_edit.new_text.contains("app_ver"), "Definition should have new name");

        // Check usage edit
        let usage_edit = result.edits.iter().find(|e| e.range.start.line == 1).unwrap();
        assert!(usage_edit.new_text.contains("app_ver"), "Usage should have new name");
    }

    /// TEST 2: Rename Section ID
    /// Renaming [[id]] should update ID and all <<id>> references
    #[test]
    fn test_rename_section_id() {
        let text = "[[old-id]]\n== Section\nSee <<old-id>>.";
        let analyzer = RenameAnalyzer::new(text);

        // Cursor on "old-id" in section ID
        let cursor = pos(0, 3);
        let new_name = "new-id";

        let result = analyzer.rename_at_position(cursor, new_name).unwrap();

        // Should have 2 edits: ID definition + xref
        assert_eq!(result.edits.len(), 2, "Should have 2 edits");

        // Check ID edit
        let id_edit = result.edits.iter().find(|e| e.range.start.line == 0).unwrap();
        assert_eq!(id_edit.new_text, "[[new-id]]", "ID should be [[new-id]]");

        // Check xref edit
        let xref_edit = result.edits.iter().find(|e| e.range.start.line == 2).unwrap();
        assert_eq!(xref_edit.new_text, "<<new-id>>", "Xref should be <<new-id>>");
    }

    /// TEST 3: Multiple References
    /// Should find and update all references
    #[test]
    fn test_rename_multiple_references() {
        let text = "\
[[target]]
== Target Section

See <<target>>.
Also see <<target>> here.";

        let analyzer = RenameAnalyzer::new(text);
        let cursor = pos(0, 3); // On "target" in ID
        let new_name = "destination";

        let result = analyzer.rename_at_position(cursor, new_name).unwrap();

        // Should have 3 edits: ID + 2 xrefs
        assert_eq!(result.edits.len(), 3, "Should have 3 edits");

        // All should have new name
        for edit in &result.edits {
            assert!(edit.new_text.contains("destination"), "All edits should use new name");
        }
    }

    /// TEST 4: No Rename Available
    /// Plain text should not be renameable
    #[test]
    fn test_no_rename_available() {
        let text = "Just plain text.";
        let analyzer = RenameAnalyzer::new(text);
        let cursor = pos(0, 5); // On "plain"

        let result = analyzer.rename_at_position(cursor, "new");
        assert!(result.is_none(), "Should not rename plain text");
    }

    /// TEST 5: Rename with Special Characters
    /// IDs with dashes should work correctly
    #[test]
    fn test_rename_with_special_chars() {
        let text = "[[id-with-dashes]]\n== Section\nSee <<id-with-dashes>>.";
        let analyzer = RenameAnalyzer::new(text);

        let cursor = pos(0, 3);
        let new_name = "id_with_underscores";

        let result = analyzer.rename_at_position(cursor, new_name).unwrap();

        assert_eq!(result.edits.len(), 2);
        assert!(result.edits[0].new_text.contains("id_with_underscores"));
        assert!(result.edits[1].new_text.contains("id_with_underscores"));
    }

    /// TEST 6: Rename from Xref
    /// Should be able to trigger rename from <<ref>> usage
    #[test]
    fn test_rename_from_xref() {
        let text = "[[my-id]]\n== Section\nSee <<my-id>>.";
        let analyzer = RenameAnalyzer::new(text);

        // Cursor on xref usage instead of definition
        let cursor = pos(2, 7); // On "my-id" in <<my-id>>

        let result = analyzer.rename_at_position(cursor, "renamed");

        // Should still find the definition and update both
        assert!(result.is_some(), "Should be able to rename from xref");
        assert_eq!(result.unwrap().edits.len(), 2);
    }

    /// TEST 7: Invalid Rename Position
    /// Cursor on brackets should not trigger rename
    #[test]
    fn test_invalid_rename_position() {
        let text = "[[valid]]\n== Section";
        let analyzer = RenameAnalyzer::new(text);

        // Cursor on brackets, not ID
        let cursor = pos(0, 0); // On "["
        let result = analyzer.rename_at_position(cursor, "new");

        assert!(result.is_none(), "Should not rename bracket characters");
    }
}
