//! # utf8dok-ooxml
//!
//! OOXML (Office Open XML) parsing and generation for utf8dok.
//!
//! This crate provides functionality to:
//! - Read and parse DOCX/DOTX files
//! - Extract document content and styles
//! - Generate DOCX files from templates
//!
//! ## Example: Reading a Document
//!
//! ```no_run
//! use utf8dok_ooxml::{OoxmlArchive, Document, StyleSheet};
//!
//! let archive = OoxmlArchive::open("document.docx")?;
//! let document = Document::parse(archive.document_xml()?)?;
//! let styles = StyleSheet::parse(archive.styles_xml()?)?;
//!
//! for block in &document.blocks {
//!     println!("{:?}", block);
//! }
//! # Ok::<(), utf8dok_ooxml::OoxmlError>(())
//! ```

pub mod archive;
pub mod conversion;
pub mod document;
pub mod error;
pub mod extract;
pub mod manifest;
pub mod relationships;
pub mod styles;
pub mod template;
pub mod writer;

pub use archive::OoxmlArchive;
pub use conversion::{convert_document, convert_document_with_styles, ConversionContext, ToAst};
pub use document::{
    Block, Document, Hyperlink, Paragraph, ParagraphChild, Run, Table, TableCell, TableRow,
};
pub use error::{OoxmlError, Result};
pub use extract::{AsciiDocExtractor, ExtractedDocument, SourceOrigin};
pub use manifest::{ElementMeta, Manifest, MANIFEST_PATH};
pub use relationships::Relationships;
pub use styles::{ElementType, Style, StyleMap, StyleSheet, StyleType};
pub use template::Template;
pub use writer::DocxWriter;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
