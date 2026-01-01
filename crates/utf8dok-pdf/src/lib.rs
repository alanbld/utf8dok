//! utf8dok-pdf - PDF generation via Typst
//!
//! This crate provides PDF generation for utf8dok documents using Typst
//! as the typesetting backend.
//!
//! # Architecture
//!
//! The PDF generation pipeline consists of two stages:
//!
//! 1. **Transpiler** - Converts `utf8dok_ast::Document` to Typst markup
//! 2. **Compiler** - Compiles Typst markup to PDF bytes
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ast::Document;
//! use utf8dok_pdf::{Transpiler, Compiler};
//!
//! let doc = Document::new();
//! let typst_markup = Transpiler::transpile(&doc);
//! let pdf_bytes = Compiler::compile(&typst_markup)?;
//! ```

mod compiler;
mod error;
mod transpiler;

pub use compiler::Compiler;
pub use error::{PdfError, Result};
pub use transpiler::Transpiler;

/// Convenience function to render a document to PDF
///
/// # Arguments
/// * `doc` - The AST document to render
///
/// # Returns
/// PDF bytes on success
pub fn render_pdf(doc: &utf8dok_ast::Document) -> Result<Vec<u8>> {
    let typst_markup = Transpiler::transpile(doc);
    Compiler::compile(&typst_markup)
}

/// Render with a custom template
///
/// # Arguments
/// * `doc` - The AST document to render
/// * `template_path` - Path to a .typ template file
///
/// # Returns
/// PDF bytes on success
pub fn render_pdf_with_template(
    doc: &utf8dok_ast::Document,
    template_path: &str,
) -> Result<Vec<u8>> {
    let typst_markup = Transpiler::transpile_with_template(doc, template_path);
    Compiler::compile(&typst_markup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify exports are accessible
        let _ = Transpiler::transpile;
        let _ = Compiler::compile;
        let _ = render_pdf;
    }
}
