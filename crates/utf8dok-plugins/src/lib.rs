//! Rhai plugin system for utf8dok custom validation rules
//!
//! This module provides a scripting engine that allows users to write
//! custom validation rules in the Rhai scripting language.
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_plugins::PluginEngine;
//! use utf8dok_ast::Document;
//!
//! let mut engine = PluginEngine::new();
//! let script = r#"
//!     let diagnostics = [];
//!     for block in doc.blocks {
//!         if block.type == "paragraph" {
//!             // Custom validation logic
//!         }
//!     }
//!     diagnostics
//! "#;
//!
//! let ast = engine.compile(script).unwrap();
//! let diagnostics = engine.run_validation(&doc, &ast).unwrap();
//! ```

use rhai::{Array, Dynamic, Engine, Map, Scope, AST};
use thiserror::Error;
use utf8dok_core::diagnostics::{Diagnostic, Severity};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors that can occur during plugin execution
#[derive(Debug, Error)]
pub enum PluginError {
    /// Script compilation failed
    #[error("Script compilation error: {0}")]
    CompileError(String),

    /// Script execution failed
    #[error("Script execution error: {0}")]
    ExecutionError(String),

    /// Invalid diagnostic format returned by script
    #[error("Invalid diagnostic format: {0}")]
    InvalidDiagnostic(String),

    /// Script did not return expected type
    #[error("Script must return an array of diagnostics")]
    InvalidReturnType,
}

/// Result type for plugin operations
pub type Result<T> = std::result::Result<T, PluginError>;

/// The plugin engine that executes Rhai validation scripts
pub struct PluginEngine {
    engine: Engine,
}

impl Default for PluginEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginEngine {
    /// Create a new plugin engine with default configuration
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Set reasonable limits for safety
        engine.set_max_expr_depths(64, 64);
        engine.set_max_call_levels(64);
        engine.set_max_operations(100_000);
        engine.set_max_modules(10);
        engine.set_max_string_size(1_000_000);
        engine.set_max_array_size(10_000);
        engine.set_max_map_size(10_000);

        // Register helper functions
        Self::register_helpers(&mut engine);

