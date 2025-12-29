//! utf8dok Language Server Protocol implementation
//!
//! This library provides LSP support for AsciiDoc files, including:
//! - Validation diagnostics from utf8dok's native validators
//! - Folding ranges for headers and blocks
//! - Document symbols for navigation
//! - Selection ranges for semantic selection
//! - Rename refactoring for anchors and references
//! - Completion for xrefs, attributes, and blocks
//! - Code actions for common fixes
//! - Semantic tokens for syntax highlighting
//! - Workspace symbols for project-wide search
//! - Compliance engine for documentation frameworks
//!
//! # Library Usage
//!
//! ```ignore
//! use utf8dok_lsp::{run_server, compliance::ComplianceEngine};
//!
//! // Run the LSP server
//! run_server().await;
//!
//! // Or use the compliance engine directly
//! let engine = ComplianceEngine::new();
//! ```
//!
//! # Binary Usage
//!
//! ```bash
//! # Start the language server (typically called by an editor)
//! utf8dok-lsp
//!
//! # With debug logging
//! RUST_LOG=debug utf8dok-lsp
//! ```

pub mod compliance;
pub mod domain;
pub mod intelligence;
pub mod server;
pub mod structural;
pub mod workspace;

// Re-export main entry point
pub use server::run_server;

// Re-export commonly used types
pub use compliance::{ComplianceEngine, ComplianceResult, Violation};
pub use workspace::graph::WorkspaceGraph;
