//! Compiler diagnostics for utf8dok
//!
//! This module provides structures for reporting errors, warnings, and
//! informational messages during document compilation.

use serde::{Deserialize, Serialize};

/// A diagnostic message from the compiler
///
/// Diagnostics represent issues found during parsing, validation,
/// or compilation. They can range from fatal errors to informational hints.
///
/// # Example
///
/// ```
/// use utf8dok_core::diagnostics::{Diagnostic, Severity, Span};
///
/// let diag = Diagnostic::new(
///     Severity::Error,
///     "Unresolved cross-reference",
/// )
/// .with_code("E0001")
/// .with_span(Span::new(10, 25))
/// .with_help("Check that the target anchor exists");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level of the diagnostic
    pub severity: Severity,

    /// The diagnostic message
    pub message: String,

    /// Optional error/warning code (e.g., "E0001", "W0042")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Source location where the issue occurred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,

    /// Optional file path where the issue occurred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Additional help text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Related notes or secondary locations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Severity level of a diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational hint, does not indicate a problem
    Hint,

    /// Informational message
    Info,

    /// Warning, indicates a potential issue
    Warning,

    /// Error, indicates a problem that should be fixed
    Error,

    /// Fatal error, compilation cannot continue
    Fatal,
}

/// A source location span
///
/// Represents a range in the source document, typically as byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// Start offset (inclusive)
    pub start: usize,

    /// End offset (exclusive)
    pub end: usize,

    /// Optional line number (1-indexed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,

    /// Optional column number (1-indexed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            code: None,
            span: None,
            file: None,
            help: None,
            notes: Vec::new(),
        }
    }

    /// Create an error diagnostic
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// Create a warning diagnostic
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    /// Create an info diagnostic
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(Severity::Info, message)
    }

    /// Create a hint diagnostic
    pub fn hint(message: impl Into<String>) -> Self {
        Self::new(Severity::Hint, message)
    }

    /// Set the diagnostic code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the source span
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Set the file path
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Check if this is an error-level diagnostic
    pub fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error | Severity::Fatal)
    }

    /// Check if this is a warning-level diagnostic
    pub fn is_warning(&self) -> bool {
        matches!(self.severity, Severity::Warning)
    }
}

impl Span {
    /// Create a new span from start and end offsets
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            line: None,
            column: None,
        }
    }

    /// Create a span at a single point
    pub fn point(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// Set the line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set both line and column
    pub fn with_position(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Check if this span contains an offset
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Merge two spans into one that covers both
    pub fn merge(&self, other: &Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Hint => write!(f, "hint"),
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Fatal => write!(f, "fatal"),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format: severity[code]: message
        write!(f, "{}", self.severity)?;
        if let Some(ref code) = self.code {
            write!(f, "[{}]", code)?;
        }
        write!(f, ": {}", self.message)?;

        // Add location if available
        if let Some(ref file) = self.file {
            write!(f, "\n  --> {}", file)?;
            if let Some(ref span) = self.span {
                if let (Some(line), Some(col)) = (span.line, span.column) {
                    write!(f, ":{}:{}", line, col)?;
                }
            }
        }

        // Add help text
        if let Some(ref help) = self.help {
            write!(f, "\n  = help: {}", help)?;
        }

        // Add notes
        for note in &self.notes {
            write!(f, "\n  = note: {}", note)?;
        }

        Ok(())
    }
}

/// A collection of diagnostics
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    /// List of diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Create a new empty diagnostics collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add an error
    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Diagnostic::error(message));
    }

    /// Add a warning
    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Diagnostic::warning(message));
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    /// Get the number of errors
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Get the number of warnings
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_warning()).count()
    }

    /// Get all diagnostics
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Get the count
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_new() {
        let diag = Diagnostic::new(Severity::Error, "Test error");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "Test error");
        assert!(diag.code.is_none());
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::error("Missing semicolon")
            .with_code("E0001")
            .with_span(Span::new(10, 11).with_position(5, 20))
            .with_file("main.adoc")
            .with_help("Add a semicolon here");

        assert!(diag.is_error());
        assert_eq!(diag.code, Some("E0001".to_string()));
        assert_eq!(diag.file, Some("main.adoc".to_string()));
        assert!(diag.span.is_some());
    }

    #[test]
    fn test_span_operations() {
        let span1 = Span::new(10, 20);
        let span2 = Span::new(15, 30);

        assert_eq!(span1.len(), 10);
        assert!(span1.contains(15));
        assert!(!span1.contains(25));

        let merged = span1.merge(&span2);
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn test_severity_order() {
        assert!(Severity::Hint < Severity::Info);
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn test_diagnostics_collection() {
        let mut diags = Diagnostics::new();
        diags.error("Error 1");
        diags.warning("Warning 1");
        diags.error("Error 2");

        assert!(diags.has_errors());
        assert_eq!(diags.error_count(), 2);
        assert_eq!(diags.warning_count(), 1);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error("Invalid syntax")
            .with_code("E0001")
            .with_file("test.adoc")
            .with_span(Span::new(10, 15).with_position(3, 5))
            .with_help("Check the syntax");

        let display = format!("{}", diag);
        assert!(display.contains("error[E0001]"));
        assert!(display.contains("Invalid syntax"));
        assert!(display.contains("test.adoc:3:5"));
        assert!(display.contains("help: Check the syntax"));
    }

    #[test]
    fn test_diagnostic_serialize() {
        let diag = Diagnostic::warning("Unused variable")
            .with_code("W0042");

        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("\"severity\":\"warning\""));
        assert!(json.contains("\"code\":\"W0042\""));

        let restored: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.severity, Severity::Warning);
        assert_eq!(restored.code, Some("W0042".to_string()));
    }
}