        Self { engine }
    }

    /// Register helper functions available to scripts
    fn register_helpers(engine: &mut Engine) {
        // Helper to create a diagnostic map
        engine.register_fn("diagnostic", |message: &str| -> Map {
            let mut map = Map::new();
            map.insert("message".into(), Dynamic::from(message.to_string()));
            map.insert("severity".into(), Dynamic::from("warning".to_string()));
            map
        });

        // Helper to create a warning diagnostic
        engine.register_fn("warning", |code: &str, message: &str| -> Map {
            let mut map = Map::new();
            map.insert("code".into(), Dynamic::from(code.to_string()));
            map.insert("message".into(), Dynamic::from(message.to_string()));
            map.insert("severity".into(), Dynamic::from("warning".to_string()));
            map
        });

        // Helper to create an error diagnostic
        engine.register_fn("error", |code: &str, message: &str| -> Map {
            let mut map = Map::new();
            map.insert("code".into(), Dynamic::from(code.to_string()));
            map.insert("message".into(), Dynamic::from(message.to_string()));
            map.insert("severity".into(), Dynamic::from("error".to_string()));
            map
        });

        // Helper to create an info diagnostic
        engine.register_fn("info", |code: &str, message: &str| -> Map {
            let mut map = Map::new();
            map.insert("code".into(), Dynamic::from(code.to_string()));
            map.insert("message".into(), Dynamic::from(message.to_string()));
            map.insert("severity".into(), Dynamic::from("info".to_string()));
            map
        });

        // String helper: check if contains (case-insensitive)
        engine.register_fn("contains_ci", |haystack: &str, needle: &str| -> bool {
            haystack.to_lowercase().contains(&needle.to_lowercase())
        });

        // String helper: check for passive voice patterns (simple heuristic)
        engine.register_fn("has_passive_pattern", |text: &str| -> bool {
            let passive_patterns = [
                "was ",
                "were ",
                "is being",
                "are being",
                "has been",
                "have been",
                "had been",
                "will be ",
                "being ",
                "been ",
            ];
            let lower = text.to_lowercase();
            passive_patterns.iter().any(|p| lower.contains(p))
        });

        // Array helper: push item
        engine.register_fn("push_diagnostic", |arr: &mut Array, diag: Map| {
            arr.push(Dynamic::from_map(diag));
        });
    }

    /// Compile a Rhai script into an AST
    pub fn compile(&self, script: &str) -> Result<AST> {
        self.engine
            .compile(script)
            .map_err(|e| PluginError::CompileError(e.to_string()))
    }

    /// Compile a Rhai script from a file
    pub fn compile_file(&self, path: &std::path::Path) -> Result<AST> {
        self.engine
            .compile_file(path.into())
            .map_err(|e| PluginError::CompileError(e.to_string()))
    }

    /// Run a validation script against a document
    ///
    /// The script receives the document as `doc` variable and should return
    /// an array of diagnostic maps.
    pub fn run_validation(
        &self,
        doc: &utf8dok_ast::Document,
        ast: &AST,
    ) -> Result<Vec<Diagnostic>> {
        // Convert document to Dynamic using serde
        let doc_dynamic = rhai::serde::to_dynamic(doc).map_err(|e| {
            PluginError::ExecutionError(format!("Failed to serialize document: {}", e))
        })?;

        // Create scope with document
        let mut scope = Scope::new();
        scope.push_constant("doc", doc_dynamic);

        // Run the script
        let result: Dynamic = self
            .engine
            .eval_ast_with_scope(&mut scope, ast)
            .map_err(|e| PluginError::ExecutionError(e.to_string()))?;

        // Convert result to diagnostics
        self.convert_result_to_diagnostics(result)
    }

    /// Convert script result to Vec<Diagnostic>
    fn convert_result_to_diagnostics(&self, result: Dynamic) -> Result<Vec<Diagnostic>> {
        // Result should be an array
        let array = result
            .try_cast::<Array>()
            .ok_or(PluginError::InvalidReturnType)?;

        let mut diagnostics = Vec::new();

        for item in array {
            let diag = self.convert_item_to_diagnostic(item)?;
            diagnostics.push(diag);
        }

        Ok(diagnostics)
    }

    /// Convert a single Dynamic item to Diagnostic
    fn convert_item_to_diagnostic(&self, item: Dynamic) -> Result<Diagnostic> {
        // Try to cast to Map
        let map = item
            .try_cast::<Map>()
            .ok_or_else(|| PluginError::InvalidDiagnostic("Expected a map/object".to_string()))?;

        // Extract message (required)
        let message = map
            .get("message")
            .and_then(|v| v.clone().try_cast::<String>())
            .ok_or_else(|| PluginError::InvalidDiagnostic("Missing 'message' field".to_string()))?;

        // Extract severity (default: warning)
        let severity_str = map
            .get("severity")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_else(|| "warning".to_string());

        let severity = match severity_str.to_lowercase().as_str() {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            "hint" => Severity::Hint,
            _ => Severity::Warning,
        };

        // Build diagnostic
        let mut diag = Diagnostic::new(severity, message);

        // Extract optional code
        if let Some(code) = map.get("code").and_then(|v| v.clone().try_cast::<String>()) {
            diag = diag.with_code(code);
        }

        // Extract optional help
        if let Some(help) = map.get("help").and_then(|v| v.clone().try_cast::<String>()) {
            diag = diag.with_help(help);
        }

        // Extract optional suggestion (also as help)
        if let Some(suggestion) = map
            .get("suggestion")
            .and_then(|v| v.clone().try_cast::<String>())
        {
            if diag.help.is_none() {
                diag = diag.with_help(suggestion);
            } else {
                diag = diag.with_note(format!("Suggestion: {}", suggestion));
            }
        }

        // Extract optional file
        if let Some(file) = map.get("file").and_then(|v| v.clone().try_cast::<String>()) {
            diag = diag.with_file(file);
        }

        // Extract optional context
        if let Some(context) = map
            .get("context")
            .and_then(|v| v.clone().try_cast::<String>())
        {
            diag = diag.with_context(context);
        }

        Ok(diag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use utf8dok_ast::{Block, Document, DocumentMeta, Heading, Inline, Paragraph};

    fn sample_document() -> Document {
        Document {
            metadata: DocumentMeta {
                title: Some("Test Document".to_string()),
                ..Default::default()
            },
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    text: vec![Inline::Text("Introduction".to_string())],
                    style_id: None,
                    anchor: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("This is a test paragraph.".to_string())],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text(
                        "The report was written by the team.".to_string(),
                    )],
                    style_id: None,
                    attributes: HashMap::new(),
                }),
            ],
            intent: None,
        }
    }

    #[test]
    fn test_engine_new() {
        let engine = PluginEngine::new();
        assert!(engine.compile("let x = 1; x").is_ok());
    }

    #[test]
    fn test_compile_valid_script() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            diagnostics
        "#;
        assert!(engine.compile(script).is_ok());
    }

    #[test]
    fn test_compile_invalid_script() {
        let engine = PluginEngine::new();
        let script = "let x = {{{ invalid";
        assert!(engine.compile(script).is_err());
    }

    #[test]
    fn test_run_empty_diagnostics() {
        let engine = PluginEngine::new();
        let script = "let diagnostics = []; diagnostics";
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_run_static_diagnostic() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            diagnostics.push(#{
                code: "TEST001",
                severity: "warning",
                message: "Test diagnostic"
            });
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, Some("TEST001".to_string()));
        assert_eq!(result[0].message, "Test diagnostic");
        assert_eq!(result[0].severity, Severity::Warning);
    }

    #[test]
    fn test_access_document_title() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            if doc.metadata.title == "Test Document" {
                diagnostics.push(#{
                    code: "TITLE001",
                    message: "Found expected title"
                });
            }
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, Some("TITLE001".to_string()));
    }

    #[test]
    fn test_iterate_blocks() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            let paragraph_count = 0;

            for block in doc.blocks {
                if block.Paragraph != () {
                    paragraph_count += 1;
                }
            }

            diagnostics.push(#{
                code: "COUNT001",
                message: `Found ${paragraph_count} paragraphs`
            });
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].message.contains("2 paragraphs"));
    }

    #[test]
    fn test_helper_warning() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            diagnostics.push(warning("WARN001", "This is a warning"));
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, Severity::Warning);
    }

    #[test]
    fn test_helper_error() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            diagnostics.push(error("ERR001", "This is an error"));
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, Severity::Error);
    }

    #[test]
    fn test_passive_voice_detection() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];

            for block in doc.blocks {
                if block.Paragraph != () {
                    // Extract text from inlines (simplified - just get first Text inline)
                    for inline in block.Paragraph.inlines {
                        if inline.Text != () {
                            let text = inline.Text;
                            if has_passive_pattern(text) {
                                diagnostics.push(#{
                                    code: "STYLE001",
                                    severity: "warning",
                                    message: "Passive voice detected",
                                    context: text
                                });
                            }
                        }
                    }
                }
            }

            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, Some("STYLE001".to_string()));
        assert!(result[0].context.is_some());
    }

    #[test]
    fn test_contains_ci_helper() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];

            for block in doc.blocks {
                if block.Paragraph != () {
                    for inline in block.Paragraph.inlines {
                        if inline.Text != () {
                            let text = inline.Text;
                            if contains_ci(text, "TEAM") {
                                diagnostics.push(#{
                                    code: "FOUND001",
                                    message: "Found 'team' reference"
                                });
                            }
                        }
                    }
                }
            }

            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_diagnostic_with_all_fields() {
        let engine = PluginEngine::new();
        let script = r#"
            let diagnostics = [];
            diagnostics.push(#{
                code: "FULL001",
                severity: "error",
                message: "Full diagnostic test",
                help: "This is help text",
                suggestion: "Try this instead",
                file: "test.adoc",
                context: "The problematic text"
            });
            diagnostics
        "#;
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, Severity::Error);
        assert_eq!(result[0].help, Some("This is help text".to_string()));
        assert_eq!(result[0].file, Some("test.adoc".to_string()));
        assert_eq!(result[0].context, Some("The problematic text".to_string()));
        assert!(!result[0].notes.is_empty()); // suggestion added as note
    }

    #[test]
    fn test_invalid_return_type() {
        let engine = PluginEngine::new();
        let script = r#"42"#; // Returns integer, not array
        let ast = engine.compile(script).unwrap();
        let doc = sample_document();

        let result = engine.run_validation(&doc, &ast);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::InvalidReturnType
        ));
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
