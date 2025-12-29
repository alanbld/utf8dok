//! Diagram Plugin (Phase 17)
//!
//! Provides syntax validation and hints for diagram code blocks:
//! - Mermaid diagram validation
//! - PlantUML diagram validation
//! - Diagram type detection

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::config::Settings;

/// Diagram syntax validator
#[derive(Debug, Clone)]
pub struct DiagramPlugin {
    /// Whether the plugin is enabled
    enabled: bool,
}

impl DiagramPlugin {
    /// Create a new diagram plugin with default settings
    pub fn new() -> Self {
        Self::with_settings(&Settings::default())
    }

    /// Create a diagram plugin configured from settings
    pub fn with_settings(settings: &Settings) -> Self {
        Self {
            enabled: settings.plugins.diagrams,
        }
    }

    /// Validate diagram blocks in text
    pub fn validate_diagrams(&self, text: &str) -> Vec<Diagnostic> {
        if !self.enabled {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Find all code blocks and validate them
        let blocks = self.find_diagram_blocks(text);
        for block in blocks {
            diagnostics.extend(self.validate_block(&block));
        }

        diagnostics
    }

    /// Find diagram code blocks in text
    fn find_diagram_blocks(&self, text: &str) -> Vec<DiagramBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            // Check for AsciiDoc diagram block start (e.g., [mermaid])
            if let Some(diagram_type) = self.detect_block_start(lines[i]) {
                let start_line = i as u32;
                i += 1;

                // Skip to the opening ---- delimiter
                while i < lines.len() && !lines[i].starts_with("----") {
                    i += 1;
                }
                i += 1; // Skip the ---- line itself

                // Collect content until closing ----
                let mut content_lines = Vec::new();
                while i < lines.len() && !lines[i].starts_with("----") {
                    content_lines.push(lines[i].to_string());
                    i += 1;
                }

                blocks.push(DiagramBlock {
                    diagram_type,
                    content: content_lines.join("\n"),
                    start_line,
                    end_line: i as u32,
                });
            }
            i += 1;
        }

