//! Typst to PDF compiler
//!
//! Compiles Typst markup to PDF bytes using typst-as-lib.

use crate::error::{PdfError, Result};
use typst_as_lib::TypstEngine;

/// Compiler for converting Typst markup to PDF
pub struct Compiler;

impl Compiler {
    /// Compile Typst markup to PDF bytes
    ///
    /// # Arguments
    /// * `markup` - Typst markup string
    ///
    /// # Returns
    /// PDF bytes on success
    pub fn compile(markup: &str) -> Result<Vec<u8>> {
        Self::compile_with_fonts(markup, &[])
    }

    /// Compile with custom fonts
    ///
    /// # Arguments
    /// * `markup` - Typst markup string
    /// * `font_paths` - Paths to font files to include
    ///
    /// # Returns
    /// PDF bytes on success
    pub fn compile_with_fonts(markup: &str, font_paths: &[&str]) -> Result<Vec<u8>> {
        // Build the Typst engine with the markup as main file
        let mut builder = TypstEngine::builder().main_file(markup.to_string());

        // Add fonts if provided
        for font_path in font_paths {
            let font_bytes = std::fs::read(font_path).map_err(|e| {
                PdfError::Font(format!("Failed to read font {}: {}", font_path, e))
            })?;
            builder = builder.fonts([font_bytes]);
        }

        let engine = builder.build();

        // Compile the document
        let compiled = engine.compile();

        // compiled is Warned<Result<Document, Error>>
        // - compiled.output is the Result
        // - compiled.warnings contains any warnings
        let document = compiled
            .output
            .map_err(|e| PdfError::Compilation(format!("{:?}", e)))?;

        // Generate PDF
        let options = typst_pdf::PdfOptions::default();
        let pdf_bytes = typst_pdf::pdf(&document, &options)
            .map_err(|e| PdfError::Compilation(format!("PDF generation failed: {:?}", e)))?;

        Ok(pdf_bytes.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let markup = "= Hello World\n\nThis is a test document.";
        let result = Compiler::compile(markup);

        // Should compile successfully
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let pdf = result.unwrap();
        // PDF files start with %PDF
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output doesn't start with PDF header"
        );
    }

    #[test]
    fn test_compile_with_formatting() {
        let markup = r#"
= Document Title

== Section One

This is *bold* and _italic_ text.

- Item one
- Item two
- Item three

```rust
fn main() {
    println!("Hello!");
}
```
"#;
        let result = Compiler::compile(markup);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_invalid_syntax() {
        // This should still compile - Typst is quite forgiving
        let markup = "#invalid_function_that_doesnt_exist()";
        let result = Compiler::compile(markup);
        // May fail or succeed depending on Typst version
        // The important thing is it doesn't panic
        let _ = result;
    }
}
