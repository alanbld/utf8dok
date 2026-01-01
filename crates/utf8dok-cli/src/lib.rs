//! utf8dok CLI - Command-line interface library
//!
//! This library provides the CLI functionality for utf8dok, including:
//! - Extract: Convert DOCX to AsciiDoc
//! - Render: Convert AsciiDoc to DOCX
//! - Check: Validate AsciiDoc files
//!
//! # Library Usage
//!
//! ```ignore
//! use utf8dok_cli::{run_cli, OutputFormat};
//!
//! // Run the full CLI
//! run_cli();
//!
//! // Or use individual commands programmatically
//! extract_command(&input, &output, false)?;
//! check_command(&input, OutputFormat::Json, &plugins)?;
//! ```
//!
//! # Binary Usage
//!
//! ```bash
//! # Extract AsciiDoc from DOCX
//! utf8dok extract document.docx --output result/
//!
//! # Render AsciiDoc to DOCX
//! utf8dok render document.adoc --output final.docx
//!
//! # Check AsciiDoc for issues
//! utf8dok check document.adoc --format json
//! ```

pub mod app;

// Re-export main entry point and types
pub use app::{
    audit_command, check_command, dashboard_command, dual_nature_command, extract_command,
    list_includes_command, render_command,
};
pub use app::{run_cli, AuditFormat, DualNatureTargetFormat, OutputFormat, RenderFormat};
