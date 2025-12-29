//! JSON Report Generator
//!
//! Generates JSON-formatted compliance reports for CI/CD integration.
//! Designed for machine parsing and pipeline integration.

use super::timestamp;
use crate::compliance::{ComplianceResult, ViolationSeverity};

/// JSON report generator
pub struct JsonGenerator;

impl JsonGenerator {
    /// Generate a complete JSON report
    #[allow(dead_code)]
    pub fn generate(result: &ComplianceResult) -> String {
        let violations: Vec<String> = result
            .violations
            .iter()
            .map(|v| {
                let severity = match v.severity {
                    ViolationSeverity::Error => "error",
                    ViolationSeverity::Warning => "warning",
                    ViolationSeverity::Info => "info",
                };

                format!(
                    r#"    {{
      "code": "{}",
      "severity": "{}",
      "message": {},
      "location": {{
        "uri": "{}",
        "range": {{
          "start": {{ "line": {}, "character": {} }},
          "end": {{ "line": {}, "character": {} }}
        }}
      }}
    }}"#,
                    v.code,
                    severity,
                    json_escape(&v.message),
                    v.uri,
                    v.range.start.line,
                    v.range.start.character,
                    v.range.end.line,
                    v.range.end.character
                )
            })
            .collect();

        let status = if result.errors > 0 {
            "failing"
        } else if result.warnings > 0 {
            "passing_with_warnings"
        } else {
            "passing"
        };

        format!(
            r#"{{
  "timestamp": "{}",
  "status": "{}",
  "summary": {{
    "total_documents": {},
    "errors": {},
    "warnings": {},
    "info": {},
    "compliance_score": {}
  }},
  "violations": [
{}
  ]
}}"#,
            timestamp(),
            status,
            result.total_documents,
            result.errors,
            result.warnings,
            result.info,
            result.compliance_score,
            violations.join(",\n")
        )
    }

    /// Generate a minimal JSON for CI exit code decisions
    #[allow(dead_code)]
    pub fn generate_minimal(result: &ComplianceResult) -> String {
        let status = if result.errors > 0 {
            "failing"
        } else {
            "passing"
        };

        format!(
            r#"{{"status":"{}","errors":{},"warnings":{},"score":{}}}"#,
            status, result.errors, result.warnings, result.compliance_score
        )
    }
}

/// JSON string escaping
fn json_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 2);
    escaped.push('"');

    for c in s.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => escaped.push(c),
        }
    }

    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compliance::Violation;
    use tower_lsp::lsp_types::{Position, Range, Url};

    #[test]
    fn test_json_escape() {
        assert_eq!(json_escape("hello"), r#""hello""#);
        assert_eq!(json_escape(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(json_escape("line\nbreak"), r#""line\nbreak""#);
        assert_eq!(json_escape("path\\to\\file"), r#""path\\to\\file""#);
    }

    #[test]
    fn test_generate_clean_report() {
        let result = ComplianceResult {
            violations: vec![],
            errors: 0,
            warnings: 0,
            info: 0,
            total_documents: 5,
            compliance_score: 100,
        };

        let json = JsonGenerator::generate(&result);
        assert!(json.contains(r#""status": "passing""#));
        assert!(json.contains(r#""errors": 0"#));
        assert!(json.contains(r#""compliance_score": 100"#));
        assert!(json.contains(r#""violations": ["#));
    }

    #[test]
    fn test_generate_with_violations() {
        let result = ComplianceResult {
            violations: vec![Violation {
                uri: Url::parse("file:///test.adoc").unwrap(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 10,
                    },
                    end: Position {
                        line: 5,
                        character: 20,
                    },
                },
                message: "Test \"violation\" with special chars".to_string(),
                severity: ViolationSeverity::Error,
                code: "TEST001".to_string(),
            }],
            errors: 1,
            warnings: 0,
            info: 0,
            total_documents: 5,
            compliance_score: 80,
        };

        let json = JsonGenerator::generate(&result);
        assert!(json.contains(r#""status": "failing""#));
        assert!(json.contains(r#""code": "TEST001""#));
        assert!(json.contains(r#""severity": "error""#));
        assert!(json.contains(r#""line": 5"#));
        // Check that quotes are escaped
        assert!(json.contains(r#"\"violation\""#));
    }

    #[test]
    fn test_generate_minimal() {
        let result = ComplianceResult {
            violations: vec![],
            errors: 0,
            warnings: 2,
            info: 1,
            total_documents: 10,
            compliance_score: 95,
        };

        let json = JsonGenerator::generate_minimal(&result);
        assert_eq!(
            json,
            r#"{"status":"passing","errors":0,"warnings":2,"score":95}"#
        );
    }

    #[test]
    fn test_valid_json_structure() {
        let result = ComplianceResult {
            violations: vec![Violation {
                uri: Url::parse("file:///test.adoc").unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                message: "Test".to_string(),
                severity: ViolationSeverity::Warning,
                code: "TEST".to_string(),
            }],
            errors: 0,
            warnings: 1,
            info: 0,
            total_documents: 1,
            compliance_score: 90,
        };

        let json = JsonGenerator::generate(&result);

        // Verify it's valid JSON by checking basic structure
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("\"timestamp\":"));
        assert!(json.contains("\"summary\":"));
        assert!(json.contains("\"violations\":"));
    }
}
