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
//! - `ComplianceEngine`: Main engine that runs all registered rules
//! - `bridge/`: Bridge Framework specific rules (ADRs, RFCs)
//! - `dashboard/`: Report generation (HTML, Markdown, JSON)

pub mod bridge;
pub mod dashboard;

#[cfg(test)]
mod tests;

use tower_lsp::lsp_types::{DiagnosticSeverity, Range, Url};

use crate::config::Settings;
use crate::workspace::graph::WorkspaceGraph;

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

/// The main compliance engine that orchestrates all rule checks
#[allow(dead_code)]
pub struct ComplianceEngine {
    /// All registered compliance rules
    rules: Vec<Box<dyn ComplianceRule>>,
}

#[allow(dead_code)]
impl ComplianceEngine {
    /// Create a new compliance engine with default rules
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(bridge::StatusRule::new()),
                Box::new(bridge::OrphanRule::new()),
            ],
        }
    }

    /// Create a compliance engine configured from settings
    pub fn with_settings(settings: &Settings) -> Self {
        Self {
            rules: vec![
                Box::new(bridge::StatusRule::with_settings(settings)),
                Box::new(bridge::OrphanRule::with_settings(settings)),
            ],
        }
    }

    /// Create an empty engine (for custom rule sets)
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a custom rule to the engine
    pub fn add_rule(&mut self, rule: Box<dyn ComplianceRule>) {
        self.rules.push(rule);
    }

    /// Run all compliance checks against the workspace graph
    pub fn run(&self, graph: &WorkspaceGraph) -> Vec<Violation> {
        let mut all_violations = Vec::new();

        for rule in &self.rules {
            let violations = rule.check(graph);
            all_violations.extend(violations);
        }

        all_violations
    }

    /// Run checks and return statistics
    pub fn run_with_stats(&self, graph: &WorkspaceGraph) -> ComplianceResult {
        let violations = self.run(graph);

        let mut errors = 0;
        let mut warnings = 0;
        let mut info = 0;

        for v in &violations {
            match v.severity {
                ViolationSeverity::Error => errors += 1,
                ViolationSeverity::Warning => warnings += 1,
                ViolationSeverity::Info => info += 1,
            }
        }

        let total_documents = graph.document_count();
        let total_checks = total_documents * self.rules.len();
        let violations_weight = errors * 10 + warnings * 3 + info;

        let compliance_score = if total_checks == 0 {
            100
        } else {
            ((total_checks as f32 - violations_weight as f32) / total_checks as f32 * 100.0)
                .clamp(0.0, 100.0)
                .round() as u32
        };

        ComplianceResult {
            violations,
            errors,
            warnings,
            info,
            total_documents,
            compliance_score,
        }
    }

    /// Get all registered rule descriptions
    pub fn rule_descriptions(&self) -> Vec<(&'static str, &'static str)> {
        self.rules
            .iter()
            .map(|r| (r.code(), r.description()))
            .collect()
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of running compliance checks
#[derive(Debug)]
#[allow(dead_code)]
pub struct ComplianceResult {
    /// All violations found
    pub violations: Vec<Violation>,
    /// Number of errors
    pub errors: usize,
    /// Number of warnings
    pub warnings: usize,
    /// Number of info-level issues
    pub info: usize,
    /// Total documents checked
    pub total_documents: usize,
    /// Overall compliance score (0-100)
    pub compliance_score: u32,
}

impl ComplianceResult {
    /// Check if there are any critical (error) violations
    #[allow(dead_code)]
    pub fn has_critical(&self) -> bool {
        self.errors > 0
    }

    /// Check if all checks passed
    #[allow(dead_code)]
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}
