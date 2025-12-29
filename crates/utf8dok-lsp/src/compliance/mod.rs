//! Compliance Engine Module
//!
//! Provides cross-file validation rules for document frameworks like Bridge (ADRs).
//! Designed to work in both LSP (real-time) and CLI (CI/CD) contexts.
//!
//! # Architecture
//!
//! The compliance engine is structured for future extraction to a shared crate:
//! - `Violation`: A compliance issue found during validation
//! - `ComplianceRule`: Trait for implementing validation rules
//! - `bridge/`: Bridge Framework specific rules (ADRs, RFCs)

pub mod bridge;

#[cfg(test)]
mod tests;

use tower_lsp::lsp_types::{DiagnosticSeverity, Range, Url};

/// A compliance violation found during validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Violation {
    /// The document URI where the violation was found
    pub uri: Url,
    /// The range in the document (for highlighting)
    pub range: Range,
    /// Human-readable message describing the violation
    pub message: String,
    /// Severity of the violation
    pub severity: ViolationSeverity,
    /// Rule code (e.g., "BRIDGE001")
    pub code: String,
}

/// Severity levels for compliance violations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ViolationSeverity {
    /// Critical issues that must be fixed
    Error,
    /// Issues that should be addressed
    Warning,
    /// Informational suggestions
    Info,
}

impl ViolationSeverity {
    /// Convert to LSP DiagnosticSeverity
    #[allow(dead_code)]
    pub fn to_lsp_severity(self) -> DiagnosticSeverity {
        match self {
            ViolationSeverity::Error => DiagnosticSeverity::ERROR,
            ViolationSeverity::Warning => DiagnosticSeverity::WARNING,
            ViolationSeverity::Info => DiagnosticSeverity::INFORMATION,
        }
    }
}

/// Trait for implementing compliance rules
#[allow(dead_code)]
pub trait ComplianceRule: Send + Sync {
    /// Check the workspace graph for violations
    fn check(&self, graph: &crate::workspace::graph::WorkspaceGraph) -> Vec<Violation>;

    /// Get the rule's unique identifier
    fn code(&self) -> &'static str;

    /// Get a human-readable description of the rule
    fn description(&self) -> &'static str;
}
