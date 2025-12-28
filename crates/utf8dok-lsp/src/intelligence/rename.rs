//! Rename refactoring for AsciiDoc documents
//!
//! Provides safe renaming of:
//! - Section IDs: [[id]] and all <<id>> references
//! - Attributes: :name: and all {name} usages

use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Result of a rename operation
#[derive(Debug, Clone)]
pub struct RenameResult {
    pub edits: Vec<TextEdit>,
    pub old_name: String,
    pub new_name: String,
}

/// Type of renameable target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameTargetType {
    SectionId,      // [[id]]
    Attribute,      // :name:
    CrossReference, // <<id>>
    AttributeUsage, // {name}
}

/// Analyzer for rename refactoring
pub struct RenameAnalyzer<'a> {
    #[allow(dead_code)]
    text: &'a str,
    lines: Vec<&'a str>,
}

impl<'a> RenameAnalyzer<'a> {
    /// Create a new rename analyzer
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            lines: text.lines().collect(),
        }
    }

    /// Analyze rename at position and return edits if possible
    pub fn rename_at_position(&self, position: Position, new_name: &str) -> Option<RenameResult> {
        let line_idx = position.line as usize;
        if line_idx >= self.lines.len() {
            return None;
        }

        let line = self.lines[line_idx];
        let char_idx = position.character as usize;

        if char_idx >= line.len() {
            return None;
        }

        // Try to find rename target at position
        let (old_name, target_type) = self.find_rename_target(line, char_idx)?;

        // Find all references based on target type
        let references = self.find_all_references(&old_name, target_type);

        if references.is_empty() {
            return None;
        }

        // Create edits for each reference
        let edits = references
            .into_iter()
            .map(|(range, ref_type)| TextEdit {
                range,
                new_text: self.build_replacement(new_name, ref_type),
            })
            .collect();

        Some(RenameResult {
            edits,
            old_name,
            new_name: new_name.to_string(),
        })
    }

    /// Check if rename is available at position (for prepareRename)
    pub fn can_rename_at(&self, position: Position) -> Option<(Range, String)> {
        let line_idx = position.line as usize;
        if line_idx >= self.lines.len() {
            return None;
        }

        let line = self.lines[line_idx];
        let char_idx = position.character as usize;

        if char_idx >= line.len() {
            return None;
        }

        // Try to find rename target
        let (name, target_type) = self.find_rename_target(line, char_idx)?;

        // Find the range of the name at cursor
        let range = self.find_name_range_at(line, char_idx, target_type)?;

        Some((
            Range {
                start: Position {
                    line: position.line,
                    character: range.0 as u32,
                },
                end: Position {
                    line: position.line,
                    character: range.1 as u32,
                },
            },
            name,
        ))
    }

    /// Find rename target at position
    fn find_rename_target(&self, line: &str, char_idx: usize) -> Option<(String, RenameTargetType)> {
        // Check for section ID: [[id]]
        if let Some(id) = self.find_section_id_at(line, char_idx) {
            return Some((id, RenameTargetType::SectionId));
        }

        // Check for cross-reference: <<id>>
        if let Some(id) = self.find_xref_at(line, char_idx) {
            return Some((id, RenameTargetType::CrossReference));
        }

        // Check for attribute definition: :name:
        if let Some(name) = self.find_attribute_def_at(line, char_idx) {
            return Some((name, RenameTargetType::Attribute));
        }

        // Check for attribute usage: {name}
        if let Some(name) = self.find_attribute_usage_at(line, char_idx) {
            return Some((name, RenameTargetType::AttributeUsage));
        }

        None
    }

    /// Find section ID at position: [[id]]
    fn find_section_id_at(&self, line: &str, char_idx: usize) -> Option<String> {
        static ID_RE: OnceLock<Regex> = OnceLock::new();
        let re = ID_RE.get_or_init(|| Regex::new(r"\[\[([\w\-]+)\]\]").unwrap());

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let id_match = cap.get(1).unwrap();

            // Check if cursor is on the ID part (not brackets)
            if full_match.start() <= char_idx && char_idx < full_match.end() {
                // Only if cursor is actually on the ID characters
                if id_match.start() <= char_idx && char_idx < id_match.end() {
                    return Some(id_match.as_str().to_string());
                }
            }
        }

        None
    }

    /// Find cross-reference at position: <<id>>
    fn find_xref_at(&self, line: &str, char_idx: usize) -> Option<String> {
        static XREF_RE: OnceLock<Regex> = OnceLock::new();
        let re = XREF_RE.get_or_init(|| Regex::new(r"<<([\w\-]+)(?:,[^>]*)?\s*>>").unwrap());

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let id_match = cap.get(1).unwrap();

            if full_match.start() <= char_idx
                && char_idx < full_match.end()
                && id_match.start() <= char_idx
                && char_idx < id_match.end()
            {
                return Some(id_match.as_str().to_string());
            }
        }

        None
    }

    /// Find attribute definition at position: :name:
    fn find_attribute_def_at(&self, line: &str, char_idx: usize) -> Option<String> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w\-]+):").unwrap());

        if let Some(cap) = re.captures(line.trim_start()) {
            let name_match = cap.get(1).unwrap();
            let line_offset = line.len() - line.trim_start().len();

            let abs_start = line_offset + name_match.start();
            let abs_end = line_offset + name_match.end();

            if abs_start <= char_idx && char_idx < abs_end {
                return Some(name_match.as_str().to_string());
            }
        }

        None
    }

    /// Find attribute usage at position: {name}
    fn find_attribute_usage_at(&self, line: &str, char_idx: usize) -> Option<String> {
        static USAGE_RE: OnceLock<Regex> = OnceLock::new();
        let re = USAGE_RE.get_or_init(|| Regex::new(r"\{([\w\-]+)\}").unwrap());

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let name_match = cap.get(1).unwrap();

            if full_match.start() <= char_idx
                && char_idx < full_match.end()
                && name_match.start() <= char_idx
                && char_idx < name_match.end()
            {
                return Some(name_match.as_str().to_string());
            }
        }

        None
    }

    /// Find name range at cursor position
    fn find_name_range_at(&self, line: &str, char_idx: usize, target_type: RenameTargetType) -> Option<(usize, usize)> {
        let re: &Regex = match target_type {
            RenameTargetType::SectionId => {
                static RE: OnceLock<Regex> = OnceLock::new();
                RE.get_or_init(|| Regex::new(r"\[\[([\w\-]+)\]\]").unwrap())
            }
            RenameTargetType::CrossReference => {
                static RE: OnceLock<Regex> = OnceLock::new();
                RE.get_or_init(|| Regex::new(r"<<([\w\-]+)(?:,[^>]*)?\s*>>").unwrap())
            }
            RenameTargetType::Attribute => {
                static RE: OnceLock<Regex> = OnceLock::new();
                RE.get_or_init(|| Regex::new(r"^:([\w\-]+):").unwrap())
            }
            RenameTargetType::AttributeUsage => {
                static RE: OnceLock<Regex> = OnceLock::new();
                RE.get_or_init(|| Regex::new(r"\{([\w\-]+)\}").unwrap())
            }
        };

        for cap in re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let name_match = cap.get(1).unwrap();

            if full_match.start() <= char_idx && char_idx < full_match.end() {
                return Some((name_match.start(), name_match.end()));
            }
        }

        None
    }

    /// Find all references to target
    fn find_all_references(&self, target: &str, target_type: RenameTargetType) -> Vec<(Range, RenameTargetType)> {
        let mut references = Vec::new();
        let escaped = regex::escape(target);

        match target_type {
            RenameTargetType::SectionId | RenameTargetType::CrossReference => {
                // Find ID definition [[id]]
                references.extend(self.find_pattern_ranges(
                    &format!(r"\[\[{}\]\]", escaped),
                    RenameTargetType::SectionId,
                ));
                // Find all xrefs <<id>>
                references.extend(self.find_pattern_ranges(
                    &format!(r"<<{}(?:,[^>]*)?\s*>>", escaped),
                    RenameTargetType::CrossReference,
                ));
            }
            RenameTargetType::Attribute | RenameTargetType::AttributeUsage => {
                // Find definition :name:
                references.extend(self.find_pattern_ranges(
                    &format!(r"^:{}:", escaped),
                    RenameTargetType::Attribute,
                ));
                // Find usages {name}
                references.extend(self.find_pattern_ranges(
                    &format!(r"\{{{}\}}", escaped),
                    RenameTargetType::AttributeUsage,
                ));
            }
        }

        references
    }

    /// Find ranges matching pattern
    fn find_pattern_ranges(&self, pattern: &str, ref_type: RenameTargetType) -> Vec<(Range, RenameTargetType)> {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let mut ranges = Vec::new();

        for (line_idx, line) in self.lines.iter().enumerate() {
            for mat in re.find_iter(line) {
                ranges.push((
                    Range {
                        start: Position {
                            line: line_idx as u32,
                            character: mat.start() as u32,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: mat.end() as u32,
                        },
                    },
                    ref_type,
                ));
            }
        }

        ranges
    }

    /// Build replacement text for a reference
    fn build_replacement(&self, new_name: &str, ref_type: RenameTargetType) -> String {
        match ref_type {
            RenameTargetType::SectionId => format!("[[{}]]", new_name),
            RenameTargetType::CrossReference => format!("<<{}>>", new_name),
            RenameTargetType::Attribute => format!(":{}:", new_name),
            RenameTargetType::AttributeUsage => format!("{{{}}}", new_name),
        }
    }
}