        blocks
    }

    /// Detect if a line starts a diagram block
    fn detect_block_start(&self, line: &str) -> Option<DiagramType> {
        let line_lower = line.to_lowercase();

        // AsciiDoc style: [mermaid] or [plantuml]
        if line_lower.contains("[mermaid]") {
            return Some(DiagramType::Mermaid);
        }
        if line_lower.contains("[plantuml]") {
            return Some(DiagramType::PlantUml);
        }

        // Also detect Markdown-style fenced blocks
        if line.starts_with("```mermaid") {
            return Some(DiagramType::Mermaid);
        }
        if line.starts_with("```plantuml") {
            return Some(DiagramType::PlantUml);
        }

        None
    }

    /// Validate a single diagram block
    fn validate_block(&self, block: &DiagramBlock) -> Vec<Diagnostic> {
        match block.diagram_type {
            DiagramType::Mermaid => self.validate_mermaid(block),
            DiagramType::PlantUml => self.validate_plantuml(block),
        }
    }

    /// Validate Mermaid diagram syntax
    fn validate_mermaid(&self, block: &DiagramBlock) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let content = block.content.trim();

        if content.is_empty() {
            diagnostics.push(self.create_diagnostic(
                block.start_line,
                0,
                block.start_line,
                10,
                "DIAGRAM001",
                "Empty Mermaid diagram block",
                DiagnosticSeverity::WARNING,
            ));
            return diagnostics;
        }

        // Check for valid Mermaid diagram type declarations
        let valid_types = [
            "graph",
            "flowchart",
            "sequencediagram",
            "classDiagram",
            "statediagram",
            "erdiagram",
            "journey",
            "gantt",
            "pie",
            "quadrantchart",
            "requirementdiagram",
            "gitgraph",
            "mindmap",
            "timeline",
            "zenuml",
            "sankey",
            "xychart",
            "block",
        ];

        let first_line = content.lines().next().unwrap_or("").to_lowercase();
        let first_word = first_line.split_whitespace().next().unwrap_or("");

        // Normalize for comparison (remove hyphens/underscores)
        let normalized = first_word.replace(['-', '_'], "");

        let has_valid_type = valid_types
            .iter()
            .any(|t| normalized.starts_with(&t.to_lowercase()));

        if !has_valid_type {
            diagnostics.push(self.create_diagnostic(
                block.start_line + 1,
                0,
                block.start_line + 1,
                first_word.len() as u32,
                "DIAGRAM002",
                &format!(
                    "Unknown Mermaid diagram type: '{}'. Expected one of: graph, flowchart, sequenceDiagram, classDiagram, etc.",
                    first_word
                ),
                DiagnosticSeverity::WARNING,
            ));
        }

        // Check for common syntax issues
        for (line_offset, line) in content.lines().enumerate() {
            let line_num = block.start_line + 1 + line_offset as u32;

            // Check for unclosed brackets
            let open_parens = line.chars().filter(|c| *c == '(').count();
            let close_parens = line.chars().filter(|c| *c == ')').count();
            if open_parens != close_parens {
                diagnostics.push(self.create_diagnostic(
                    line_num,
                    0,
                    line_num,
                    line.len() as u32,
                    "DIAGRAM003",
                    "Unbalanced parentheses in Mermaid diagram",
                    DiagnosticSeverity::WARNING,
                ));
            }

            let open_brackets = line.chars().filter(|c| *c == '[').count();
            let close_brackets = line.chars().filter(|c| *c == ']').count();
            if open_brackets != close_brackets {
                diagnostics.push(self.create_diagnostic(
                    line_num,
                    0,
                    line_num,
                    line.len() as u32,
                    "DIAGRAM003",
                    "Unbalanced brackets in Mermaid diagram",
                    DiagnosticSeverity::WARNING,
                ));
            }

            let open_braces = line.chars().filter(|c| *c == '{').count();
            let close_braces = line.chars().filter(|c| *c == '}').count();
            if open_braces != close_braces {
                diagnostics.push(self.create_diagnostic(
                    line_num,
                    0,
                    line_num,
                    line.len() as u32,
                    "DIAGRAM003",
                    "Unbalanced braces in Mermaid diagram",
                    DiagnosticSeverity::WARNING,
                ));
            }
        }

        diagnostics
    }

    /// Validate PlantUML diagram syntax
    fn validate_plantuml(&self, block: &DiagramBlock) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let content = block.content.trim();

        if content.is_empty() {
            diagnostics.push(self.create_diagnostic(
                block.start_line,
                0,
                block.start_line,
                10,
                "DIAGRAM001",
                "Empty PlantUML diagram block",
                DiagnosticSeverity::WARNING,
            ));
            return diagnostics;
        }

        // Check for @startuml
        if !content.to_lowercase().contains("@startuml") {
            diagnostics.push(self.create_diagnostic(
                block.start_line + 1,
                0,
                block.start_line + 1,
                10,
                "DIAGRAM004",
                "PlantUML diagram should start with @startuml",
                DiagnosticSeverity::WARNING,
            ));
        }

        // Check for @enduml
        if !content.to_lowercase().contains("@enduml") {
            let last_content_line = block.end_line.saturating_sub(1);
            diagnostics.push(self.create_diagnostic(
                last_content_line,
                0,
                last_content_line,
                10,
                "DIAGRAM005",
                "PlantUML diagram should end with @enduml",
                DiagnosticSeverity::WARNING,
            ));
        }

        // Check for common syntax issues
        for (line_offset, line) in content.lines().enumerate() {
            let line_num = block.start_line + 1 + line_offset as u32;

            // Check for unclosed quotes
            let quote_count = line.chars().filter(|c| *c == '"').count();
            if quote_count % 2 != 0 {
                diagnostics.push(self.create_diagnostic(
                    line_num,
                    0,
                    line_num,
                    line.len() as u32,
                    "DIAGRAM006",
                    "Unclosed quote in PlantUML diagram",
                    DiagnosticSeverity::WARNING,
                ));
            }
        }

        diagnostics
    }

    /// Create a diagnostic with the given parameters
    #[allow(clippy::too_many_arguments)]
    fn create_diagnostic(
        &self,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
        code: &str,
        message: &str,
        severity: DiagnosticSeverity,
    ) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_char,
                },
                end: Position {
                    line: end_line,
                    character: end_char,
                },
            },
            severity: Some(severity),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                code.to_string(),
            )),
            source: Some("diagrams".to_string()),
            message: message.to_string(),
            ..Default::default()
        }
    }
}

impl Default for DiagramPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of diagrams we can validate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    Mermaid,
    PlantUml,
}

/// A detected diagram block
#[derive(Debug, Clone)]
struct DiagramBlock {
    diagram_type: DiagramType,
    content: String,
    start_line: u32,
    end_line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_block_detection() {
        let plugin = DiagramPlugin::new();
        let text = r#"= Document

[mermaid]
----
graph TD
    A --> B
----

Some text.
"#;

        let diagnostics = plugin.validate_diagrams(text);
        // Valid mermaid should produce no diagnostics
        assert!(
            diagnostics.is_empty(),
            "Valid mermaid should not produce diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_mermaid_block() {
        let plugin = DiagramPlugin::new();
        let text = r#"[mermaid]
----
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty(), "Should detect empty diagram");
        assert!(diagnostics[0].message.contains("Empty"));
    }

    #[test]
    fn test_invalid_mermaid_type() {
        let plugin = DiagramPlugin::new();
        let text = r#"[mermaid]
----
invalid_type
    A --> B
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty(), "Should detect invalid type");
        assert!(diagnostics[0]
            .message
            .contains("Unknown Mermaid diagram type"));
    }

    #[test]
    fn test_plantuml_block_detection() {
        let plugin = DiagramPlugin::new();
        let text = r#"[plantuml]
----
@startuml
Alice -> Bob: Hello
@enduml
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(
            diagnostics.is_empty(),
            "Valid PlantUML should not produce diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_plantuml_missing_startuml() {
        let plugin = DiagramPlugin::new();
        let text = r#"[plantuml]
----
Alice -> Bob: Hello
@enduml
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty(), "Should detect missing @startuml");
        assert!(diagnostics.iter().any(|d| d.message.contains("@startuml")));
    }

    #[test]
    fn test_plantuml_missing_enduml() {
        let plugin = DiagramPlugin::new();
        let text = r#"[plantuml]
----
@startuml
Alice -> Bob: Hello
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty(), "Should detect missing @enduml");
        assert!(diagnostics.iter().any(|d| d.message.contains("@enduml")));
    }

    #[test]
    fn test_unbalanced_brackets() {
        let plugin = DiagramPlugin::new();
        let text = r#"[mermaid]
----
graph TD
    A[Node A --> B
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty(), "Should detect unbalanced brackets");
        assert!(diagnostics.iter().any(|d| d.message.contains("Unbalanced")));
    }

    #[test]
    fn test_plugin_disabled() {
        let mut settings = Settings::default();
        settings.plugins.diagrams = false;

        let plugin = DiagramPlugin::with_settings(&settings);
        let text = r#"[mermaid]
----
invalid_content
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(diagnostics.is_empty(), "Disabled plugin should not report");
    }

    #[test]
    fn test_multiple_diagram_blocks() {
        let plugin = DiagramPlugin::new();
        let text = r#"= Document

[mermaid]
----
graph TD
    A --> B
----

[plantuml]
----
@startuml
Alice -> Bob
@enduml
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        // Both diagrams are valid
        assert!(
            diagnostics.is_empty(),
            "Valid diagrams should not produce diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_warning_severity() {
        let plugin = DiagramPlugin::new();
        let text = r#"[mermaid]
----
----
"#;

        let diagnostics = plugin.validate_diagrams(text);
        assert!(!diagnostics.is_empty());
        assert_eq!(
            diagnostics[0].severity,
            Some(DiagnosticSeverity::WARNING),
            "Diagram issues should be warnings"
        );
    }
}
